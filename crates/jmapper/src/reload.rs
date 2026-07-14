//! SIGHUP account-task reconciliation.

use std::{
   collections::{
      BTreeSet,
      HashMap,
      HashSet,
   },
   path::PathBuf,
   sync::Arc,
   time::Duration,
};

use anyhow::{
   Context as _,
   Result,
};
use dav_sync::{
   service::{
      self,
      DavHandle,
      DavTask,
   },
   store::{
      self as dav_store,
      DavKind,
   },
};
use deadpool_postgres::Pool as PgPool;
use imap_sync::{
   account::{
      AccountHandle,
      AccountRequest,
      AccountRuntime,
      ImapRuntime,
      SmtpRuntime,
      spawn,
   },
   db,
   provider::{
      ImapTls,
      ProviderKind,
   },
};
use jmap_server::{
   AccountInfo,
   AppState,
   state::{
      AppStateLive,
      DavAvailability,
   },
};
use sha1::{
   Digest as _,
   Sha1,
};
use tokio::{
   sync::{
      Mutex,
      oneshot,
   },
   time,
};
use tracing::{
   info,
   warn,
};

use crate::config::{
   self,
   Account,
   Config,
   DavEndpointConfig,
};

pub struct ManagedDav {
   pub service:      DavTask,
   pub availability: DavAvailability,
}

pub struct ReloadContext {
   pub config_path:  PathBuf,
   pub pool:         PgPool,
   pub state:        AppState,
   pub handles:      Mutex<HashMap<String, AccountHandle>>,
   pub dav_tasks:    Mutex<HashMap<String, ManagedDav>>,
   pub fingerprints: Mutex<HashMap<String, [u8; 20]>>,
}

impl ReloadContext {
   pub fn new(config_path: PathBuf, pool: PgPool, state: AppState) -> Self {
      Self {
         config_path,
         pool,
         state,
         handles: Mutex::new(HashMap::new()),
         dav_tasks: Mutex::new(HashMap::new()),
         fingerprints: Mutex::new(HashMap::new()),
      }
   }

   pub async fn seed(
      &self,
      accounts: &[Account],
      handles: HashMap<String, AccountHandle>,
      dav_tasks: HashMap<String, ManagedDav>,
   ) {
      *self.handles.lock().await = handles;
      *self.dav_tasks.lock().await = dav_tasks;
      let mut fps = self.fingerprints.lock().await;
      fps.clear();
      for account in accounts {
         fps.insert(account.id.clone(), fingerprint(account));
      }
   }
}

/// Fingerprint every account setting consumed by a running task.
pub fn fingerprint(account: &Account) -> [u8; 20] {
   let mut hasher = Sha1::new();
   hasher.update(account.email.as_bytes());
   hasher.update(b"\0");
   hasher.update(account.bearer_token.as_bytes());
   hasher.update(b"\0");
   hasher.update(account.display_name.as_bytes());
   hasher.update(b"\0");
   match account.provider {
      ProviderKind::Gmail => hasher.update(b"gmail"),
      ProviderKind::Imap => hasher.update(b"imap"),
   }
   hasher.update(b"\0");
   hasher.update(account.backfill_days.to_be_bytes());
   hasher.update(b"\0");
   if let Some(gmail) = &account.gmail {
      if let Some(password) = gmail.app_password() {
         hasher.update(b"app-password\0");
         hasher.update(password.as_bytes());
      } else if let Some((client_id, client_secret)) = gmail.oauth() {
         hasher.update(b"oauth\0");
         hasher.update(client_id.as_bytes());
         hasher.update(b"\0");
         hasher.update(client_secret.as_bytes());
      }
   }
   if let Some(i) = &account.imap {
      hasher.update(i.host.as_bytes());
      hasher.update(b"\0");
      hasher.update(i.port.to_be_bytes());
      hasher.update(match i.tls {
         ImapTls::Implicit => b"implicit".as_slice(),
         ImapTls::Starttls => b"starttls".as_slice(),
      });
      hasher.update(i.username.as_bytes());
      hasher.update(b"\0");
      hasher.update(i.password.as_bytes());
   }
   if let Some(smtp) = &account.smtp {
      hasher.update(smtp.host.as_bytes());
      hasher.update(b"\0");
      hasher.update(smtp.port.to_be_bytes());
      hasher.update(match smtp.tls {
         ImapTls::Implicit => b"implicit".as_slice(),
         ImapTls::Starttls => b"starttls".as_slice(),
      });
      hasher.update(smtp.username.as_deref().unwrap_or("").as_bytes());
      hasher.update(b"\0");
      hasher.update(smtp.password.as_deref().unwrap_or("").as_bytes());
   }
   fingerprint_dav(&mut hasher, b"caldav", account.caldav.as_ref());
   fingerprint_dav(&mut hasher, b"carddav", account.carddav.as_ref());
   let mut out = [0_u8; 20];
   out.copy_from_slice(&hasher.finalize());
   out
}

