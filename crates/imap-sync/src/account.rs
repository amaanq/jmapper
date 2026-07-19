//! Per-account IMAP task: sync, IDLE, serialized mutations, and reconnects.

use std::{
   sync::atomic::{
      AtomicU64,
      Ordering,
   },
   time::Duration,
};

use async_imap::extensions::idle::IdleResponse;
use chrono::Utc;
use deadpool_postgres::Pool as PgPool;
use jmapper_codegen::queries;
use serde::Deserialize;
use tokio::{
   sync::{
      mpsc,
      oneshot,
   },
   task::JoinHandle,
   time::{
      self,
      Instant,
   },
};
use tracing::{
   debug,
   error,
   info,
   warn,
};

use crate::{
   db,
   error::{
      Result,
      SyncError,
   },
   imap::{
      self,
      ImapSession,
   },
   oauth::GmailOAuth,
   provider::{
      ImapTls,
      ProviderKind,
   },
   smtp,
   sync,
};

/// Account-task failures that triggered a reconnect.
pub static SYNC_FAILURES_TOTAL: AtomicU64 = AtomicU64::new(0);
const REQUEST_QUEUE_CAPACITY: usize = 128;
const BODY_CACHE_WINDOW: usize = 2_048;
const BODY_CACHE_BATCH: usize = 32;
const INCREMENTAL_BODY_PREFETCH: usize = 16;

#[derive(Debug, Clone)]
pub struct AccountRuntime {
   pub id:            String,
   pub email:         String,
   pub provider:      ProviderKind,
   /// Depth of the first ingest of each folder, in days (`UID SEARCH SINCE`).
   /// 0 falls back to the fixed most-recent-UIDs window.
   pub backfill_days: u32,
   pub gmail:         Option<GmailAuth>,
   pub imap:          Option<ImapRuntime>,
   pub smtp:          Option<SmtpRuntime>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
#[non_exhaustive]
pub enum GmailAuth {
   AppPassword {
      app_password: String,
   },
   OAuth {
      client_id:     String,
      client_secret: String,
   },
}

impl GmailAuth {
   #[must_use]
   #[inline]
   pub fn app_password(&self) -> Option<String> {
      match *self {
         Self::AppPassword { ref app_password } => {
            Some(
               app_password
                  .chars()
                  .filter(|character| !character.is_whitespace())
                  .collect(),
            )
         },
         Self::OAuth { .. } => None,
      }
   }