fn fingerprint_dav(hasher: &mut Sha1, label: &[u8], endpoint: Option<&DavEndpointConfig>) {
   hasher.update(b"\0");
   hasher.update(label);
   hasher.update(b"\0");
   let Some(endpoint) = endpoint else {
      hasher.update(b"none");
      return;
   };
   hasher.update(endpoint.url().as_bytes());
   hasher.update(b"\0");
   let (auth_kind, auth_user, auth_secret) = endpoint.auth_parts();
   hasher.update(auth_kind.as_bytes());
   hasher.update(b"\0");
   hasher.update(auth_user.unwrap_or_default().as_bytes());
   hasher.update(b"\0");
   hasher.update(auth_secret.unwrap_or_default().as_bytes());
}

/// Remove dead tasks and reconcile them from the current config.
pub async fn supervise_tick(ctx: &Arc<ReloadContext>) {
   let dead = {
      let mut handles = ctx.handles.lock().await;
      let dead_ids = handles
         .iter()
         .filter(|(_, handle)| handle.task.is_finished())
         .map(|(id, _)| id.clone())
         .collect::<Vec<String>>();
      for id in &dead_ids {
         if let Some(handle) = handles.remove(id) {
            match handle.task.await {
               Ok(()) => {
                  warn!(account_id = %id, "task exited without a shutdown request; will respawn");
               },
               Err(err) if err.is_panic() => {
                  warn!(
                      account_id = %id,
                      "account task panicked; will respawn",
                  );
               },
               Err(err) => {
                  warn!(account_id = %id, error = %err, "account task aborted; will respawn");
               },
            }
         }
      }
      dead_ids
   };

   let dead_dav = {
      let mut tasks = ctx.dav_tasks.lock().await;
      let dead_ids = tasks
         .iter()
         .filter(|(_, managed)| managed.service.task.is_finished())
         .map(|(id, _)| id.clone())
         .collect::<Vec<String>>();
      for id in &dead_ids {
         if let Some(managed) = tasks.remove(id) {
            match managed.service.task.await {
               Ok(()) => {
                  warn!(
                      account_id = %id,
                      "DAV task exited without shutdown; will respawn"
                  );
               },
               Err(err) if err.is_panic() => {
                  warn!(account_id = %id, "DAV task panicked; will respawn");
               },
               Err(err) => {
                  warn!(
                      account_id = %id,
                      error = %err,
                      "DAV task aborted; will respawn"
                  );
               },
            }
         }
      }
      dead_ids
   };

   if (!dead.is_empty() || !dead_dav.is_empty())
      && let Err(err) = reload(ctx).await
   {
      warn!(error = %err, ?dead, ?dead_dav, "supervise: reload after task death failed");
   }
}

pub async fn reload(ctx: &Arc<ReloadContext>) -> Result<()> {
   info!(path = %ctx.config_path.display(), "SIGHUP received; reloading config");
   let new_config = config::Config::load(&ctx.config_path)
      .with_context(|| format!("re-reading {}", ctx.config_path.display()))?;

   warn_if_immutable_changed_early(&new_config);

   let mut handles = ctx.handles.lock().await;
   let mut dav_tasks = ctx.dav_tasks.lock().await;
   let mut fingerprints = ctx.fingerprints.lock().await;

   let new_ids = new_config
      .accounts
      .iter()
      .map(|account| account.id.as_str())
      .collect::<HashSet<&str>>();
   let old_ids = handles
      .keys()
      .chain(dav_tasks.keys())
      .cloned()
      .collect::<BTreeSet<String>>();

   let mut to_stop = BTreeSet::<String>::new();
   for id in &old_ids {
      if !new_ids.contains(id.as_str()) {
         to_stop.insert(id.clone());
      }
   }
   for account in &new_config.accounts {
      let new_fp = fingerprint(account);
      if let Some(old_fp) = fingerprints.get(&account.id)
         && *old_fp != new_fp
      {
         to_stop.insert(account.id.clone());
      }
   }
   for id in &to_stop {
      if let Some(handle) = handles.remove(id) {
         stop_one(id, handle).await;
      }
      if let Some(managed) = dav_tasks.remove(id) {
         stop_dav_one(id, managed).await;
      }
      fingerprints.remove(id);
   }

   for account in &new_config.accounts {
      if handles.contains_key(&account.id) {
         continue; // still running, unchanged
      }
      match spawn_one(&ctx.pool, account).await {
         Ok(handle) => {
            info!(account_id = %account.id, "spawned account task");
            handles.insert(account.id.clone(), handle);
            fingerprints.insert(account.id.clone(), fingerprint(account));
         },
         Err(err) => {
            warn!(account_id = %account.id, error = %err, "failed to spawn on reload; skipping");
         },
      }
   }
   drop(fingerprints);

   for account in &new_config.accounts {
      if dav_tasks.contains_key(&account.id) {
         continue;
      }
      match spawn_dav_one(&ctx.pool, account).await {
         Ok((Some(managed), initial_sync)) => {
            let handle = managed.service.handle.clone();
            for (kind, force) in initial_sync {
               let handle = handle.clone();
               let account_id = account.id.clone();
               tokio::spawn(async move {
                  if let Err(err) = handle.sync_now(kind, force).await {
                     warn!(
                         account_id = %account_id,
                         kind = kind.as_str(),
                         error = %err,
                         "initial DAV sync failed"
                     );
                  }
               });
            }
            dav_tasks.insert(account.id.clone(), managed);
            info!(account_id = %account.id, "spawned DAV account task");
         },
         Ok((None, _)) => {},
         Err(err) => {
            warn!(account_id = %account.id, error = %err, "failed to spawn DAV on reload; skipping");
         },
      }
   }

   let mut account_senders = HashMap::new();
   let mut dav_handles = HashMap::<String, DavHandle>::new();
   let mut dav_availability = HashMap::<String, DavAvailability>::new();
   let mut http_accounts = Vec::with_capacity(new_config.accounts.len());
   for account in &new_config.accounts {
      if let Some(handle) = handles.get(&account.id) {
         account_senders.insert(account.id.clone(), handle.tx.clone());
      }
      if let Some(managed) = dav_tasks.get(&account.id) {
         dav_handles.insert(account.id.clone(), managed.service.handle.clone());
         dav_availability.insert(account.id.clone(), managed.availability);
      }
      http_accounts.push(AccountInfo::from_bearer_token(
         &account.id,
         &account.email,
         &account.display_name,
         &account.bearer_token,
      ));
   }
   let running = handles.len();
   let dav_running = dav_tasks.len();
   drop(handles);
   drop(dav_tasks);
   ctx.state.swap(AppStateLive {
      accounts: http_accounts,
      account_senders,
      dav_handles,
      dav_availability,
   });
   info!(
      accounts = new_config.accounts.len(),
      running, dav_running, "reload complete",
   );
   Ok(())
}

pub async fn stop_one(id: &str, handle: AccountHandle) {
   // A dropped JoinHandle detaches its task, so retain an explicit abort path.
   let abort = handle.task.abort_handle();

   let (tx, rx) = oneshot::channel();
   if handle
      .tx
      .send(AccountRequest::Shutdown { respond: tx })
      .await
      .is_err()
   {
      abort.abort();
      return;
   }
   if time::timeout(Duration::from_secs(5), rx).await == Ok(Ok(())) {
      if time::timeout(Duration::from_secs(5), handle.task)
         .await
         .is_ok()
      {
         info!(account_id = id, "account task stopped");
      } else {
         warn!(
            account_id = id,
            "task didn't exit within 5s of shutdown ack; aborting"
         );
         abort.abort();
      }
   } else {
      warn!(account_id = id, "shutdown ack timeout; aborting task");
      abort.abort();
      let _ = time::timeout(Duration::from_secs(1), handle.task).await;
   }
}

pub async fn spawn_one(pool: &PgPool, account: &Account) -> Result<AccountHandle> {
   let bearer_hash = {
      use sha2::{
         Digest as _,
         Sha256,
      };
      let mut hasher = Sha256::new();
      hasher.update(account.bearer_token.as_bytes());
      hasher.finalize().to_vec()
   };
   db::upsert_account(
      pool,
      &account.id,
      &account.email,
      account.provider,
      &account.display_name,
      &bearer_hash,
   )
   .await
   .with_context(|| format!("upserting account {}", account.id))?;

   let runtime = AccountRuntime {
      id:            account.id.clone(),
      email:         account.email.clone(),
      provider:      account.provider,
      backfill_days: account.backfill_days,
      gmail:         account.gmail.clone(),
      imap:          account.imap.as_ref().map(|i| {
         ImapRuntime {
            host:     i.host.clone(),
            port:     i.port,
            tls:      i.tls,
            username: i.username.clone(),
            password: i.password.clone(),
         }
      }),
      smtp:          account.smtp.as_ref().map(|creds| {
         SmtpRuntime {
            host:     creds.host.clone(),
            port:     creds.port,
            tls:      creds.tls,
            username: creds.username.clone(),
            password: creds.password.clone(),
         }
      }),
   };
   Ok(spawn(runtime, pool.clone()))
}