   #[must_use]
   #[inline]
   pub fn oauth(&self) -> Option<(&str, &str)> {
      match *self {
         Self::AppPassword { .. } => None,
         Self::OAuth {
            ref client_id,
            ref client_secret,
         } => Some((client_id, client_secret)),
      }
   }
}

#[derive(Debug, Clone)]
pub struct ImapRuntime {
   pub host:     String,
   pub port:     u16,
   pub tls:      ImapTls,
   pub username: String,
   pub password: String,
}

#[derive(Debug, Clone)]
pub struct SmtpRuntime {
   pub host:     String,
   pub port:     u16,
   pub tls:      ImapTls,
   /// Fall back to the IMAP username/password when unset.
   pub username: Option<String>,
   pub password: Option<String>,
}

#[derive(Debug)]
#[non_exhaustive]
pub enum AccountRequest {
   Ping,
   FetchBody {
      msgid:   String,
      respond: oneshot::Sender<Result<()>>,
   },
   FetchBodies {
      msgids:  Vec<String>,
      respond: oneshot::Sender<Result<()>>,
   },
   /// Coalesced by `(folder, flag-delta)` before issuing `UID STORE`.
   StoreFlags {
      msgid:   String,
      add:     Vec<String>,
      remove:  Vec<String>,
      respond: oneshot::Sender<Result<()>>,
   },
   MutateMailboxes {
      msgid:   String,
      add:     Vec<String>,
      remove:  Vec<String>,
      respond: oneshot::Sender<Result<()>>,
   },
   DestroyMessage {
      msgid:   String,
      respond: oneshot::Sender<Result<()>>,
   },
   ImportMessage {
      blob_id:          String,
      mailbox_id:       String,
      flags:            Vec<String>,
      received_at_secs: Option<i64>,
      respond:          oneshot::Sender<Result<ImportOutcome>>,
   },
   CreateFolder {
      name:              String,
      parent_mailbox_id: Option<String>,
      respond:           oneshot::Sender<Result<String>>,
   },
   RenameFolder {
      mailbox_id: String,
      new_name:   String,
      respond:    oneshot::Sender<Result<()>>,
   },
   DeleteFolder {
      mailbox_id: String,
      respond:    oneshot::Sender<Result<()>>,
   },
   SubmitEmail {
      msgid:     String,
      mail_from: String,
      rcpt_to:   Vec<String>,
      respond:   oneshot::Sender<Result<String>>,
   },
   /// Uses staged bytes because the source draft may already be destroyed.
   SubmitStaged {
      mail_from: String,
      rcpt_to:   Vec<String>,
      raw:       Vec<u8>,
      respond:   oneshot::Sender<Result<String>>,
   },
   Shutdown {
      respond: oneshot::Sender<()>,
   },
}

#[derive(Debug, Clone)]
pub struct ImportOutcome {
   pub msgid:     String,
   pub thread_id: String,
}

pub struct AccountHandle {
   pub tx:   mpsc::Sender<AccountRequest>,
   pub task: JoinHandle<()>,
}

struct BodyCacheWarmer(JoinHandle<()>);

impl BodyCacheWarmer {
   fn spawn(runtime: AccountRuntime, pool: PgPool) -> Self {
      Self(tokio::spawn(warm_body_cache(runtime, pool)))
   }
}

impl Drop for BodyCacheWarmer {
   fn drop(&mut self) {
      self.0.abort();
   }
}

#[must_use]
#[inline]
pub fn spawn(runtime: AccountRuntime, pool: PgPool) -> AccountHandle {
   let (tx, rx) = mpsc::channel(REQUEST_QUEUE_CAPACITY);
   let task = tokio::spawn(run_account(runtime, pool, rx));
   AccountHandle { tx, task }
}

async fn run_account(
   runtime: AccountRuntime,
   pool: PgPool,
   mut rx: mpsc::Receiver<AccountRequest>,
) {
   let mut backoff = Duration::from_secs(2);
   let max_backoff = Duration::from_secs(300);

   loop {
      match run_once(&runtime, &pool, &mut rx).await {
         Ok(()) => {
            info!(account_id = %runtime.id, "account task exited cleanly");
            return;
         },
         Err(err) => {
            SYNC_FAILURES_TOTAL.fetch_add(1, Ordering::Relaxed);
            error!(
                account_id = %runtime.id,
                error = %err,
                backoff_secs = backoff.as_secs(),
                "account task failed; will reconnect",
            );
            // While sleeping, still respond to Shutdown / closed channel —
            // the reload path shouldn't have to wait on `max_backoff`.
            tokio::select! {
                () = time::sleep(backoff) => {}
                req = rx.recv() => {
                    match req {
                        None => {
                            info!(account_id = %runtime.id, "channel closed during backoff; exiting");
                            return;
                        }
                        Some(AccountRequest::Shutdown { respond }) => {
                            let _ = respond.send(());
                            info!(account_id = %runtime.id, "shutdown during backoff; exiting");
                            return;
                        }
                        Some(_) => {} // drop other reqs during backoff
                    }
                }
            }
            backoff = (backoff * 2).min(max_backoff);
         },
      }
   }
}

async fn run_once(
   runtime: &AccountRuntime,
   pool: &PgPool,
   rx: &mut mpsc::Receiver<AccountRequest>,
) -> Result<()> {
   let mut session = connect_and_auth(runtime, pool).await?;
   info!(account_id = %runtime.id, "connected + authenticated");

   // Refuse to proceed on IDLE-less servers per the plan.
   if !imap::has_idle_capability(&mut session).await? {
      return Err(SyncError::IdleUnsupported);
   }

   let stats = sync::initial_sync(&mut session, pool, &runtime.id, runtime.backfill_days).await?;
   info!(
       account_id = %runtime.id,
       folders = stats.folders_synced,
       messages = stats.messages_upserted,
       "initial sync complete",
   );
   let repaired = sync::repair_cached_body_metadata(pool, &runtime.id).await?;
   if repaired > 0 {
      info!(account_id = %runtime.id, repaired, "repaired cached body metadata");
   }
   let _body_cache_warmer = BodyCacheWarmer::spawn(runtime.clone(), pool.clone());

   let primary = discover_primary_folder(&mut session, runtime.provider).await?;
   info!(account_id = %runtime.id, folder = %primary, "entering IDLE");

   loop {
      // Select the primary folder each pass — cheap, and ensures the session
      // is in SELECTED state for IDLE even after a delta_sync touched other
      // folders in a later extension.
      let _ = session.select(&primary).await?;

      let mut idle = session.idle();
      idle.init().await?;

      // Scope the borrow so `wait_fut` and `stop_source` are dropped before
      // we call `idle.done()` (which takes `idle` by value).
      let (idle_result, pending_req) = {
         let (wait_fut, _stop_source) = idle.wait_with_timeout(Duration::from_mins(28));
         tokio::pin!(wait_fut);
         tokio::select! {
             idle_result = &mut wait_fut => (idle_result.map_err(SyncError::from), None),
             req = rx.recv() => (Ok(IdleResponse::ManualInterrupt), req),
         }
      };

      session = idle.done().await?;
      let idle_response = idle_result?;
      let reconcile = idle_wake_needs_reconcile(&idle_response);
      match idle_response {
         IdleResponse::NewData(_) => {
            debug!(account_id = %runtime.id, "idle: NewData, running delta sync");
         },
         IdleResponse::Timeout => {
            debug!(account_id = %runtime.id, "idle: 28m timeout, re-arming");
         },
         IdleResponse::ManualInterrupt => {
            debug!(account_id = %runtime.id, "idle: manual interrupt");
         },
      }

      if reconcile {
         match sync::reconcile_all_folders(&mut session, pool, &runtime.id, runtime.backfill_days)
            .await
         {
            Ok(n) if n > 0 => {
               debug!(
                   account_id = %runtime.id,
                   changes = n,
                   "reconcile applied changes"
               );
               if let Err(err) = sync::prefetch_recent_bodies(
                  &mut session,
                  pool,
                  &runtime.id,
                  BODY_CACHE_WINDOW,
                  INCREMENTAL_BODY_PREFETCH,
               )
               .await
               {
                  warn!(
                     account_id = %runtime.id,
                     error = %err,
                     "incremental body prefetch failed; continuing",
                  );
               }
            },
            Ok(_) => {},
            Err(err) => {
               warn!(
                   account_id = %runtime.id,
                   error = %err,
                   "reconcile failed; continuing",
               );
            },
         }
      }

      // Collect every request that's already queued, plus the one that
      // interrupted IDLE. If any of them is a StoreFlags, give writers a
      // 50 ms grace window to coalesce — multi-message "mark as read"
      // operations fire N oneshots in quick succession and we want to
      // emit one `UID STORE {uid,uid,...}` per folder, not N.
      let mut pending = Vec::<AccountRequest>::new();
      if let Some(req) = pending_req {
         pending.push(req);
      }
      while let Ok(req) = rx.try_recv() {
         pending.push(req);
      }
      if pending
         .iter()
         .any(|request| matches!(request, AccountRequest::StoreFlags { .. }))
      {
         let deadline = Instant::now() + Duration::from_millis(50);
         loop {
            tokio::select! {
                () = time::sleep_until(deadline) => break,
                req = rx.recv() => match req {
                    None => break,
                    Some(req) => pending.push(req),
                }
            }
         }
      }

      if process_pending(pending, &mut session, pool, runtime).await == Flow::Exit {
         return Ok(());
      }
      // If every sender has been dropped (e.g. reload without explicit
      // Shutdown), exit cleanly on the next iteration.
      if rx.is_closed() {
         info!(account_id = %runtime.id, "request channel closed; exiting");
         return Ok(());
      }
   }
}

async fn warm_body_cache(runtime: AccountRuntime, pool: PgPool) {
   let mut backoff = Duration::from_secs(2);
   let mut warmed = 0_usize;

   loop {
      let mut session = match connect_and_auth(&runtime, &pool).await {
         Ok(session) => session,
         Err(error) => {
            warn!(
               account_id = %runtime.id,
               %error,
               "body cache connection failed; retrying",
            );
            time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(300));
            continue;
         },
      };
      backoff = Duration::from_secs(2);

      loop {
         match sync::prefetch_recent_bodies(
            &mut session,
            &pool,
            &runtime.id,
            BODY_CACHE_WINDOW,
            BODY_CACHE_BATCH,
         )
         .await
         {
            Ok(0) => {
               info!(account_id = %runtime.id, warmed, "recent body cache ready");
               return;
            },
            Ok(count) => warmed += count,
            Err(error) => {
               warn!(
                  account_id = %runtime.id,
                  %error,
                  "body cache fill interrupted; reconnecting",
               );
               break;
            },
         }
         time::sleep(Duration::from_millis(250)).await;
      }

      time::sleep(backoff).await;
      backoff = (backoff * 2).min(Duration::from_secs(300));
   }
}