/// Persist DAV endpoints and spawn the account's serialized DAV task.
pub async fn spawn_dav_one(
   pool: &PgPool,
   account: &Account,
) -> Result<(Option<ManagedDav>, Vec<(DavKind, bool)>)> {
   let client = pool
      .get()
      .await
      .context("getting postgres client for DAV configuration")?;
   let mut initial_sync = Vec::<(DavKind, bool)>::new();
   let mut availability = DavAvailability::default();

   for (kind, endpoint) in [
      (DavKind::CalDav, account.caldav.as_ref()),
      (DavKind::CardDav, account.carddav.as_ref()),
   ] {
      if let Some(endpoint) = endpoint {
         let (auth_kind, auth_user, auth_secret) = endpoint.auth_parts();
         let force = dav_store::upsert_endpoint(
            &client,
            &account.id,
            kind,
            endpoint.url(),
            auth_kind,
            auth_user,
            auth_secret,
         )
         .await
         .with_context(|| format!("configuring {} for {}", kind.as_str(), account.id))?;
         initial_sync.push((kind, force));
         match kind {
            DavKind::CalDav => availability.calendars = true,
            DavKind::CardDav => availability.contacts = true,
         }
      } else {
         dav_store::delete_endpoint(&client, &account.id, kind)
            .await
            .with_context(|| format!("removing {} for {}", kind.as_str(), account.id))?;
      }
   }
   drop(client);

   if !availability.any() {
      return Ok((None, initial_sync));
   }
   Ok((
      Some(ManagedDav {
         service: service::spawn_managed(pool.clone(), account.id.clone()),
         availability,
      }),
      initial_sync,
   ))
}

pub async fn stop_dav_one(id: &str, managed: ManagedDav) {
   let abort = managed.service.task.abort_handle();
   let handle = managed.service.handle;
   let task = managed.service.task;
   if time::timeout(Duration::from_secs(5), handle.shutdown())
      .await
      .is_err()
   {
      warn!(account_id = id, "DAV shutdown timeout; aborting task");
      abort.abort();
   }
   if time::timeout(Duration::from_secs(5), task).await.is_err() {
      warn!(account_id = id, "DAV task did not exit; aborting task");
      abort.abort();
   }
}

fn warn_if_immutable_changed_early(_new: &Config) {
   tracing::debug!(
      "hot reload applies to [[accounts]] only; `server.bind`, `server.session_url`, \
       `server.cors_origins`, and `server.database_url` changes require a full restart"
   );
}

#[cfg(test)]
mod tests {
   use std::time::Duration;

   use imap_sync::account::{
      AccountHandle,
      AccountRequest,
   };
   use tokio::{
      sync::mpsc,
      time,
   };

   use super::*;

   #[tokio::test]
   async fn stop_one_aborts_unresponsive_task() {
      let (tx, mut rx) = mpsc::channel::<AccountRequest>(4);
      let task = tokio::spawn(async move {
         while let Some(req) = rx.recv().await {
            drop(req);
         }
      });
      let handle = AccountHandle { tx, task };
      let abort = handle.task.abort_handle();

      let stop_fut = stop_one("unresponsive", handle);
      time::timeout(Duration::from_secs(10), stop_fut)
         .await
         .expect("stop_one must return within 10s");

      for _ in 0..20 {
         if abort.is_finished() {
            break;
         }
         time::sleep(Duration::from_millis(25)).await;
      }
      assert!(
         abort.is_finished(),
         "stop_one returned but the task is still running (abort was not called)",
      );
   }

   #[tokio::test]
   async fn stop_one_graceful_ack() {
      let (tx, mut rx) = mpsc::channel::<AccountRequest>(4);
      let task = tokio::spawn(async move {
         while let Some(req) = rx.recv().await {
            if let AccountRequest::Shutdown { respond } = req {
               let _ = respond.send(());
               return;
            }
         }
      });
      let handle = AccountHandle { tx, task };
      let abort = handle.task.abort_handle();

      stop_one("graceful", handle).await;

      assert!(abort.is_finished());
   }
}