const fn idle_wake_needs_reconcile(response: &IdleResponse) -> bool {
   !matches!(response, IdleResponse::ManualInterrupt)
}

#[derive(Debug, PartialEq, Eq)]
enum Flow {
   Continue,
   Exit,
}

async fn process_request(
   req: AccountRequest,
   session: &mut ImapSession,
   pool: &PgPool,
   runtime: &AccountRuntime,
) -> Flow {
   let account_id = runtime.id.as_str();
   match req {
      AccountRequest::Ping => Flow::Continue,
      AccountRequest::FetchBody { msgid, respond } => {
         let result = sync::fetch_and_cache_body(session, pool, account_id, &msgid).await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::FetchBodies { msgids, respond } => {
         let result = sync::fetch_and_cache_bodies(session, pool, account_id, &msgids).await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::StoreFlags {
         msgid,
         add,
         remove,
         respond,
      } => {
         // Single-request path (the batch path unwraps to here on an
         // empty coalesce window). Reuse the batch helper with a
         // one-element vec so the STORE logic lives in exactly one place.
         let result = sync::store_flags_batch(session, pool, account_id, &[sync::StoreFlagsOp {
            msgid,
            add,
            remove,
         }])
         .await;
         let resolved = result.into_iter().next().unwrap_or_else(|| {
            Err(SyncError::Other(
               "store_flags_batch returned no result".into(),
            ))
         });
         let _ = respond.send(resolved);
         Flow::Continue
      },
      AccountRequest::MutateMailboxes {
         msgid,
         add,
         remove,
         respond,
      } => {
         let result =
            sync::mutate_mailboxes(session, pool, account_id, &msgid, &add, &remove).await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::DestroyMessage { msgid, respond } => {
         let result = sync::destroy_message(session, pool, account_id, &msgid).await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::ImportMessage {
         blob_id,
         mailbox_id,
         flags,
         received_at_secs,
         respond,
      } => {
         let result = sync::import_message(
            session,
            pool,
            account_id,
            &blob_id,
            &mailbox_id,
            &flags,
            received_at_secs,
         )
         .await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::CreateFolder {
         name,
         parent_mailbox_id,
         respond,
      } => {
         let result = sync::create_folder(
            session,
            pool,
            account_id,
            &name,
            parent_mailbox_id.as_deref(),
         )
         .await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::RenameFolder {
         mailbox_id,
         new_name,
         respond,
      } => {
         let result = sync::rename_folder(session, pool, account_id, &mailbox_id, &new_name).await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::DeleteFolder {
         mailbox_id,
         respond,
      } => {
         let result = sync::delete_folder(session, pool, account_id, &mailbox_id).await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::SubmitEmail {
         msgid,
         mail_from,
         rcpt_to,
         respond,
      } => {
         let result = submit_email(session, pool, runtime, &msgid, &mail_from, &rcpt_to).await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::SubmitStaged {
         mail_from,
         rcpt_to,
         raw,
         respond,
      } => {
         let result = async {
            let params = smtp_params_for(runtime, pool).await?;
            info!(
                account_id = %runtime.id,
                rcpt = rcpt_to.len(),
                host = %params.host,
                "submitting staged message over SMTP",
            );
            smtp::submit(&params, &mail_from, &rcpt_to, &raw).await
         }
         .await;
         let _ = respond.send(result);
         Flow::Continue
      },
      AccountRequest::Shutdown { respond } => {
         let _ = respond.send(());
         info!(account_id, "shutdown requested; exiting");
         Flow::Exit
      },
   }
}

/// Handle requests collected during one IDLE wakeup.
async fn process_pending(
   pending: Vec<AccountRequest>,
   session: &mut ImapSession,
   pool: &PgPool,
   runtime: &AccountRuntime,
) -> Flow {
   let account_id = runtime.id.as_str();
   let mut body_msgids = Vec::<String>::new();
   let mut body_responders = Vec::<oneshot::Sender<Result<()>>>::new();
   let mut store_ops = Vec::<sync::StoreFlagsOp>::new();
   let mut store_responders = Vec::<oneshot::Sender<Result<()>>>::new();
   let mut others = Vec::<AccountRequest>::new();
   for req in pending {
      match req {
         AccountRequest::FetchBody { msgid, respond } => {
            if !respond.is_closed() {
               body_msgids.push(msgid);
               body_responders.push(respond);
            }
         },
         AccountRequest::FetchBodies { msgids, respond } => {
            if !respond.is_closed() {
               body_msgids.extend(msgids);
               body_responders.push(respond);
            }
         },
         AccountRequest::StoreFlags {
            msgid,
            add,
            remove,
            respond,
         } => {
            store_ops.push(sync::StoreFlagsOp { msgid, add, remove });
            store_responders.push(respond);
         },
         other => others.push(other),
      }
   }

   if !body_msgids.is_empty() {
      match sync::fetch_and_cache_bodies(session, pool, account_id, &body_msgids).await {
         Ok(()) => {
            for respond in body_responders {
               let _ = respond.send(Ok(()));
            }
         },
         Err(error) => {
            let message = error.to_string();
            for respond in body_responders {
               let _ = respond.send(Err(SyncError::Other(message.clone())));
            }
         },
      }
   }

   if !store_ops.is_empty() {
      let results = sync::store_flags_batch(session, pool, account_id, &store_ops).await;
      for (resp, res) in store_responders.into_iter().zip(results) {
         let _ = resp.send(res);
      }
   }

   for req in others {
      if process_request(req, session, pool, runtime).await == Flow::Exit {
         return Flow::Exit;
      }
   }
   Flow::Continue
}

/// Pick the folder we'll camp on with IDLE. Role-based so localized Gmail
/// accounts (Tous les messages, 所有邮件, etc.) don't reconnect-loop against a
/// folder name that was only ever correct for `en-US`.
async fn discover_primary_folder(
   session: &mut ImapSession,
   provider: ProviderKind,
) -> Result<String> {
   let folders = imap::list_folders(session).await?;
   pick_primary_folder(&folders, provider)
}

/// Pure policy: given a folder list + provider, pick the IDLE target.
///
/// Preference order:
///   1. Gmail only: role=`all`
///   2. role=`inbox`
///   3. Case-insensitive `INBOX` name match
///   4. First folder that the sync policy itself would not skip — we share
///      `sync::should_skip` here so we never park IDLE on the exact
///      `[Gmail]/Important` / `[Gmail]/Starred` folders the rest of the
///      pipeline intentionally ignores.
pub(crate) fn pick_primary_folder(
   folders: &[imap::ImapFolder],
   provider: ProviderKind,
) -> Result<String> {
   if provider == ProviderKind::Gmail
      && let Some(folder) = folders
         .iter()
         .find(|folder| !sync::should_skip(folder) && folder.role.as_deref() == Some("all"))
   {
      return Ok(folder.name.clone());
   }
   if let Some(folder) = folders
      .iter()
      .find(|folder| !sync::should_skip(folder) && folder.role.as_deref() == Some("inbox"))
   {
      return Ok(folder.name.clone());
   }
   if let Some(folder) = folders
      .iter()
      .find(|folder| !sync::should_skip(folder) && folder.name.eq_ignore_ascii_case("INBOX"))
   {
      return Ok(folder.name.clone());
   }
   if let Some(folder) = folders.iter().find(|folder| !sync::should_skip(folder)) {
      return Ok(folder.name.clone());
   }
   Err(SyncError::Other(
      "no selectable folder on server; cannot pick a primary folder for IDLE".into(),
   ))
}

async fn connect_and_auth(runtime: &AccountRuntime, pool: &PgPool) -> Result<ImapSession> {
   match runtime.provider {
      ProviderKind::Gmail => gmail_connect(runtime, pool).await,
      ProviderKind::Imap => imap_connect(runtime).await,
   }
}

/// Current Gmail access token: the cached one when still valid, otherwise a
/// fresh refresh-grant round trip (persisted back to the cache). Shared by
/// the IMAP connect path and SMTP submission.
async fn gmail_access_token(
   runtime: &AccountRuntime,
   pool: &PgPool,
   client_id: &str,
   client_secret: &str,
) -> Result<String> {
   let oauth = GmailOAuth::new(client_id, client_secret)?;

   let stored = db::get_oauth(pool, &runtime.id).await?.ok_or_else(|| {
      SyncError::OAuth(format!(
         "no refresh token for account {}; run `jmapper bootstrap --account {}` first",
         runtime.id, runtime.id
      ))
   })?;

   let now = Utc::now();
   match (stored.access_token.as_deref(), stored.expires_at) {
      (Some(tok), Some(exp)) if exp > (now.timestamp() + 60) => Ok(tok.to_owned()),
      _ => {
         debug!(account_id = %runtime.id, "refreshing OAuth access token");
         let fresh = oauth.refresh(&stored.refresh_token).await?;
         db::upsert_oauth(
            pool,
            &runtime.id,
            Some(&fresh.access_token),
            &fresh.refresh_token,
            fresh.expires_at,
         )
         .await?;
         Ok(fresh.access_token)
      },
   }
}

async fn gmail_connect(runtime: &AccountRuntime, pool: &PgPool) -> Result<ImapSession> {
   let auth = runtime
      .gmail
      .as_ref()
      .ok_or_else(|| SyncError::Other("missing gmail credentials".into()))?;
   let client = imap::connect("imap.gmail.com", 993, ImapTls::Implicit).await?;
   if let Some(password) = auth.app_password() {
      client
         .login(&runtime.email, password)
         .await
         .map_err(|(error, _)| SyncError::Imap(error))
   } else {
      let (client_id, client_secret) = auth
         .oauth()
         .ok_or_else(|| SyncError::Other("missing gmail credentials".into()))?;
      let access_token = gmail_access_token(runtime, pool, client_id, client_secret).await?;
      imap::authenticate_xoauth2(client, &runtime.email, &access_token).await
   }
}

/// Resolve the SMTP endpoint + credentials for this account. Gmail defaults
/// to smtp.gmail.com:465 with the same authentication as IMAP;
/// `[accounts.smtp]` can override host/port/TLS. Generic IMAP requires explicit
/// config, with username/password falling back to the IMAP login.
async fn smtp_params_for(runtime: &AccountRuntime, pool: &PgPool) -> Result<smtp::SmtpParams> {
   use crate::smtp::{
      SmtpAuth,
      SmtpParams,
   };
   match runtime.provider {
      ProviderKind::Gmail => {
         let (host, port, tls) = runtime.smtp.as_ref().map_or_else(
            || ("smtp.gmail.com".to_owned(), 465, ImapTls::Implicit),
            |config| (config.host.clone(), config.port, config.tls),
         );
         let gmail = runtime
            .gmail
            .as_ref()
            .ok_or_else(|| SyncError::Other("missing gmail credentials".into()))?;
         let auth = if let Some(password) = gmail.app_password() {
            SmtpAuth::Plain {
               username: runtime.email.clone(),
               password,
            }
         } else {
            let (client_id, client_secret) = gmail
               .oauth()
               .ok_or_else(|| SyncError::Other("missing gmail credentials".into()))?;
            SmtpAuth::XOAuth2 {
               email:        runtime.email.clone(),
               access_token: gmail_access_token(runtime, pool, client_id, client_secret).await?,
            }
         };
         Ok(SmtpParams {
            host,
            port,
            tls,
            auth,
         })
      },
      ProviderKind::Imap => {
         let smtp = runtime.smtp.as_ref().ok_or_else(|| {
            SyncError::Other(
               "no [accounts.smtp] configured for this account; submission disabled".into(),
            )
         })?;
         let imap = runtime
            .imap
            .as_ref()
            .ok_or_else(|| SyncError::Other("missing imap credentials".into()))?;
         Ok(SmtpParams {
            host: smtp.host.clone(),
            port: smtp.port,
            tls:  smtp.tls,
            auth: SmtpAuth::Plain {
               username: smtp
                  .username
                  .clone()
                  .unwrap_or_else(|| imap.username.clone()),
               password: smtp
                  .password
                  .clone()
                  .unwrap_or_else(|| imap.password.clone()),
            },
         })
      },
   }
}

/// `AccountRequest::SubmitEmail` — pull the raw RFC 5322 bytes (fetching
/// over IMAP if the body cache is cold) and hand them to the SMTP client.
async fn submit_email(
   session: &mut ImapSession,
   pool: &PgPool,
   runtime: &AccountRuntime,
   msgid: &str,
   mail_from: &str,
   rcpt_to: &[String],
) -> Result<String> {
   let load = || {
      async {
         Ok::<_, SyncError>(
            queries::raw_messages::raw_message_bytes()
               .bind(&db::client(pool).await?, &runtime.id.as_str(), &msgid)
               .opt()
               .await?
               .map(|row| row.raw_rfc822),
         )
      }
   };
   let raw = if let Some(bytes) = load().await? {
      bytes
   } else {
      sync::fetch_and_cache_body(session, pool, &runtime.id, msgid).await?;
      load().await?.ok_or_else(|| {
         SyncError::Other(format!("no raw bytes cached for msgid {msgid} after fetch"))
      })?
   };

   let params = smtp_params_for(runtime, pool).await?;
   info!(
       account_id = %runtime.id,
       msgid,
       rcpt = rcpt_to.len(),
       host = %params.host,
       "submitting message over SMTP",
   );
   smtp::submit(&params, mail_from, rcpt_to, &raw).await
}

async fn imap_connect(runtime: &AccountRuntime) -> Result<ImapSession> {
   let creds = runtime
      .imap
      .as_ref()
      .ok_or_else(|| SyncError::Other("missing imap credentials".into()))?;
   let client = imap::connect(&creds.host, creds.port, creds.tls).await?;
   let session = client
      .login(&creds.username, &creds.password)
      .await
      .map_err(|(err, _)| SyncError::Imap(err))?;
   Ok(session)
}

#[cfg(test)]
mod tests {
   use super::*;
   use crate::imap::ImapFolder;

   fn folder(name: &str, flags: &[&str], role: Option<&str>) -> ImapFolder {
      ImapFolder {
         name:      name.into(),
         delimiter: '/',
         flags:     flags.iter().map(|flag| (*flag).to_owned()).collect(),
         role:      role.map(str::to_owned),
      }
   }

   /// Gmail path: `role=all` wins over `INBOX`, even when INBOX appears first
   /// in the list. This is the regression target for the hard-coded
   /// `[Gmail]/All Mail` string — a localized account returns `role=all` on
   /// whatever the server chose to call its All Mail folder.
   #[test]
   fn primary_prefers_all_for_gmail_localized() {
      let folders = vec![
         folder("INBOX", &[], Some("inbox")),
         folder("[Gmail]/Tous les messages", &["all"], Some("all")),
      ];
      let got = pick_primary_folder(&folders, ProviderKind::Gmail).unwrap();
      assert_eq!(got, "[Gmail]/Tous les messages");
   }

   /// Non-Gmail IMAP: `role=all` is ignored (providers like Fastmail label
   /// their archive as All), so INBOX wins by role.
   #[test]
   fn primary_prefers_inbox_for_plain_imap() {
      let folders = vec![
         folder("Archive", &["all"], Some("all")),
         folder("INBOX", &[], Some("inbox")),
      ];
      let got = pick_primary_folder(&folders, ProviderKind::Imap).unwrap();
      assert_eq!(got, "INBOX");
   }

   /// Fallback path: no roles at all, still uses the case-insensitive INBOX
   /// name match before the last-resort first-selectable pick.
   #[test]
   fn primary_name_fallback_inbox_case_insensitive() {
      let folders = vec![
         folder("Sent", &[], None),
         folder("Inbox", &[], None), // note lowercase
      ];
      let got = pick_primary_folder(&folders, ProviderKind::Imap).unwrap();
      assert_eq!(got, "Inbox");
   }

   /// When `role=all` is missing on Gmail, fall through to the inbox role so
   /// a mis-configured Gmail account still reaches IDLE instead of erroring
   /// or reconnect-looping. Also verifies the skip filter doesn't chase
   /// `[Gmail]/Starred` just because it shows up first.
   #[test]
   fn primary_gmail_falls_back_to_inbox_and_skips_starred() {
      let folders = vec![
         folder("[Gmail]/Starred", &["flagged"], None),
         folder("INBOX", &[], Some("inbox")),
      ];
      let got = pick_primary_folder(&folders, ProviderKind::Gmail).unwrap();
      assert_eq!(got, "INBOX");
   }

   /// Last-resort fallback must still honor `sync::should_skip`: given only
   /// `[Gmail]/Important` (which sync explicitly skips) + `noselect`
   /// containers, we should error rather than park IDLE on a folder the
   /// rest of the pipeline ignores.
   #[test]
   fn primary_refuses_to_idle_on_skipped_folders() {
      let folders = vec![
         folder("[Gmail]", &["noselect"], None),
         folder("[Gmail]/Important", &["flagged"], None),
      ];
      pick_primary_folder(&folders, ProviderKind::Gmail).unwrap_err();
   }

   #[test]
   fn interactive_idle_wake_skips_reconcile() {
      assert!(!idle_wake_needs_reconcile(&IdleResponse::ManualInterrupt));
      assert!(idle_wake_needs_reconcile(&IdleResponse::Timeout));
   }
}
