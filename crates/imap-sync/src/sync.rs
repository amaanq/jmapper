//! IMAP ingestion, reconciliation, and remote mutation.

use std::{
   borrow::Cow,
   collections::{
      BTreeMap,
      BTreeSet,
      HashMap,
      HashSet,
   },
   iter,
   result::Result as StdResult,
   str::from_utf8,
};

use async_imap::{
   error::Error as ImapError,
   imap_proto::types::Envelope,
   types::Fetch,
};
use chrono::{
   DateTime,
   Utc,
};
use deadpool_postgres::Pool as PgPool;
use futures::{
   StreamExt as _,
   TryStreamExt as _,
};
use jmap_protocol::{
   email::{
      EmailAddress,
      EmailBodyPart,
      EmailBodyValue,
      EmailHeader,
   },
   ids::Id,
};
use jmapper_codegen::queries;
use sha1::{
   Digest as _,
   Sha1,
};
use tracing::{
   debug,
   info,
   warn,
};

use crate::{
   account::ImportOutcome,
   db,
   error::{
      Result,
      SyncError,
   },
   imap::{
      self,
      GMAIL_EXT_CAPABILITY,
      ImapFolder,
      ImapSession,
   },
   threading::{
      self,
      ThreadInputs,
   },
};

/// How many recent UIDs to fetch when a bounded backfill's date search fails.
pub const INITIAL_WINDOW: u32 = 500;
const MAX_PREFETCH_MESSAGE_BYTES: i64 = 2 * 1024 * 1024;

/// Redundant Gmail views that add no distinct messages.
const SKIP_FOLDERS: &[&str] = &[
   "[Gmail]",
   "[Gmail]/Important", // duplicates of All Mail with extra flag, noisy
   "[Gmail]/Starred",   // ditto
];

pub struct SyncStats {
   pub folders_synced:    u32,
   pub messages_upserted: u64,
}

/// Per-pass knobs threaded from the account task into every folder sync.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SyncOptions {
   pub backfill_days:   u32,
   /// Server advertises X-GM-EXT-1: prefer X-GM-THRID for threading.
   pub gmail_thrid:     bool,
   /// Interactive requests (move, destroy, import) reconcile with this off
   /// so a huge folder cannot turn one delete into a full-history fetch.
   pub deepen_history:  bool,
   /// The expunge sweep runs `UID SEARCH ALL`, seconds on six-figure
   /// folders, so interactive reconciles skip it.
   pub detect_expunges: bool,
}

/// Per-connection capabilities, probed once at login.
#[derive(Debug, Clone, Copy)]
pub struct SessionTraits {
   /// X-GM-EXT-1. Gmail flags are message-global and a trash move strips
   /// every label, which lets interactive paths skip whole IMAP legs.
   pub gmail:    bool,
   pub has_move: bool,
   pub uidplus:  bool,
}

/// # Errors
///
/// Returns [`SyncError`] if the CAPABILITY probe fails.
pub async fn detect_session_traits(session: &mut ImapSession) -> Result<SessionTraits> {
   Ok(SessionTraits {
      gmail:    imap::has_capability(session, GMAIL_EXT_CAPABILITY).await?,
      has_move: imap::has_capability(session, "MOVE").await?,
      uidplus:  imap::has_capability(session, "UIDPLUS").await?,
   })
}

async fn sync_options(session: &mut ImapSession, backfill_days: u32) -> SyncOptions {
   let gmail_thrid = imap::has_capability(session, GMAIL_EXT_CAPABILITY)
      .await
      .unwrap_or(false);
   SyncOptions {
      backfill_days,
      gmail_thrid,
      deepen_history: true,
      detect_expunges: true,
   }
}

const fn interactive_options(traits: SessionTraits) -> SyncOptions {
   SyncOptions {
      backfill_days:   0,
      gmail_thrid:     traits.gmail,
      deepen_history:  false,
      detect_expunges: false,
   }
}

fn wants_history_deepening(opts: SyncOptions, previous_uidfirst: Option<u32>) -> bool {
   opts.deepen_history && opts.backfill_days == 0 && previous_uidfirst != Some(1)
}

fn fresh_backfill_low(opts: SyncOptions, uidnext: u32) -> u32 {
   if opts.backfill_days == 0 && opts.deepen_history {
      1
   } else {
      uidnext.saturating_sub(INITIAL_WINDOW).max(1)
   }
}

/// # Errors
///
/// Returns [`SyncError`] if listing the account's folders over IMAP fails, or
/// if the closing bookkeeping (`recompute_mailbox_counts`,
/// `mark_initial_sync_done`) cannot reach Postgres. Individual folder sync
/// failures are logged and skipped rather than aborting the pass.
pub async fn initial_sync(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   backfill_days: u32,
) -> Result<SyncStats> {
   let folders = imap::list_folders(session).await?;
   info!(account_id, n = folders.len(), "discovered folders");

   let opts = sync_options(session, backfill_days).await;
   let mut stats = SyncStats {
      folders_synced:    0,
      messages_upserted: 0,
   };

   for folder in &folders {
      if should_skip(folder) {
         debug!(name = %folder.name, "skipping");
         continue;
      }
      match sync_folder(session, pool, account_id, folder, opts).await {
         Ok(n) => {
            stats.folders_synced += 1;
            stats.messages_upserted += n;
            info!(name = %folder.name, n, "folder synced");
         },
         Err(err) => {
            warn!(name = %folder.name, error = %err, "folder sync failed; continuing");
         },
      }
   }

   db::recompute_mailbox_counts(pool, account_id).await?;
   // Flip the readyz gate: even an empty account is "synced" once the first
   // pass over its folders finished without the transport blowing up.
   db::mark_initial_sync_done(pool, account_id).await?;
   Ok(stats)
}

pub(crate) fn should_skip(folder: &ImapFolder) -> bool {
   if folder.flags.iter().any(|flag| flag == "noselect") {
      return true;
   }
   SKIP_FOLDERS.iter().any(|skip| folder.name == *skip)
}

async fn sync_folder(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   folder: &ImapFolder,
   opts: SyncOptions,
) -> Result<u64> {
   let mut stored = queries::folders::folder_by_name()
      .bind(&db::client(pool).await?, &account_id, &folder.name.as_str())
      .opt()
      .await?;

   // SELECT returns a Mailbox struct with uidvalidity, exists, uidnext, etc.
   let mbox = session.select(&folder.name).await?;
   let uidvalidity = mbox.uid_validity.unwrap_or(0);
   let uidnext = mbox.uid_next.unwrap_or(0);
   let highestmodseq = mbox.highest_modseq.unwrap_or(0);
   let exists = mbox.exists;

   if let Some(previous) = stored.as_ref()
      && previous.uidvalidity != 0
      && previous.uidvalidity as u32 != uidvalidity
   {
      db::purge_folder_for_uidvalidity_change(pool, account_id, previous.id, &previous.mailbox_id)
         .await?;
      stored = None;
   }
   let fresh = stored.as_ref().is_none_or(|row| row.uidnext == 0);
   let previous_uidnext = stored
      .as_ref()
      .and_then(|row| u32::try_from(row.uidnext).ok())
      .filter(|uid| *uid > 0);
   let previous_uidfirst = stored
      .as_ref()
      .and_then(|row| u32::try_from(row.uidfirst).ok());

   let mailbox_id = stored.as_ref().map_or_else(
      || mailbox_id_for_folder(account_id, &folder.name),
      |row| row.mailbox_id.clone(),
   );
   db::upsert_mailbox(
      pool,
      &mailbox_id,
      account_id,
      &display_name_for(&folder.name),
      None,
      folder.role.as_deref(),
      sort_order_for_role(folder.role.as_deref()),
   )
   .await?;
   let folder_id = db::upsert_folder(pool, db::FolderUpsert {
      account_id,
      imap_name: &folder.name,
      uidvalidity,
      // Advance this checkpoint only after the corresponding FETCH and DB
      // writes succeed. Otherwise a transient failure would permanently
      // skip the new UID range on the next pass.
      uidnext: previous_uidnext.unwrap_or(0),
      highestmodseq,
      role: folder.role.as_deref(),
      mailbox_id: &mailbox_id,
   })
   .await?;

   if exists == 0 {
      if fresh || opts.backfill_days == 0 {
         queries::folders::set_folder_uidfirst()
            .bind(&db::client(pool).await?, &1_i64, &folder_id)
            .await?;
      }
      queries::folders::set_folder_uidnext()
         .bind(&db::client(pool).await?, &i64::from(uidnext), &folder_id)
         .await?;
      return Ok(0);
   }

   let mut ranges = Vec::<String>::new();
   let target_uidfirst = if fresh {
      let mut low = fresh_backfill_low(opts, uidnext);
      if opts.backfill_days > 0 {
         let since =
            (Utc::now() - chrono::Duration::days(i64::from(opts.backfill_days))).format("%d-%b-%Y");
         match session.uid_search(format!("SINCE {since}")).await {
            Ok(uids) if uids.is_empty() => {
               debug!(name = %folder.name, %since, "no messages since backfill horizon");
               low = uidnext.max(1);
            },
            Ok(uids) => {
               low = uids.iter().copied().min().unwrap_or(low).max(1);
               debug!(name = %folder.name, %since, low, n = uids.len(), "backfill window from SEARCH");
            },
            Err(err) => {
               warn!(name = %folder.name, error = %err, "UID SEARCH SINCE failed; using window fallback");
            },
         }
      }
      push_uid_range(&mut ranges, low, uidnext);
      Some(low)
   } else {
      let target = if wants_history_deepening(opts, previous_uidfirst) {
         let previous_uidnext = previous_uidnext.unwrap_or(uidnext);
         if let Some(high) = older_history_end(previous_uidfirst, previous_uidnext) {
            push_uid_range(&mut ranges, 1, high.saturating_add(1));
         }
         Some(1)
      } else {
         previous_uidfirst
      };
      if let Some(low) = previous_uidnext {
         push_uid_range(&mut ranges, low, uidnext);
      }
      target
   };

   if ranges.is_empty() {
      if let Some(low) = target_uidfirst
         && previous_uidfirst != Some(low)
      {
         queries::folders::set_folder_uidfirst()
            .bind(&db::client(pool).await?, &i64::from(low), &folder_id)
            .await?;
      }
      queries::folders::set_folder_uidnext()
         .bind(&db::client(pool).await?, &i64::from(uidnext), &folder_id)
         .await?;
      return Ok(0);
   }
   let uid_set = ranges.join(",");

   let mut count = 0_u64;
   let query = "(UID FLAGS INTERNALDATE RFC822.SIZE ENVELOPE BODY.PEEK[HEADER.FIELDS (MESSAGE-ID \
                REFERENCES IN-REPLY-TO)])";

   // Collect before threading: the fetch stream borrows the session, and the
   // X-GM-THRID pass needs it back for a second command.
   let mut decoded = Vec::<DecodedFetch>::new();
   let mut skipped = Vec::<u32>::new();
   {
      let mut fetches = session.uid_fetch(&uid_set, query).await?;
      while let Some(fetch) = fetches.next().await {
         let fetch = fetch?;
         match decode_fetch(&fetch, uidvalidity) {
            FetchDecode::Complete(env) => decoded.push(*env),
            FetchDecode::Incomplete { uid } => {
               warn!(
                   name = %folder.name,
                   uid,
                   "FETCH missing INTERNALDATE/RFC822.SIZE/ENVELOPE; deferring to next round",
               );
               skipped.push(uid);
            },
            FetchDecode::Anonymous => {
               warn!(name = %folder.name, "FETCH response without UID; ignoring");
            },
         }
      }
   }
   let gm_thrids = gmail_thrids_for(session, &decoded, opts).await;

   for mut env in decoded {
      env.envelope.thrid = thrid_for(
         pool,
         account_id,
         &env.envelope,
         gm_thrids.get(&env.uid).copied(),
      )
      .await?;
      // upsert_message only bumps modseq when the row actually changed.
      db::upsert_message(pool, account_id, &env.envelope).await?;
      db::upsert_message_imap(
         pool,
         account_id,
         &env.envelope.msgid,
         folder_id,
         env.uid,
         uidvalidity,
         &mailbox_id,
      )
      .await?;
      // Merge the folder into the message's mailbox set rather than
      // replacing it, so a message already seen in another folder keeps
      // its other memberships.
      queries::messages::add_message_mailbox()
         .bind(
            &db::client(pool).await?,
            &account_id,
            &env.envelope.msgid.as_str(),
            &mailbox_id.as_str(),
         )
         .await?;
      count += 1;
   }

   let (checkpoint_uidnext, allow_uidfirst) = skip_checkpoints(&skipped, previous_uidnext, uidnext);
   if allow_uidfirst
      && let Some(low) = target_uidfirst
      && previous_uidfirst != Some(low)
   {
      queries::folders::set_folder_uidfirst()
         .bind(&db::client(pool).await?, &i64::from(low), &folder_id)
         .await?;
   }
   queries::folders::set_folder_uidnext()
      .bind(
         &db::client(pool).await?,
         &i64::from(checkpoint_uidnext),
         &folder_id,
      )
      .await?;

   Ok(count)
}

fn push_uid_range(ranges: &mut Vec<String>, start: u32, end_exclusive: u32) {
   if start < end_exclusive {
      ranges.push(format!("{start}:{}", end_exclusive - 1));
   }
}

fn older_history_end(previous_uidfirst: Option<u32>, previous_uidnext: u32) -> Option<u32> {
   if previous_uidfirst == Some(1) {
      return None;
   }
   let high = previous_uidfirst
      .filter(|first| *first > 1)
      .map_or_else(
         || previous_uidnext.saturating_sub(1),
         |first| first.saturating_sub(1),
      )
      .min(previous_uidnext.saturating_sub(1));
   (high > 0).then_some(high)
}

/// X-GM-THRID map for freshly decoded fetches, when the server supports it.
/// Lookup failures degrade to RFC 5322 threading rather than failing sync.
async fn gmail_thrids_for(
   session: &mut ImapSession,
   decoded: &[DecodedFetch],
   opts: SyncOptions,
) -> HashMap<u32, u64> {
   if !opts.gmail_thrid || decoded.is_empty() {
      return HashMap::new();
   }
   let uids = decoded.iter().map(|fetch| fetch.uid).collect::<Vec<u32>>();
   match imap::fetch_gmail_thrids(session, &uids).await {
      Ok(map) => map,
      Err(err) => {
         warn!(error = %err, "X-GM-THRID fetch failed; falling back to RFC 5322 threading");
         HashMap::new()
      },
   }
}

/// Prefer Gmail's server-side thread id when available; record it in the
/// thread index so RFC 5322 lookups for later replies join the same thread.
async fn thrid_for(
   pool: &PgPool,
   account_id: &str,
   env: &db::MessageEnvelope,
   gm_thrid: Option<u64>,
) -> Result<String> {
   match gm_thrid {
      Some(tid) => {
         let thrid = format!("t-gm-{tid:x}");
         let inputs = thread_inputs(account_id, env);
         threading::record_known_thrid(pool, &inputs, &thrid).await?;
         Ok(thrid)
      },
      None => derive_thrid_for(pool, account_id, env).await,
   }
}

fn thread_inputs<'a>(account_id: &'a str, env: &'a db::MessageEnvelope) -> ThreadInputs<'a> {
   ThreadInputs {
      account_id,
      msgid: &env.msgid,
      message_id: env.message_id_header.as_deref(),
      in_reply_to: env.in_reply_to_header.as_deref(),
      references: env.references_header.as_deref(),
      subject: env.subject.as_deref(),
      received_at_secs: env.received_at.timestamp(),
   }
}

/// Hand `decode_fetch`'s envelope off to the threading module so new
/// messages pick up real RFC 5322 thrids instead of the `thrid = msgid`
/// singleton placeholder `decode_fetch` writes. A lookup failure is
/// treated as "new thread" — the upsert still succeeds with a newly-minted
/// thrid so sync never stalls on a transient index read.
async fn derive_thrid_for(
   pool: &PgPool,
   account_id: &str,
   env: &db::MessageEnvelope,
) -> Result<String> {
   let inputs = thread_inputs(account_id, env);
   threading::resolve_and_record_thrid(pool, &inputs).await
}

#[must_use]
pub fn mailbox_id_for_folder(account_id: &str, folder_name: &str) -> String {
   let mut hash = Sha1::new();
   hash.update(account_id.as_bytes());
   hash.update(b"\0");
   hash.update(folder_name.as_bytes());
   let digest = hash.finalize();
   format!("mb_{}", hex::encode(&digest[..10]))
}

fn display_name_for(imap_name: &str) -> String {
   // Gmail nests things under "[Gmail]/X"; strip that for the display name.
   if let Some(rest) = imap_name.strip_prefix("[Gmail]/") {
      return rest.to_owned();
   }
   imap_name.to_owned()
}

fn sort_order_for_role(role: Option<&str>) -> u32 {
   match role {
      Some("inbox") => 0,
      Some("drafts") => 1,
      Some("sent") => 2,
      Some("archive" | "all") => 3,
      Some("junk") => 4,
      Some("trash") => 5,
      _ => 10,
   }
}

struct DecodedFetch {
   uid:      u32,
   envelope: db::MessageEnvelope,
}

struct FetchParts<'a> {
   uid:           Option<u32>,
   internal_date: Option<DateTime<Utc>>,
   size:          Option<u32>,
   envelope:      Option<&'a Envelope<'a>>,
   header:        Option<&'a [u8]>,
   flags:         Vec<String>,
}

enum FetchDecode {
   Complete(Box<DecodedFetch>),
   /// Invented timestamps would change fallback msgids on each retry.
   Incomplete {
      uid: u32,
   },
   Anonymous,
}

fn decode_fetch(fetch: &Fetch, uidvalidity: u32) -> FetchDecode {
   let flags = fetch
      .flags()
      .map(|flag| format_flag(&flag))
      .collect::<Vec<String>>();
   decode_parts(
      FetchParts {
         uid: fetch.uid,
         internal_date: fetch.internal_date().map(|date| date.with_timezone(&Utc)),
         size: fetch.size,
         envelope: fetch.envelope(),
         header: fetch.header(),
         flags,
      },
      uidvalidity,
   )
}

fn decode_parts(parts: FetchParts<'_>, uidvalidity: u32) -> FetchDecode {
   let FetchParts {
      uid,
      internal_date,
      size,
      envelope,
      header,
      flags,
   } = parts;
   let Some(uid) = uid else {
      return FetchDecode::Anonymous;
   };
   let (Some(internal_date), Some(size), Some(envelope)) = (internal_date, size, envelope) else {
      return FetchDecode::Incomplete { uid };
   };
   let size = u64::from(size);

   let from = addresses(envelope.from.as_deref());
   let to = addresses(envelope.to.as_deref());
   let cc = addresses(envelope.cc.as_deref());
   let bcc = addresses(envelope.bcc.as_deref());
   let reply_to = addresses(envelope.reply_to.as_deref());
   let sender_date = parse_imap_date(envelope.date.as_deref());
   let subject = envelope.subject.as_deref().and_then(decode_envelope_text);
   let msgid_header = envelope
      .message_id
      .as_deref()
      .and_then(|bytes| from_utf8(bytes).ok().map(str::to_owned));
   let irt_header = envelope
      .in_reply_to
      .as_deref()
      .and_then(|bytes| from_utf8(bytes).ok().map(str::to_owned));

   let msgid = normalized_msgid(msgid_header.as_deref(), uidvalidity, uid, internal_date);

   let has_attachment = false; // determined later when we parse full body

   let header_bytes = header.unwrap_or(&[]);
   let parsed_headers = mail_parser::MessageParser::default().parse_headers(header_bytes);
   let references_header = parsed_headers
      .as_ref()
      .and_then(|message| joined_message_ids(message.references()));
   let in_reply_to_header = parsed_headers
      .as_ref()
      .and_then(|message| joined_message_ids(message.in_reply_to()))
      .or(irt_header);

   FetchDecode::Complete(Box::new(DecodedFetch {
      uid,
      envelope: db::MessageEnvelope {
         msgid: msgid.clone(),
         thrid: msgid,
         flags,
         received_at: internal_date,
         sent_at: sender_date,
         size,
         from,
         to,
         cc,
         bcc,
         reply_to,
         subject,
         preview: None,
         has_attachment,
         message_id_header: msgid_header.map(|header| unangle_brackets(&header)),
         in_reply_to_header: in_reply_to_header.map(|header| unangle_brackets(&header)),
         references_header,
      },
   }))
}

/// Keep checkpoints behind skipped UIDs so later syncs retry them.
fn skip_checkpoints(
   skipped: &[u32],
   previous_uidnext: Option<u32>,
   server_uidnext: u32,
) -> (u32, bool) {
   let boundary = previous_uidnext.unwrap_or(0);
   let delta_clamp = skipped.iter().copied().filter(|uid| *uid >= boundary).min();
   let history_clean = !skipped.iter().any(|uid| *uid < boundary);
   (
      delta_clamp.map_or(server_uidnext, |low| low.min(server_uidnext)),
      history_clean,
   )
}

fn joined_message_ids(value: &mail_parser::HeaderValue<'_>) -> Option<String> {
   let ids = value
      .as_text_list()?
      .iter()
      .map(AsRef::as_ref)
      .filter(|id| !id.is_empty())
      .collect::<Vec<_>>();
   (!ids.is_empty()).then(|| ids.join(" "))
}

fn unangle_brackets(raw: &str) -> String {
   raw.trim()
      .trim_matches(|ch| ch == '<' || ch == '>')
      .to_owned()
}

fn format_flag(flag: &async_imap::types::Flag) -> String {
   use async_imap::types::Flag;
   match flag {
      Flag::Seen => "$seen".into(),
      Flag::Answered => "$answered".into(),
      Flag::Flagged => "$flagged".into(),
      Flag::Deleted => "$deleted".into(),
      Flag::Draft => "$draft".into(),
      Flag::Recent => "$recent".into(),
      Flag::MayCreate => "$maycreate".into(),
      Flag::Custom(name) => format!("${}", name.trim_start_matches('$').to_ascii_lowercase()),
   }
}

fn addresses(addrs: Option<&[async_imap::imap_proto::Address]>) -> Option<Vec<EmailAddress>> {
   let addrs = addrs?;
   if addrs.is_empty() {
      return None;
   }
   let out = addrs
      .iter()
      .filter_map(|addr| {
         let mailbox = addr
            .mailbox
            .as_deref()
            .and_then(|bytes| from_utf8(bytes).ok())?;
         let host = addr
            .host
            .as_deref()
            .and_then(|bytes| from_utf8(bytes).ok())?;
         let email = format!("{mailbox}@{host}");
         let name = addr.name.as_deref().and_then(decode_envelope_text);
         Some(EmailAddress { name, email })
      })
      .collect::<Vec<EmailAddress>>();
   if out.is_empty() { None } else { Some(out) }
}

fn decode_envelope_text(value: &[u8]) -> Option<String> {
   let mut terminated = Vec::with_capacity(value.len() + 1);
   terminated.extend_from_slice(value);
   terminated.push(b'\n');
   mail_parser::parsers::MessageStream::new(&terminated)
      .parse_unstructured()
      .as_text()
      .filter(|text| !text.is_empty())
      .map(str::to_owned)
}

fn parse_imap_date(raw: Option<&[u8]>) -> Option<DateTime<Utc>> {
   let raw = raw?;
   let raw = from_utf8(raw).ok()?;
   // ENVELOPE's date field is whatever the Date: header contained; many
   // libraries need lenient RFC 2822 parsing.
   DateTime::parse_from_rfc2822(raw)
      .ok()
      .map(|date| date.with_timezone(&Utc))
      .or_else(|| {
         // Fall back to a permissive parse on a subset of strftime formats.
         let fmts = [
            "%a, %e %b %Y %T %z",
            "%a, %d %b %Y %T %z",
            "%d %b %Y %T %z",
            "%Y-%m-%d %H:%M:%S %z",
         ];
         for fmt in fmts {
            if let Ok(date) = DateTime::parse_from_str(raw.trim(), fmt) {
               return Some(date.with_timezone(&Utc));
            }
         }
         None
      })
}

fn normalized_msgid(
   header: Option<&str>,
   uidvalidity: u32,
   uid: u32,
   internal_date: DateTime<Utc>,
) -> String {
   if let Some(value) = header {
      let trimmed = value
         .trim()
         .trim_matches(|ch| ch == '<' || ch == '>')
         .trim();
      if !trimmed.is_empty() {
         let mut sha = Sha1::new();
         sha.update(trimmed.as_bytes());
         return hex::encode(sha.finalize());
      }
   }
   let mut sha = Sha1::new();
   sha.update(format!(
      "{}|{}|{}",
      uidvalidity,
      uid,
      internal_date.timestamp()
   ));
   format!("fb_{}", hex::encode(&sha.finalize()[..10]))
}

/// Fetch and cache one full RFC 5322 message body.
///
/// # Errors
///
/// Propagates any error from [`fetch_and_cache_bodies`].
pub async fn fetch_and_cache_body(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   msgid: &str,
) -> Result<()> {
   fetch_and_cache_bodies(session, pool, account_id, &[msgid.to_owned()]).await
}

/// Fetch cold bodies in folder-sized IMAP batches.
///
/// # Errors
///
/// Returns [`SyncError`] if a database query fails, if a message has no
/// `message_imap` row to locate it, if the folder's UIDVALIDITY has rotated
/// since it was cached, if the IMAP FETCH omits a requested body, or if a
/// fetched body cannot be parsed and cached.
pub async fn fetch_and_cache_bodies(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   msgids: &[String],
) -> Result<()> {
   const CHUNK_SIZE: usize = 32;

   let wanted = msgids
      .iter()
      .cloned()
      .collect::<BTreeSet<_>>()
      .into_iter()
      .collect::<Vec<_>>();
   if wanted.is_empty() {
      return Ok(());
   }

   let db = db::client(pool).await?;
   let cached = queries::raw_messages::cached_message_ids()
      .bind(&db, &account_id, &wanted)
      .all()
      .await?
      .into_iter()
      .collect::<HashSet<_>>();
   let missing = wanted
      .into_iter()
      .filter(|msgid| !cached.contains(msgid))
      .collect::<Vec<_>>();
   if missing.is_empty() {
      return Ok(());
   }

   let rows = queries::messages::preferred_message_locations()
      .bind(&db, &account_id, &missing)
      .all()
      .await?;
   drop(db);

   let mut locations = HashMap::<String, (String, u32, u32)>::new();
   for row in rows {
      let uid = u32::try_from(row.uid)
         .map_err(|_| SyncError::Other(format!("invalid UID for msgid {}", row.msgid)))?;
      let uidvalidity = u32::try_from(row.uidvalidity)
         .map_err(|_| SyncError::Other(format!("invalid UIDVALIDITY for msgid {}", row.msgid)))?;
      locations.insert(row.msgid, (row.imap_name, uid, uidvalidity));
   }
   if locations.len() != missing.len() {
      let unresolved = missing
         .iter()
         .filter(|msgid| !locations.contains_key(*msgid))
         .cloned()
         .collect::<Vec<_>>();
      return Err(SyncError::Other(format!(
         "no message_imap row for msgids {}",
         unresolved.join(", ")
      )));
   }

   let mut folders = BTreeMap::<(String, u32), Vec<(String, u32)>>::new();
   for (msgid, (imap_name, uid, uidvalidity)) in locations {
      folders
         .entry((imap_name, uidvalidity))
         .or_default()
         .push((msgid, uid));
   }

   for ((imap_name, uidvalidity), entries) in folders {
      let mailbox = session.select(&imap_name).await?;
      let current_uidvalidity = mailbox.uid_validity.unwrap_or(0);
      if current_uidvalidity != uidvalidity {
         return Err(SyncError::UidValidityChanged {
            folder: imap_name,
            was:    uidvalidity,
            now:    current_uidvalidity,
         });
      }

      for chunk in entries.chunks(CHUNK_SIZE) {
         let by_uid = chunk
            .iter()
            .map(|(msgid, uid)| (*uid, msgid.as_str()))
            .collect::<HashMap<_, _>>();
         let uid_set = chunk
            .iter()
            .map(|(_, uid)| uid.to_string())
            .collect::<Vec<_>>()
            .join(",");
         let mut returned = HashSet::<u32>::with_capacity(chunk.len());
         {
            let mut fetches = session.uid_fetch(&uid_set, "(UID BODY.PEEK[])").await?;
            while let Some(fetch) = fetches.next().await {
               let fetch = fetch?;
               let uid = fetch
                  .uid
                  .ok_or_else(|| SyncError::Other("body fetch omitted UID".into()))?;
               let msgid = by_uid.get(&uid).ok_or_else(|| {
                  SyncError::Other(format!("body fetch returned unexpected UID {uid}"))
               })?;
               let raw = fetch
                  .body()
                  .ok_or_else(|| SyncError::Other(format!("no body returned for UID {uid}")))?;
               let raw = raw.to_vec();
               cache_raw_body(pool, account_id, msgid, &raw).await?;
               returned.insert(uid);
            }
         }
         if returned.len() != by_uid.len() {
            return Err(SyncError::Other(format!(
               "body fetch returned {} of {} messages",
               returned.len(),
               by_uid.len()
            )));
         }
      }
   }

   Ok(())
}

/// Cache the newest small messages that do not have parsed bodies yet.
///
/// This keeps the normal conversation-opening path local while avoiding an
/// unbounded mirror of large attachments.
///
/// # Errors
///
/// Returns [`SyncError`] if selecting candidates, fetching them over IMAP, or
/// caching their parsed bodies fails.
pub async fn prefetch_recent_bodies(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   window: usize,
   limit: usize,
) -> Result<usize> {
   let window = i64::try_from(window)
      .map_err(|_| SyncError::Other("body cache window exceeds i64".into()))?;
   let limit = i64::try_from(limit)
      .map_err(|_| SyncError::Other("body prefetch limit exceeds i64".into()))?;
   let msgids = queries::raw_messages::recent_uncached_message_ids()
      .bind(
         &db::client(pool).await?,
         &account_id,
         &window,
         &MAX_PREFETCH_MESSAGE_BYTES,
         &limit,
      )
      .all()
      .await?;
   if msgids.is_empty() {
      return Ok(0);
   }
   let count = msgids.len();
   fetch_and_cache_bodies(session, pool, account_id, &msgids).await?;
   Ok(count)
}

async fn cache_raw_body(pool: &PgPool, account_id: &str, msgid: &str, raw: &[u8]) -> Result<()> {
   let parsed = mail_parser::MessageParser::default()
      .parse(raw)
      .ok_or_else(|| SyncError::Other("mail-parser could not parse body".into()))?;
   let projected = project_to_jmap_with_msgid(&parsed, Some(msgid));

   // Persist body + cache attachment presence. Note: `has_attachment` is
   // updated on the messages row too so Email/query's hasAttachment filter
   // reflects reality after a body fetch.
   let has_attachment = !projected.attachments.is_empty();
   let headers_json = serde_json::to_string(&serde_json::json!({
       "headers": projected.headers,
       "textBody": projected.text_body,
       "htmlBody": projected.html_body,
   }))
   .unwrap_or_default();
   let body_values_json =
      serde_json::to_string(&projected.body_values).unwrap_or_else(|_| "{}".into());
   let attachments_json =
      serde_json::to_string(&projected.attachments).unwrap_or_else(|_| "[]".into());
   let preview = projected.preview.unwrap_or_default();
   queries::raw_messages::upsert_raw_message()
      .bind(
         &db::client(pool).await?,
         &account_id,
         &msgid,
         &headers_json.as_str(),
         &body_values_json.as_str(),
         &attachments_json.as_str(),
         &raw,
      )
      .await?;

   // Preview + has_attachment backfill. Only touch the row if something
   // changed so we don't fabricate Email/changes noise.
   let cur = queries::messages::message_body_metadata()
      .bind(&db::client(pool).await?, &account_id, &msgid)
      .opt()
      .await?
      .ok_or_else(|| SyncError::Other(format!("no messages row for {msgid} after body fetch")))?;
   let preview_changed = cur.preview.as_deref() != Some(preview.as_str());
   let attachment_changed = (cur.has_attachment != 0) != has_attachment;
   if preview_changed || attachment_changed {
      let new_modseq = db::bump_modseq(pool, account_id, db::StateKind::Email).await?;
      queries::messages::update_message_body_cache()
         .bind(
            &db::client(pool).await?,
            &preview.as_str(),
            &i64::from(has_attachment),
            &(new_modseq as i64),
            &account_id,
            &msgid,
         )
         .await?;
   }

   Ok(())
}

pub(crate) async fn repair_cached_body_metadata(pool: &PgPool, account_id: &str) -> Result<u64> {
   let mut updates = Vec::<(String, String, bool, Option<String>)>::new();
   {
      let db = db::client(pool).await?;
      let rows = queries::raw_messages::cached_metadata_repair_candidates()
         .bind(&db, &account_id)
         .iter()
         .await?;
      futures::pin_mut!(rows);
      while let Some(row) = rows.try_next().await? {
         let Some(message) = mail_parser::MessageParser::default().parse(&row.raw_rfc822) else {
            warn!(
               account_id,
               msgid = row.msgid,
               "cached message no longer parses"
            );
            continue;
         };
         let projected = project_to_jmap_with_msgid(&message, Some(&row.msgid));
         let subject = if row
            .subject
            .as_deref()
            .is_some_and(|subject| subject.contains("=?"))
         {
            message.subject().map(str::to_owned).or(row.subject)
         } else {
            row.subject
         };
         updates.push((
            row.msgid,
            projected.preview.unwrap_or_default(),
            !projected.attachments.is_empty(),
            subject,
         ));
      }
   }
   if updates.is_empty() {
      return Ok(0);
   }

   let mut db = db::client(pool).await?;
   let transaction = db.transaction().await?;
   let modseq = queries::state::bump_email_modseq()
      .bind(&transaction, &account_id)
      .one()
      .await?;
   for (msgid, preview, has_attachment, subject) in &updates {
      queries::messages::repair_message_body_metadata()
         .bind(
            &transaction,
            &subject.as_deref(),
            &preview.as_str(),
            &i64::from(*has_attachment),
            &modseq,
            &account_id,
            &msgid.as_str(),
         )
         .await?;
   }
   transaction.commit().await?;
   Ok(updates.len() as u64)
}

/// One message's requested flag delta. The account-task collects a batch of
/// these and hands them to [`store_flags_batch`].
#[derive(Debug, Clone)]
pub struct StoreFlagsOp {
   pub msgid:  String,
   pub add:    Vec<String>,
   pub remove: Vec<String>,
}

/// Apply a batch of per-message flag deltas.
///
/// Group by `(folder, add, remove)` and emit one `UID STORE {uid-set}
/// +FLAGS.SILENT (...)` per group, plus a parallel `-FLAGS.SILENT` when the
/// bucket had removes. Returns one result per input op in the same order the
/// caller supplied.
///
/// Failure policy: one bucket's STORE failing marks every op in that bucket
/// as failed but does not abort the whole batch — other folders still apply.
/// Every op that makes it through the STOREs has its DB row updated inside a
/// single transaction so a crash mid-commit leaves both the IMAP and the DB
/// consistent (or leaves the DB stale and sync rebuilds it from the server
/// — never the other way around).
pub async fn store_flags_batch(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   ops: &[StoreFlagsOp],
   traits: SessionTraits,
   selected: Option<(&str, u32)>,
) -> Vec<Result<()>> {
   type BucketKey = (String, Vec<String>, Vec<String>);
   if ops.is_empty() {
      return Vec::new();
   }

   // Generic IMAP flags are per-copy, so update every folder or
   // reconciliation restores stale flags. Gmail flags are message-global and
   // one STORE in the preferred location covers every label.
   let mut resolved = Vec::<(usize, String, u32, u32)>::new();
   let mut results = iter::repeat_with(|| None)
      .take(ops.len())
      .collect::<Vec<Option<Result<()>>>>();
   if traits.gmail {
      let msgids = ops.iter().map(|op| op.msgid.clone()).collect::<Vec<_>>();
      let rows = match db::client(pool).await {
         Ok(conn) => {
            queries::messages::preferred_message_locations()
               .bind(&conn, &account_id, &msgids)
               .all()
               .await
               .map_err(SyncError::from)
         },
         Err(err) => Err(err),
      };
      match rows {
         Ok(rows) => {
            let by_msgid = rows
               .into_iter()
               .map(|row| (row.msgid.clone(), row))
               .collect::<HashMap<_, _>>();
            for (i, op) in ops.iter().enumerate() {
               match by_msgid.get(&op.msgid) {
                  Some(row) => {
                     resolved.push((
                        i,
                        row.imap_name.clone(),
                        row.uid as u32,
                        row.uidvalidity as u32,
                     ));
                  },
                  None => {
                     results[i] = Some(Err(SyncError::Other(format!(
                        "no message_imap row for msgid {}",
                        op.msgid
                     ))));
                  },
               }
            }
         },
         Err(err) => {
            let shared = err.to_string();
            for slot in &mut results {
               slot.get_or_insert_with(|| Err(SyncError::Other(shared.clone())));
            }
         },
      }
   } else {
      for (i, op) in ops.iter().enumerate() {
         let rows = match db::client(pool).await {
            Ok(conn) => {
               queries::messages::message_locations()
                  .bind(&conn, &account_id, &op.msgid.as_str())
                  .all()
                  .await
                  .map_err(SyncError::from)
            },
            Err(err) => Err(err),
         };
         match rows {
            Ok(rows) if rows.is_empty() => {
               results[i] = Some(Err(SyncError::Other(format!(
                  "no message_imap row for msgid {}",
                  op.msgid
               ))));
            },
            Ok(rows) => {
               for row in rows {
                  resolved.push((i, row.imap_name, row.uid as u32, row.uidvalidity as u32));
               }
            },
            Err(err) => results[i] = Some(Err(SyncError::Other(err.to_string()))),
         }
      }
   }

   // Bucket by (folder, add-set, remove-set). One op spans multiple
   // buckets when its msgid is in multiple folders; we emit one STORE
   // per bucket and only mark the op `Ok` if every bucket succeeds.
   let mut buckets = BTreeMap::<BucketKey, Vec<(usize, u32, u32, String)>>::new();
   let mut bucket_count_per_op = vec![0_usize; ops.len()];
   for (i, folder, uid, uidvalidity) in resolved {
      let mut add = ops[i].add.clone();
      let mut remove = ops[i].remove.clone();
      add.sort();
      remove.sort();
      let key = (folder.clone(), add, remove);
      buckets
         .entry(key)
         .or_default()
         .push((i, uid, uidvalidity, ops[i].msgid.clone()));
      bucket_count_per_op[i] += 1;
   }
   // First-failure-wins: if any bucket for an op fails, the op is Err.
   // An op without any pre-recorded resolution error is Ok at the end iff
   // every one of its buckets succeeded.
   let mut op_first_error = iter::repeat_with(|| None)
      .take(ops.len())
      .collect::<Vec<Option<SyncError>>>();
   let mut updated_ops = BTreeSet::<usize>::new();
   let mut live_selection = selected.map(|(name, uv)| (name.to_owned(), uv));

   for ((folder, add, remove), entries) in buckets {
      // Verify UIDVALIDITY by SELECTing the folder once.
      // Helper: record a per-op failure if not already failed. Mutating
      // `op_first_error` (rather than `results` directly) lets us emit
      // the final per-op verdict only after every bucket has been
      // attempted, so an op spanning two folders is `Ok` only when both
      // succeeded.
      let mark_failed =
         |op_first_error: &mut [Option<SyncError>], idxs: &[usize], err: &SyncError| {
            for i in idxs {
               if op_first_error[*i].is_none() {
                  // Clone the error message rather than the variant so we
                  // don't need SyncError: Clone.
                  op_first_error[*i] = Some(SyncError::Other(err.to_string()));
               }
            }
         };

      // Skip the SELECT when the session already sits on this folder, it
      // costs whole seconds on large Gmail folders.
      let live_uv = match &live_selection {
         Some((name, uv)) if *name == folder => *uv,
         _ => {
            match session.select(&folder).await {
               Ok(mailbox) => {
                  let uv = mailbox.uid_validity.unwrap_or(0);
                  live_selection = Some((folder.clone(), uv));
                  uv
               },
               Err(source) => {
                  let err = SyncError::Other(format!("SELECT {folder}: {source}"));
                  let idxs = entries.iter().map(|(i, ..)| *i).collect::<Vec<usize>>();
                  mark_failed(&mut op_first_error, &idxs, &err);
                  continue;
               },
            }
         },
      };
      // Per-bucket UIDVALIDITY check: if any entry in the bucket has a
      // stale uidvalidity, fail just those. Mismatches here are rare
      // unless the server reset the folder; surface them loudly.
      let fresh = entries
         .into_iter()
         .filter_map(|(i, uid, uv, msgid)| {
            if uv == live_uv {
               Some((i, uid, msgid))
            } else {
               let err = SyncError::UidValidityChanged {
                  folder: folder.clone(),
                  was:    uv,
                  now:    live_uv,
               };
               mark_failed(&mut op_first_error, &[i], &err);
               None
            }
         })
         .collect::<Vec<(usize, u32, String)>>();
      if fresh.is_empty() {
         continue;
      }

      let uid_set = fresh
         .iter()
         .map(|(_, uid, _)| uid.to_string())
         .collect::<Vec<_>>()
         .join(",");

      if !add.is_empty() {
         let cmd = format!("+FLAGS.SILENT ({})", add.join(" "));
         if let Err(source) = drain_store(session, &uid_set, &cmd).await {
            let err = SyncError::Other(format!("STORE +{add:?}: {source}"));
            let idxs = fresh.iter().map(|(i, ..)| *i).collect::<Vec<usize>>();
            mark_failed(&mut op_first_error, &idxs, &err);
            continue;
         }
      }
      if !remove.is_empty() {
         let cmd = format!("-FLAGS.SILENT ({})", remove.join(" "));
         if let Err(source) = drain_store(session, &uid_set, &cmd).await {
            let err = SyncError::Other(format!("STORE -{remove:?}: {source}"));
            let idxs = fresh.iter().map(|(i, ..)| *i).collect::<Vec<usize>>();
            mark_failed(&mut op_first_error, &idxs, &err);
            continue;
         }
      }

      // DB update — one row per successful msgid. A shared modseq bump
      // keeps each touched row's Email/changes entry correct.
      let new_modseq = match db::bump_modseq(pool, account_id, db::StateKind::Email).await {
         Ok(n) => n,
         Err(source) => {
            let err = SyncError::Other(source.to_string());
            let idxs = fresh.iter().map(|(i, ..)| *i).collect::<Vec<usize>>();
            mark_failed(&mut op_first_error, &idxs, &err);
            continue;
         },
      };
      for (i, _uid, msgid) in &fresh {
         match db_apply_flag_delta(pool, account_id, msgid, &add, &remove, new_modseq).await {
            Ok(()) => {
               updated_ops.insert(*i);
            },
            Err(err) => {
               mark_failed(
                  &mut op_first_error,
                  &[*i],
                  &SyncError::Other(err.to_string()),
               );
            },
         }
      }
   }

   if !updated_ops.is_empty()
      && let Err(error) = db::recompute_mailbox_counts(pool, account_id).await
   {
      for i in updated_ops {
         op_first_error[i].get_or_insert_with(|| SyncError::Other(error.to_string()));
      }
   }

   // Finalize: an op is Ok if (a) it had at least one bucket and (b)
   // none of its buckets failed. Resolution-time errors stored in
   // `results` take precedence (they fired before any bucket ran).
   for (i, slot) in results.iter_mut().enumerate() {
      if slot.is_some() {
         continue;
      }
      if let Some(err) = op_first_error[i].take() {
         *slot = Some(Err(err));
      } else {
         *slot = Some(Ok(()));
      }
   }

   results
      .into_iter()
      .map(|result| result.unwrap_or_else(|| Err(SyncError::Other("no result produced".into()))))
      .collect()
}

/// Run a STORE command and drain the response stream. async-imap returns a
/// stream of untagged FETCH responses on STORE — we don't care about the
/// contents (we're using `.SILENT`), we just need to exhaust the stream so
/// the next IMAP command doesn't see leftover bytes.
async fn drain_store(
   session: &mut ImapSession,
   uid_set: &str,
   cmd: &str,
) -> StdResult<(), ImapError> {
   let mut stream = session.uid_store(uid_set, cmd).await?;
   while let Some(resp) = stream.next().await {
      let _ = resp?;
   }
   Ok(())
}

async fn db_apply_flag_delta(
   pool: &PgPool,
   account_id: &str,
   msgid: &str,
   add: &[String],
   remove: &[String],
   new_modseq: u64,
) -> Result<()> {
   // Read current flags, apply delta in-memory, write back. Using a
   // read-modify-write makes the JMAP-side keyword set authoritative over
   // the IMAP-side flag set — matches what Email/get reports.
   let flags_json = queries::messages::message_flags_json()
      .bind(&db::client(pool).await?, &account_id, &msgid)
      .opt()
      .await?;
   let mut flags = flags_json
      .as_deref()
      .map(|json| serde_json::from_str::<Vec<String>>(json).unwrap_or_default())
      .unwrap_or_default();
   for removed in remove {
      let keyword = imap_flag_to_keyword(removed);
      flags.retain(|flag| flag != &keyword);
   }
   for added in add {
      let keyword = imap_flag_to_keyword(added);
      if !flags.contains(&keyword) {
         flags.push(keyword);
      }
   }
   let flags_json = serde_json::to_string(&flags).unwrap_or_else(|_| "[]".into());
   queries::messages::set_message_flags()
      .bind(
         &db::client(pool).await?,
         &flags_json.as_str(),
         &(new_modseq as i64),
         &account_id,
         &msgid,
      )
      .await?;
   Ok(())
}

/// IMAP `\Seen` → JMAP `$seen`.
///
/// Mirrors `format_flag`'s wire → keyword mapping (which is only used for the
/// inbound direction today). System flags are dollar-prefixed keywords; custom
/// flags pass through unchanged modulo a lowercase normalize.
#[must_use]
pub fn imap_flag_to_keyword(imap: &str) -> String {
   match imap {
      "\\Seen" | "\\seen" => "$seen".into(),
      "\\Flagged" | "\\flagged" => "$flagged".into(),
      "\\Answered" | "\\answered" => "$answered".into(),
      "\\Deleted" | "\\deleted" => "$deleted".into(),
      "\\Draft" | "\\draft" => "$draft".into(),
      other => {
         // `$forwarded`, `$junk`, etc. are IMAP-wire keywords already.
         let trimmed = other.trim_start_matches('$').to_ascii_lowercase();
         format!("${trimmed}")
      },
   }
}

/// Apply a mailbox-membership delta to a message over IMAP.
///
/// Prefers `UID MOVE` (RFC 6851) when the server advertises it: one RTT
/// and atomic under concurrent writers. Falls back to `UID COPY` + `UID
/// STORE +FLAGS (\Deleted)` + `UID EXPUNGE` / `UID EXPUNGE`-less EXPUNGE
/// on servers without MOVE. The DB row's `message_mailboxes` set is
/// updated in the same transaction as the modseq bump so `Email/changes`
/// reflects the new membership.
///
/// `add` and `remove` use JMAP mailbox ids. An empty delta is a no-op.
///
/// # Errors
///
/// Returns [`SyncError`] if a capability probe or database lookup fails, if
/// `msgid` has no IMAP membership, if any `add`/`remove` mailbox id does not
/// resolve to a folder, if a folder's UIDVALIDITY has rotated, if a remove is
/// required but the server lacks UIDPLUS, or if any IMAP MOVE/COPY/STORE/
/// EXPUNGE command fails.
pub async fn mutate_mailboxes(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   msgid: &str,
   add: &[String],
   remove: &[String],
   traits: SessionTraits,
) -> Result<()> {
   if add.is_empty() && remove.is_empty() {
      return Ok(());
   }

   let locs = queries::messages::message_locations()
      .bind(&db::client(pool).await?, &account_id, &msgid)
      .all()
      .await?
      .into_iter()
      .map(|row| {
         (
            row.folder_id,
            row.imap_name,
            row.uid as u32,
            row.uidvalidity as u32,
         )
      })
      .collect::<Vec<(i64, String, u32, u32)>>();
   if locs.is_empty() {
      return Err(SyncError::Other(format!(
         "msgid {msgid} has no IMAP membership to mutate"
      )));
   }

   let add_folders = resolve_mailboxes(pool, account_id, add).await?;
   let remove_folders = resolve_mailboxes(pool, account_id, remove).await?;

   let current = locs
      .iter()
      .map(|(_, name, uid, uv)| (name.clone(), (*uid, *uv)))
      .collect::<HashMap<String, (u32, u32)>>();

   // Drop redundant membership operations before touching IMAP.
   let net_remove = net_memberships(&remove_folders, &current, true);
   let net_adds = net_memberships(&add_folders, &current, false);

   // Strategy:
   //   * For each net-remove with a matching net-add, use UID MOVE (when
   //     advertised) — one RTT and atomic.
   //   * For any remaining net-removes, STORE \Deleted + UID EXPUNGE. UID EXPUNGE
   //     requires UIDPLUS so we don't clobber other clients' pending \Deleted on a
   //     folder-wide EXPUNGE.
   //   * For any remaining net-adds, UID COPY from any current location.
   let mut next_add = 0_usize;
   let mut label_stripping_move = false;
   for rm in &net_remove {
      let (rm_uid, rm_uv) = current[&rm.imap_name];
      if label_stripping_move {
         // The Gmail move into Trash or Spam already stripped every other
         // label server-side.
         continue;
      }
      if traits.has_move && next_add < net_adds.len() {
         let dst = net_adds[next_add];
         next_add += 1;
         let mbox = session.select(&rm.imap_name).await?;
         let live_uv = mbox.uid_validity.unwrap_or(0);
         if live_uv != rm_uv {
            return Err(SyncError::UidValidityChanged {
               folder: rm.imap_name.clone(),
               was:    rm_uv,
               now:    live_uv,
            });
         }
         session.uid_mv(rm_uid.to_string(), &dst.imap_name).await?;
         if traits.gmail && matches!(dst.role.as_deref(), Some("trash" | "junk")) {
            label_stripping_move = true;
         }
         continue;
      }
      // Fall-through: explicit STORE \Deleted + UID EXPUNGE.
      if !traits.uidplus {
         return Err(SyncError::Other(format!(
            "remove from {} requires UIDPLUS to scope EXPUNGE; server does not advertise it",
            rm.imap_name,
         )));
      }
      let mbox = session.select(&rm.imap_name).await?;
      if mbox.uid_validity.unwrap_or(0) != rm_uv {
         return Err(SyncError::UidValidityChanged {
            folder: rm.imap_name.clone(),
            was:    rm_uv,
            now:    mbox.uid_validity.unwrap_or(0),
         });
      }
      {
         let mut stream = session
            .uid_store(rm_uid.to_string(), r"+FLAGS.SILENT (\Deleted)")
            .await?;
         while let Some(resp) = stream.next().await {
            let _ = resp?;
         }
      }
      let stream = session.uid_expunge(rm_uid.to_string()).await?;
      futures::pin_mut!(stream);
      while let Some(resp) = stream.next().await {
         let _ = resp?;
      }
   }

   // Any net-adds not paired with a remove: COPY from any current
   // location. Prefer one we haven't already removed via MOVE.
   let copy_source = locs
      .iter()
      .find(|(_, name, ..)| !net_remove.iter().any(|rm| &rm.imap_name == name))
      .or_else(|| locs.first());
   if let Some((_, src_name, src_uid, src_uv)) = copy_source {
      for dst in &net_adds[next_add..] {
         let mbox = session.select(src_name).await?;
         if mbox.uid_validity.unwrap_or(0) != *src_uv {
            return Err(SyncError::UidValidityChanged {
               folder: src_name.clone(),
               was:    *src_uv,
               now:    mbox.uid_validity.unwrap_or(0),
            });
         }
         session
            .uid_copy(src_uid.to_string(), &dst.imap_name)
            .await?;
      }
   }

   // We know exactly which placements moved, no need to sweep the sources.
   let removed = net_remove
      .iter()
      .map(|rm| (rm.folder_id, rm.mailbox_id.clone()))
      .collect::<Vec<(i64, String)>>();
   let added = net_adds
      .iter()
      .map(|dst| dst.mailbox_id.clone())
      .collect::<Vec<String>>();
   apply_move_bookkeeping(pool, account_id, msgid, &removed, &added).await?;

   // Only the destinations need a reconcile, to learn the new uids. Real
   // LIST entries so the mailbox role is not clobbered.
   if !net_adds.is_empty() {
      let opts = interactive_options(traits);
      let listed = imap::list_folders(session).await?;
      for dst in &net_adds {
         let Some(folder) = listed.iter().find(|folder| folder.name == dst.imap_name) else {
            continue;
         };
         let _ = reconcile_folder(session, pool, account_id, folder, opts).await?;
      }
   }

   db::recompute_mailbox_counts(pool, account_id).await?;
   Ok(())
}

fn net_memberships<'a>(
   resolved: &'a [ResolvedMailbox],
   current: &HashMap<String, (u32, u32)>,
   want_member: bool,
) -> Vec<&'a ResolvedMailbox> {
   resolved
      .iter()
      .filter(|folder| current.contains_key(&folder.imap_name) == want_member)
      .collect()
}

async fn apply_move_bookkeeping(
   pool: &PgPool,
   account_id: &str,
   msgid: &str,
   removed: &[(i64, String)],
   added_mailboxes: &[String],
) -> Result<()> {
   if removed.is_empty() && added_mailboxes.is_empty() {
      return Ok(());
   }
   let mut conn = db::client(pool).await?;
   let tx = conn.transaction().await?;
   for mailbox_id in added_mailboxes {
      queries::messages::add_message_mailbox()
         .bind(&tx, &account_id, &msgid, &mailbox_id.as_str())
         .await?;
   }
   for (folder_id, mailbox_id) in removed {
      queries::messages::delete_message_imap_placement()
         .bind(&tx, &account_id, &msgid, folder_id)
         .await?;
      queries::messages::remove_message_mailbox()
         .bind(&tx, &account_id, &msgid, &mailbox_id.as_str())
         .await?;
   }
   let new_modseq = queries::state::bump_email_modseq()
      .bind(&tx, &account_id)
      .one()
      .await?;
   queries::messages::set_message_modseq()
      .bind(&tx, &new_modseq, &account_id, &msgid)
      .await?;
   tx.commit().await?;
   Ok(())
}

struct ResolvedMailbox {
   folder_id:  i64,
   imap_name:  String,
   mailbox_id: String,
   role:       Option<String>,
}

/// Look up folder ids + imap names for a list of JMAP mailbox ids.
/// Returns `Err(SyncError::Other(...))` listing unresolved ids if any are
/// missing — the caller must surface this as `invalidProperties` so the
/// client doesn't get a silent success for a bad mailbox reference.
async fn resolve_mailboxes(
   pool: &PgPool,
   account_id: &str,
   mailbox_ids: &[String],
) -> Result<Vec<ResolvedMailbox>> {
   if mailbox_ids.is_empty() {
      return Ok(Vec::new());
   }
   let ids = mailbox_ids.to_vec();
   let rows = queries::mailboxes::resolve_mailbox_folders()
      .bind(&db::client(pool).await?, &account_id, &ids)
      .all()
      .await?;
   let resolved = rows
      .iter()
      .map(|row| row.mailbox_id.as_str())
      .collect::<HashSet<&str>>();
   let missing = mailbox_ids
      .iter()
      .map(String::as_str)
      .filter(|id| !resolved.contains(id))
      .collect::<Vec<&str>>();
   if !missing.is_empty() {
      return Err(SyncError::Other(format!(
         "unknown mailbox ids: {}",
         missing.join(", ")
      )));
   }
   Ok(rows
      .into_iter()
      .map(|row| {
         ResolvedMailbox {
            folder_id:  row.id,
            imap_name:  row.imap_name,
            mailbox_id: row.mailbox_id,
            role:       row.role,
         }
      })
      .collect())
}

/// Validate a client-supplied mailbox name before it becomes part of an IMAP
/// CREATE/RENAME argument.
///
/// Rejects control characters and IMAP list wildcards; the quoted-string
/// encoding downstream can't protect against CRLF injection, so refuse those
/// outright.
pub fn is_valid_folder_name(name: &str) -> bool {
   !name.is_empty()
      && name.len() <= 255
      && !name.chars().any(char::is_control)
      && !name.contains(['%', '*', '"'])
      && name != "."
      && name != ".."
}

/// Server delimiter as observed on a live folder, defaulting to '/'.
fn delimiter_of(folders: &[ImapFolder], name: &str) -> char {
   folders
      .iter()
      .find(|folder| folder.name == name)
      .map(|folder| folder.delimiter)
      .filter(|delim| *delim != '\0')
      .unwrap_or('/')
}

/// Mailbox/set create — IMAP CREATE + ingest of the new (empty) folder.
/// Returns the JMAP mailbox id minted for it.
///
/// `parent_mailbox_id` resolves to an existing folder whose IMAP path (plus
/// the server delimiter) prefixes the new name. The mailbox row itself stays
/// flat (`parent_id` NULL) like every synced folder — hierarchy is expressed
/// in the display name.
///
/// # Errors
///
/// Returns [`SyncError`] if `name` is invalid, if the folder list (or resolved
/// parent mailbox) cannot be fetched, if a mailbox with that path already
/// exists on the server, if the IMAP `CREATE` fails, or if the freshly created
/// folder does not surface in a follow-up `LIST`.
pub async fn create_folder(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   name: &str,
   parent_mailbox_id: Option<&str>,
   traits: SessionTraits,
) -> Result<String> {
   if !is_valid_folder_name(name) {
      return Err(SyncError::Other(format!("invalid mailbox name {name:?}")));
   }
   let live = imap::list_folders(session).await?;
   let imap_name = match parent_mailbox_id {
      Some(pid) => {
         let parent = resolve_mailboxes(pool, account_id, &[pid.to_owned()]).await?;
         let parent_name = &parent
            .first()
            .ok_or_else(|| SyncError::Other(format!("unknown parent mailbox {pid}")))?
            .imap_name;
         format!("{parent_name}{}{name}", delimiter_of(&live, parent_name))
      },
      None => name.to_owned(),
   };
   if live.iter().any(|folder| folder.name == imap_name) {
      return Err(SyncError::Other(format!(
         "mailbox {imap_name:?} already exists on the server"
      )));
   }

   session.create(&imap_name).await?;

   // Re-LIST so we pick up the server's canonical spelling + flags, then run
   // the ordinary folder ingest to mint the folder + mailbox rows.
   let folders = imap::list_folders(session).await?;
   let folder = folders
      .iter()
      .find(|folder| folder.name == imap_name)
      .ok_or_else(|| {
         SyncError::Other(format!(
            "created {imap_name:?} but the server does not list it"
         ))
      })?;
   let opts = interactive_options(traits);
   sync_folder(session, pool, account_id, folder, opts).await?;

   let folder = queries::folders::folder_by_name()
      .bind(&db::client(pool).await?, &account_id, &imap_name.as_str())
      .one()
      .await?;
   Ok(folder.mailbox_id)
}

/// Mailbox/set update (name) — IMAP RENAME keeping the JMAP mailbox id.
///
/// The new display name is interpreted the same way sync derives display
/// names: it's the full IMAP path with any `[Gmail]/` prefix stripped, so
/// renaming a nested folder means passing the full new path. Child folders
/// are renamed implicitly by the server (RFC 3501 §6.3.5); we mirror that in
/// the cache so their rows keep their ids too.
///
/// # Errors
///
/// Returns [`SyncError`] if `new_name` is invalid, if `mailbox_id` does not
/// resolve, if the folder list cannot be fetched, if the target name already
/// exists on the server, if the IMAP `RENAME` fails, or if any of the cache
/// updates for the folder or its renamed children fail.
pub async fn rename_folder(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   mailbox_id: &str,
   new_name: &str,
) -> Result<()> {
   if !is_valid_folder_name(new_name) {
      return Err(SyncError::Other(format!(
         "invalid mailbox name {new_name:?}"
      )));
   }
   let resolved = resolve_mailboxes(pool, account_id, &[mailbox_id.to_owned()]).await?;
   let resolved = resolved
      .into_iter()
      .next()
      .ok_or_else(|| SyncError::Other(format!("unknown mailbox {mailbox_id}")))?;
   let (folder_id, old_imap) = (resolved.folder_id, resolved.imap_name);

   let new_imap = if old_imap.starts_with("[Gmail]/") {
      format!("[Gmail]/{new_name}")
   } else {
      new_name.to_owned()
   };
   if new_imap == old_imap {
      return Ok(());
   }

   let live = imap::list_folders(session).await?;
   let delim = delimiter_of(&live, &old_imap);
   if live.iter().any(|folder| folder.name == new_imap) {
      return Err(SyncError::Other(format!(
         "mailbox {new_imap:?} already exists on the server"
      )));
   }

   session.rename(&old_imap, &new_imap).await?;

   queries::folders::rename_folder()
      .bind(
         &db::client(pool).await?,
         &new_imap.as_str(),
         &folder_id,
         &account_id,
      )
      .await?;
   let modseq = db::bump_modseq(pool, account_id, db::StateKind::Mailbox).await?;
   queries::mailboxes::set_mailbox_name()
      .bind(
         &db::client(pool).await?,
         &display_name_for(&new_imap).as_str(),
         &(modseq as i64),
         &mailbox_id,
         &account_id,
      )
      .await?;

   // Children were renamed server-side with the parent; rewrite their cached
   // paths so their folder rows (and mailbox ids) survive instead of being
   // dropped + re-minted by the next reconcile.
   let old_prefix = format!("{old_imap}{delim}");
   let children = queries::folders::folder_children()
      .bind(&db::client(pool).await?, &account_id, &old_prefix.as_str())
      .all()
      .await?;
   for child in children {
      let child_new = format!("{new_imap}{delim}{}", &child.imap_name[old_prefix.len()..]);
      queries::folders::rename_folder()
         .bind(
            &db::client(pool).await?,
            &child_new.as_str(),
            &child.id,
            &account_id,
         )
         .await?;
      let modseq = db::bump_modseq(pool, account_id, db::StateKind::Mailbox).await?;
      queries::mailboxes::set_mailbox_name()
         .bind(
            &db::client(pool).await?,
            &display_name_for(&child_new).as_str(),
            &(modseq as i64),
            &child.mailbox_id.as_str(),
            &account_id,
         )
         .await?;
   }
   Ok(())
}

/// Mailbox/set destroy — IMAP DELETE + cache cleanup.
///
/// The caller is responsible for the `mailboxHasEmail` policy check
/// (`onDestroyRemoveEmails`); by the time we're called the decision to drop
/// the folder's contents has been made. Refuses folders with children — IMAP
/// servers differ on whether DELETE cascades, so force the client to be
/// explicit bottom-up.
///
/// # Errors
///
/// Returns [`SyncError`] if `mailbox_id` does not resolve, if the folder list
/// cannot be fetched, if the folder still has child folders
/// ([`SyncError::MailboxHasChild`]), if the IMAP `DELETE` fails, or if the
/// cache purge and mailbox-row cleanup fail.
pub async fn delete_folder(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   mailbox_id: &str,
) -> Result<()> {
   let resolved = resolve_mailboxes(pool, account_id, &[mailbox_id.to_owned()]).await?;
   let resolved = resolved
      .into_iter()
      .next()
      .ok_or_else(|| SyncError::Other(format!("unknown mailbox {mailbox_id}")))?;
   let (folder_id, imap_name) = (resolved.folder_id, resolved.imap_name);

   let live = imap::list_folders(session).await?;
   let delim = delimiter_of(&live, &imap_name);
   let prefix = format!("{imap_name}{delim}");
   if live.iter().any(|folder| folder.name.starts_with(&prefix)) {
      return Err(SyncError::MailboxHasChild(imap_name));
   }

   session.delete(&imap_name).await?;

   // Reuse the UIDVALIDITY purge: it already handles the orphan-vs-survivor
   // split for messages that also live in other folders and bumps the email
   // modseq for membership shrinks.
   db::purge_folder_for_uidvalidity_change(pool, account_id, folder_id, mailbox_id).await?;
   queries::folders::delete_folder()
      .bind(&db::client(pool).await?, &folder_id, &account_id)
      .await?;
   db::bump_modseq(pool, account_id, db::StateKind::Mailbox).await?;
   queries::mailboxes::delete_mailbox()
      .bind(&db::client(pool).await?, &mailbox_id, &account_id)
      .await?;
   db::recompute_mailbox_counts(pool, account_id).await?;
   Ok(())
}

/// Remove a message from every folder with UID-scoped expunges.
///
/// # Errors
///
/// Returns [`SyncError`] if the message cannot be located, if the account's
/// server lacks UIDPLUS (folder-wide EXPUNGE is refused), if a folder's
/// UIDVALIDITY has rotated, if any IMAP STORE/EXPUNGE fails, or if the DB row
/// cannot be deleted.
pub async fn destroy_message(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   msgid: &str,
   traits: SessionTraits,
) -> Result<()> {
   let locs = queries::messages::message_locations()
      .bind(&db::client(pool).await?, &account_id, &msgid)
      .all()
      .await?
      .into_iter()
      .map(|row| (row.imap_name, row.uid as u32, row.uidvalidity as u32))
      .collect::<Vec<(String, u32, u32)>>();
   if locs.is_empty() {
      return Err(SyncError::Other(format!("unknown msgid {msgid}")));
   }

   if !traits.uidplus {
      // Without UIDPLUS, EXPUNGE is folder-wide — we'd clobber any
      // other client's pending \Deleted markers in the same folder.
      // Refuse rather than silently nuke unrelated mail.
      return Err(SyncError::Other(
         "Email/set destroy requires the IMAP UIDPLUS extension on this account; folder-wide \
          EXPUNGE would risk clobbering unrelated \\Deleted messages"
            .into(),
      ));
   }
   for (folder, uid, uv) in &locs {
      let mbox = session.select(folder).await?;
      if mbox.uid_validity.unwrap_or(0) != *uv {
         return Err(SyncError::UidValidityChanged {
            folder: folder.clone(),
            was:    *uv,
            now:    mbox.uid_validity.unwrap_or(0),
         });
      }
      {
         let mut stream = session
            .uid_store(uid.to_string(), r"+FLAGS.SILENT (\Deleted)")
            .await?;
         while let Some(resp) = stream.next().await {
            let _ = resp?;
         }
      }
      let stream = session.uid_expunge(uid.to_string()).await?;
      futures::pin_mut!(stream);
      while let Some(resp) = stream.next().await {
         let _ = resp?;
      }
   }

   // Proactively drop the DB row so the caller's follow-up `Email/get`
   // returns `notFound` rather than stale data. CASCADE handles
   // message_imap + message_mailboxes + raw_messages.
   let _ = db::bump_modseq(pool, account_id, db::StateKind::Email).await?;
   queries::messages::delete_message()
      .bind(&db::client(pool).await?, &account_id, &msgid)
      .await?;
   db::recompute_mailbox_counts(pool, account_id).await?;
   Ok(())
}

pub struct ImportSpec<'a> {
   pub blob_id:          &'a str,
   pub mailbox_id:       &'a str,
   pub flags:            &'a [String],
   pub received_at_secs: Option<i64>,
}

/// APPEND an uploaded message and reconcile it into the cache before returning.
///
/// # Errors
///
/// Returns [`SyncError`] if the uploaded blob is unknown, if `mailbox_id` has
/// no IMAP folder, if the IMAP `APPEND` fails, if the target folder vanishes
/// before reconcile, or if the appended message cannot be located by its
/// Message-ID afterwards.
pub async fn import_message(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   spec: ImportSpec<'_>,
   traits: SessionTraits,
) -> Result<ImportOutcome> {
   let ImportSpec {
      blob_id,
      mailbox_id,
      flags,
      received_at_secs,
   } = spec;
   let bytes = queries::blobs::get_uploaded_blob()
      .bind(&db::client(pool).await?, &account_id, &blob_id)
      .opt()
      .await?
      .map(|row| row.bytes)
      .ok_or_else(|| SyncError::Other(format!("unknown uploaded blob {blob_id}")))?;

   let resolved = resolve_mailboxes(pool, account_id, &[mailbox_id.to_owned()])
      .await?
      .into_iter()
      .next()
      .ok_or_else(|| SyncError::Other(format!("mailbox id {mailbox_id} has no IMAP folder")))?;
   let (folder_id, imap_name) = (resolved.folder_id, resolved.imap_name);

   // A stable Message-ID distinguishes this APPEND from concurrent arrivals.
   let (append_bytes, message_id_search) = ensure_message_id(&bytes, account_id);

   let flags_str = if flags.is_empty() {
      None
   } else {
      Some(format!("({})", flags.join(" ")))
   };
   let internaldate = received_at_secs.map(|secs| {
      let dt =
         chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0).unwrap_or_else(chrono::Utc::now);
      dt.format("%d-%b-%Y %H:%M:%S %z").to_string()
   });
   session
      .append(
         &imap_name,
         flags_str.as_deref(),
         internaldate.as_deref(),
         append_bytes.as_ref(),
      )
      .await?;

   // 5. Pull the folder back in so the new message lands in the cache.
   let folder_name_only = imap_name.clone();
   let folders = imap::list_folders(session).await?;
   let folder = folders
      .into_iter()
      .find(|folder| folder.name == folder_name_only)
      .ok_or_else(|| {
         SyncError::Other(format!(
            "folder {folder_name_only} disappeared after APPEND"
         ))
      })?;
   let opts = interactive_options(traits);
   let _ = reconcile_folder(session, pool, account_id, &folder, opts).await?;

   // 6. Locate the appended message via its (now guaranteed-unique) Message-ID
   //    header. SELECTing the folder fresh and probing the message_id_header
   //    column resists concurrent arrivals: a parallel append from another client
   //    (or even our own retry) has its own Message-ID and won't satisfy this
   //    lookup.
   let imported = queries::messages::imported_message_by_header()
      .bind(
         &db::client(pool).await?,
         &account_id,
         &folder_id,
         &message_id_search.as_str(),
      )
      .opt()
      .await?
      .ok_or_else(|| {
         SyncError::Other(
            "APPEND succeeded but the Message-ID did not surface after reconcile".into(),
         )
      })?;

   Ok(ImportOutcome {
      msgid:     imported.msgid,
      thread_id: imported.thrid,
   })
}

/// If the bytes already include a Message-ID header, return them unchanged
/// alongside the unangled token. Otherwise prepend a synthesized one with
/// a `<jmap-import-{sha256}@{host}>` form so we can locate the message via
/// `message_id_header` after reconcile. The substitute is hash-based so a
/// retry of the same import yields a stable id (no duplicate row in the
/// cache after a transient error).
fn ensure_message_id<'a>(bytes: &'a [u8], account_id: &str) -> (Cow<'a, [u8]>, String) {
   if let Some(existing) = extract_message_id_header(bytes) {
      return (Cow::Borrowed(bytes), existing);
   }
   let mut hash = Sha1::new();
   hash.update(account_id.as_bytes());
   hash.update(b"\0");
   hash.update(bytes);
   let token = format!(
      "jmap-import-{}@import.local",
      hex::encode(&hash.finalize()[..12])
   );
   let header = format!("Message-ID: <{token}>\r\n");
   let mut out = Vec::with_capacity(header.len() + bytes.len());
   out.extend_from_slice(header.as_bytes());
   out.extend_from_slice(bytes);
   (Cow::Owned(out), token)
}

fn extract_message_id_header(bytes: &[u8]) -> Option<String> {
   mail_parser::MessageParser::default()
      .parse_headers(bytes)?
      .message_id()
      .filter(|id| !id.is_empty())
      .map(str::to_owned)
}

#[cfg(test)]
mod validation_tests {
   use super::*;

   #[test]
   fn keyword_validator_rejects_atom_specials() {
      assert!(!is_valid_keyword("bad) \\Deleted ("));
      assert!(!is_valid_keyword(""));
      assert!(!is_valid_keyword("has space"));
      assert!(!is_valid_keyword("has(paren"));
      assert!(!is_valid_keyword("has\\backslash"));
      assert!(!is_valid_keyword("has\"quote"));
      assert!(!is_valid_keyword("has*wild"));
      assert!(!is_valid_keyword("has\nnewline"));
   }

   #[test]
   fn keyword_validator_accepts_real_keywords() {
      assert!(is_valid_keyword("$seen"));
      assert!(is_valid_keyword("$flagged"));
      assert!(is_valid_keyword("$Forwarded"));
      assert!(is_valid_keyword("project-x"));
      assert!(is_valid_keyword("a"));
   }

   #[test]
   fn extract_message_id_handles_lf_only() {
      let bytes = b"From: a@b\nMessage-ID: <abc@x>\n\nbody";
      assert_eq!(extract_message_id_header(bytes).as_deref(), Some("abc@x"));
   }

   #[test]
   fn extract_message_id_returns_none_when_absent() {
      let bytes = b"From: a@b\r\n\r\nbody";
      assert!(extract_message_id_header(bytes).is_none());
   }

   #[test]
   fn ensure_message_id_inserts_when_missing() {
      let bytes = b"From: a@b\r\n\r\nbody";
      let (out, token) = ensure_message_id(bytes, "acctA");
      assert!(token.starts_with("jmap-import-"), "{token}");
      let text = from_utf8(out.as_ref()).unwrap();
      assert!(text.starts_with("Message-ID: <jmap-import-"), "{text}");
   }

   #[test]
   fn ensure_message_id_passes_through_when_present() {
      let bytes = b"Message-ID: <abc@x>\r\nFrom: a@b\r\n\r\nbody";
      let (out, token) = ensure_message_id(bytes, "acctA");
      assert_eq!(token, "abc@x");
      assert_eq!(out.as_ref(), bytes);
   }
}

/// JMAP keyword validation (RFC 8621 §4.1.1, which references RFC 5788).
///
/// A keyword is an `atom` per RFC 3501 — 1+ ATOM-CHARs. ATOM-CHAR excludes
/// `(`, `)`, `{`, SP, CTL (0x00–0x1F + 0x7F), list-wildcards `%`/`*`,
/// quoted-specials `"`/`\`, resp-specials `]`. This is a hard security
/// boundary: an unvalidated keyword reaches `+FLAGS.SILENT (...)` verbatim,
/// where `bad) \Deleted (` would silently mark the message for deletion.
#[must_use]
pub fn is_valid_keyword(keyword: &str) -> bool {
   if keyword.is_empty() {
      return false;
   }
   keyword.chars().all(|ch| {
      // Reject anything outside printable ASCII.
      if !ch.is_ascii() || ch.is_ascii_control() {
         return false;
      }
      // Reject IMAP atom-specials and friends.
      !matches!(ch, '(' | ')' | '{' | ' ' | '%' | '*' | '"' | '\\' | ']')
   })
}

/// Inverse of [`imap_flag_to_keyword`]: JMAP `$seen` → IMAP `\Seen`.
#[must_use]
pub fn keyword_to_imap_flag(keyword: &str) -> String {
   match keyword {
      "$seen" => r"\Seen".into(),
      "$flagged" => r"\Flagged".into(),
      "$answered" => r"\Answered".into(),
      "$deleted" => r"\Deleted".into(),
      "$draft" => r"\Draft".into(),
      other => other.to_owned(),
   }
}

/// Parsed-message projection into the JMAP Email body/attachment shape.
pub struct ProjectedBody {
   pub body_values: HashMap<String, EmailBodyValue>,
   pub text_body:   Vec<EmailBodyPart>,
   pub html_body:   Vec<EmailBodyPart>,
   pub attachments: Vec<EmailBodyPart>,
   pub headers:     Vec<EmailHeader>,
   pub preview:     Option<String>,
}

/// Projection leaves blob ids null until the caller knows the message id.
#[must_use]
pub fn project_to_jmap_with_msgid(
   msg: &mail_parser::Message<'_>,
   msgid: Option<&str>,
) -> ProjectedBody {
   use mail_parser::PartType;

   let mut body_values = HashMap::new();
   for (index, part) in msg.parts.iter().enumerate() {
      let (PartType::Text(value) | PartType::Html(value)) = &part.body else {
         continue;
      };
      body_values.insert(format!("p{index}"), EmailBodyValue {
         value:               normalize_body_newlines(value),
         is_encoding_problem: part.is_encoding_problem,
         is_truncated:        false,
      });
   }

   let project = |index| projected_body_part(msg, index, msgid);
   let text_parts = msg
      .text_body
      .iter()
      .filter_map(|index| usize::try_from(*index).ok().and_then(&project))
      .collect::<Vec<EmailBodyPart>>();
   let html_parts = msg
      .html_body
      .iter()
      .filter_map(|index| usize::try_from(*index).ok().and_then(&project))
      .collect::<Vec<EmailBodyPart>>();
   let attachments = msg
      .attachments
      .iter()
      .filter_map(|index| usize::try_from(*index).ok().and_then(&project))
      .collect::<Vec<EmailBodyPart>>();
   let headers = projected_headers(msg, msg.headers());

   // Preview = first ~256 chars of plain-text body, stopping on a char
   // boundary (not a byte index) to avoid splitting a multibyte codepoint.
   let preview = msg.body_text(0).map(|text| {
      let trimmed = text.trim();

      trimmed.chars().take(256).collect::<String>()
   });

   ProjectedBody {
      body_values,
      text_body: text_parts,
      html_body: html_parts,
      attachments,
      headers,
      preview,
   }
}

fn projected_body_part(
   message: &mail_parser::Message<'_>,
   index: usize,
   msgid: Option<&str>,
) -> Option<EmailBodyPart> {
   use mail_parser::{
      MimeHeaders as _,
      PartType,
   };

   let part = message.parts.get(index)?;
   if matches!(&part.body, PartType::Multipart(_)) {
      return None;
   }
   let content_type = part.content_type();
   let mime_type = content_type.map_or_else(
      || {
         match &part.body {
            PartType::Text(_) => "text/plain".into(),
            PartType::Html(_) => "text/html".into(),
            PartType::Message(_) => "message/rfc822".into(),
            _ => "application/octet-stream".into(),
         }
      },
      |content_type| {
         format!(
            "{}/{}",
            content_type.ctype(),
            content_type.subtype().unwrap_or("octet-stream")
         )
      },
   );
   let part_id = format!("p{index}");
   let charset = mime_type.starts_with("text/").then(|| {
      content_type
         .and_then(|content_type| content_type.attribute("charset"))
         .unwrap_or("us-ascii")
         .to_owned()
   });
   Some(EmailBodyPart {
      blob_id: msgid.map(|msgid| Id(format!("blob-{msgid}~{part_id}"))),
      part_id: Some(part_id),
      size: part.contents().len() as u64,
      mime_type,
      charset,
      disposition: part
         .content_disposition()
         .map(|disposition| disposition.ctype().to_ascii_lowercase()),
      name: part.attachment_name().map(str::to_owned),
      content_id: part
         .content_id()
         .map(|content_id| content_id.trim_matches(['<', '>']).to_owned()),
      language: part.content_language().as_text_list().map(|languages| {
         languages
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
      }),
      location: part.content_location().map(str::to_owned),
      headers: Some(projected_headers(message, &part.headers)),
      sub_parts: None,
   })
}

fn projected_headers(
   message: &mail_parser::Message<'_>,
   headers: &[mail_parser::Header<'_>],
) -> Vec<EmailHeader> {
   headers
      .iter()
      .map(|header| {
         EmailHeader {
            name:  header.name().to_owned(),
            value: usize::try_from(header.offset_start())
               .ok()
               .zip(usize::try_from(header.offset_end()).ok())
               .and_then(|(start, end)| message.raw_message.get(start..end))
               .and_then(|bytes| from_utf8(bytes).ok())
               .unwrap_or_default()
               .trim_end_matches(['\r', '\n'])
               .to_owned(),
         }
      })
      .collect::<Vec<_>>()
}

fn normalize_body_newlines(value: &str) -> String {
   value.replace("\r\n", "\n").replace('\r', "\n")
}

/// Per-folder reconcile: walks every synced folder, pulls new UIDs,
/// re-validates flags on recently-seen messages, and detects expunges.
///
/// Cost: one SELECT + one UID SEARCH + one bounded UID FETCH per folder.
/// On Gmail's All Mail (many tens of thousands of messages) the SEARCH is
/// still cheap because the server returns only the UID list.
///
/// # Errors
///
/// Returns [`SyncError`] if listing the account's folders fails or if the
/// final `recompute_mailbox_counts` cannot reach Postgres. Individual folder
/// reconcile failures are logged and skipped rather than aborting the pass.
pub async fn reconcile_all_folders(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   backfill_days: u32,
) -> Result<u64> {
   let folders = imap::list_folders(session).await?;
   let opts = sync_options(session, backfill_days).await;
   let mut total_changes = 0_u64;
   for folder in &folders {
      if should_skip(folder) {
         continue;
      }
      match reconcile_folder(session, pool, account_id, folder, opts).await {
         Ok(n) => total_changes += n,
         Err(err) => {
            warn!(
                account_id,
                folder = %folder.name,
                error = %err,
                "folder reconcile failed; continuing",
            );
         },
      }
   }
   if total_changes > 0 {
      db::recompute_mailbox_counts(pool, account_id).await?;
   }
   Ok(total_changes)
}

async fn reconcile_folder(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   folder: &ImapFolder,
   opts: SyncOptions,
) -> Result<u64> {
   // Its idempotent upsert only bumps modseq when rows actually change.
   let mut changed = sync_folder(session, pool, account_id, folder, opts).await?;

   if !opts.detect_expunges {
      return Ok(changed);
   }

   // Look up the folder's cached id (sync_folder just upserted it).
   let cached_folder = queries::folders::folder_by_name()
      .bind(&db::client(pool).await?, &account_id, &folder.name.as_str())
      .opt()
      .await?;
   let Some(cached_folder) = cached_folder else {
      return Ok(changed);
   };
   let folder_id = cached_folder.id;

   let cached_uids = queries::messages::uids_in_folder()
      .bind(&db::client(pool).await?, &account_id, &folder_id)
      .all()
      .await?
      .into_iter()
      .map(|uid| uid as u32)
      .collect::<BTreeSet<u32>>();

   if cached_uids.is_empty() {
      return Ok(changed);
   }

   // Server-side current UID set via UID SEARCH. Much cheaper than a full
   // FETCH because the server only returns the integer list.
   let server_uids = session
      .uid_search("ALL")
      .await?
      .into_iter()
      .collect::<HashSet<u32>>();

   // Any cached UID not in the server set has been expunged.
   let expunged = cached_uids
      .into_iter()
      .filter(|uid| !server_uids.contains(uid))
      .collect::<BTreeSet<u32>>();

   let reconciled = apply_expunges(
      pool,
      account_id,
      folder_id,
      &cached_folder.mailbox_id,
      &expunged,
   )
   .await?;
   if reconciled > 0 {
      debug!(
          account_id,
          folder = %folder.name,
          expunged = reconciled,
          "reconciled expunges",
      );
   }
   changed += reconciled;

   Ok(changed)
}

/// Clear every cached occupant of each expunged UID slot.
async fn apply_expunges(
   pool: &PgPool,
   account_id: &str,
   folder_id: i64,
   mailbox_id: &str,
   expunged: &BTreeSet<u32>,
) -> Result<u64> {
   if expunged.is_empty() {
      return Ok(0);
   }
   let mut changed = 0_u64;
   let mut conn = db::client(pool).await?;
   let tx = conn.transaction().await?;
   for uid in expunged {
      let msgids = queries::messages::msgid_for_folder_uid()
         .bind(&tx, &account_id, &folder_id, &i64::from(*uid))
         .all()
         .await?;
      if msgids.is_empty() {
         continue;
      }
      queries::messages::delete_message_imap_by_uid()
         .bind(&tx, &account_id, &folder_id, &i64::from(*uid))
         .await?;
      for msgid in &msgids {
         db::remove_folder_placement(&tx, account_id, msgid, mailbox_id).await?;
      }
      changed += 1;
   }
   tx.commit().await?;
   Ok(changed)
}

/// Delta sync — after an IDLE notification, fetch new UIDs since the folder's
/// stored `uidnext`. Kept as a narrow entry point; the broader reconcile path
/// is [`reconcile_all_folders`].
///
/// # Errors
///
/// Returns [`SyncError`] if the IMAP `SELECT`, `UID FETCH`, or a database
/// query fails. When the server rotated the folder's UIDVALIDITY the cache is
/// purged and a full folder sync runs; a failure in that fallback propagates
/// too.
pub async fn delta_sync(
   session: &mut ImapSession,
   pool: &PgPool,
   account_id: &str,
   folder_name: &str,
   backfill_days: u32,
) -> Result<u64> {
   let opts = sync_options(session, backfill_days).await;
   let mbox = session.select(folder_name).await?;
   let uidvalidity = mbox.uid_validity.unwrap_or(0);
   let uidnext = mbox.uid_next.unwrap_or(0);

   let stored = queries::folders::folder_by_name()
      .bind(&db::client(pool).await?, &account_id, &folder_name)
      .opt()
      .await?;

   let Some(stored) = stored else {
      // Unknown folder — fall through to a full folder sync.
      let folders = imap::list_folders(session).await?;
      for folder in &folders {
         if folder.name == folder_name {
            return sync_folder(session, pool, account_id, folder, opts).await;
         }
      }
      return Ok(0);
   };
   let (folder_id, stored_uidvalidity, stored_uidnext, mailbox_id) = (
      stored.id,
      stored.uidvalidity,
      stored.uidnext,
      stored.mailbox_id,
   );

   if stored_uidvalidity != 0 && (stored_uidvalidity as u32) != uidvalidity {
      // Server rotated UIDVALIDITY — our UID cache for this folder is
      // meaningless. Purge messages that only lived here (and strip the
      // mapping to this folder's mailbox from messages that survive in
      // other folders), then re-ingest.
      let dropped =
         db::purge_folder_for_uidvalidity_change(pool, account_id, folder_id, &mailbox_id).await?;
      warn!(
         account_id,
         folder = folder_name,
         was = stored_uidvalidity as u32,
         now = uidvalidity,
         dropped,
         "UIDVALIDITY rotated; purged folder and falling back to full sync",
      );
      let folders = imap::list_folders(session).await?;
      for folder in &folders {
         if folder.name == folder_name {
            let n = sync_folder(session, pool, account_id, folder, opts).await?;
            // Either branch of purge (orphan delete, survivor remap) can
            // shift total/unread counts, and sync_folder repopulating the
            // mailbox mapping doesn't recompute counts by itself.
            db::recompute_mailbox_counts(pool, account_id).await?;
            return Ok(n);
         }
      }
      // Folder is no longer listed (deleted server-side) — still recompute
      // so the purge's dropped memberships reflect in Mailbox/get.
      db::recompute_mailbox_counts(pool, account_id).await?;
      return Ok(0);
   }
   let _ = stored_uidvalidity; // no-op when value was 0 (fresh folder)

   let low = (stored_uidnext as u32).max(1);
   if low >= uidnext {
      return Ok(0);
   }
   let uid_set = format!("{}:{}", low, uidnext.saturating_sub(1));

   let query = "(UID FLAGS INTERNALDATE RFC822.SIZE ENVELOPE BODY.PEEK[HEADER.FIELDS (MESSAGE-ID \
                REFERENCES IN-REPLY-TO)])";

   let mut count = 0_u64;
   let mut decoded = Vec::<DecodedFetch>::new();
   let mut skipped = Vec::<u32>::new();
   {
      let mut fetches = session.uid_fetch(&uid_set, query).await?;
      while let Some(fetch) = fetches.next().await {
         let fetch = fetch?;
         match decode_fetch(&fetch, uidvalidity) {
            FetchDecode::Complete(env) => decoded.push(*env),
            FetchDecode::Incomplete { uid } => {
               warn!(
                  account_id,
                  folder = folder_name,
                  uid,
                  "FETCH missing INTERNALDATE/RFC822.SIZE/ENVELOPE; deferring to next round",
               );
               skipped.push(uid);
            },
            FetchDecode::Anonymous => {
               warn!(
                  account_id,
                  folder = folder_name,
                  "FETCH response without UID; ignoring"
               );
            },
         }
      }
   }
   let gm_thrids = gmail_thrids_for(session, &decoded, opts).await;
   for mut env in decoded {
      env.envelope.thrid = thrid_for(
         pool,
         account_id,
         &env.envelope,
         gm_thrids.get(&env.uid).copied(),
      )
      .await?;
      db::upsert_message(pool, account_id, &env.envelope).await?;
      db::upsert_message_imap(
         pool,
         account_id,
         &env.envelope.msgid,
         folder_id,
         env.uid,
         uidvalidity,
         &mailbox_id,
      )
      .await?;
      queries::messages::add_message_mailbox()
         .bind(
            &db::client(pool).await?,
            &account_id,
            &env.envelope.msgid.as_str(),
            &mailbox_id.as_str(),
         )
         .await?;
      count += 1;
   }

   let (checkpoint_uidnext, _) = skip_checkpoints(&skipped, Some(low), uidnext);
   queries::folders::set_folder_uidnext()
      .bind(
         &db::client(pool).await?,
         &i64::from(checkpoint_uidnext),
         &folder_id,
      )
      .await?;

   if count > 0 {
      db::recompute_mailbox_counts(pool, account_id).await?;
   }

   Ok(count)
}

#[cfg(test)]
mod tests {
   use chrono::TimeZone as _;

   use super::*;
   use crate::{
      provider::ProviderKind,
      testkit,
   };

   #[test]
   fn mailbox_id_is_stable() {
      let first = mailbox_id_for_folder("acct", "INBOX");
      let second = mailbox_id_for_folder("acct", "INBOX");
      assert_eq!(first, second);
      assert_ne!(
         mailbox_id_for_folder("acct", "INBOX"),
         mailbox_id_for_folder("acct", "Sent")
      );
      assert!(first.starts_with("mb_"));
   }

   #[test]
   fn full_history_deepens_partial_checkpoint() {
      assert_eq!(older_history_end(Some(334_919), 339_522), Some(334_918));
      assert_eq!(older_history_end(Some(1), 339_522), None);
      assert_eq!(older_history_end(Some(0), 7), Some(6));
      assert_eq!(older_history_end(None, 1), None);

      let mut ranges = Vec::new();
      push_uid_range(&mut ranges, 1, 334_919);
      push_uid_range(&mut ranges, 339_522, 339_525);
      push_uid_range(&mut ranges, 4, 4);
      assert_eq!(ranges, ["1:334918", "339522:339524"]);
   }

   #[test]
   fn folder_name_validation() {
      assert!(is_valid_folder_name("Receipts"));
      assert!(is_valid_folder_name("Work/2026"));
      assert!(is_valid_folder_name("émails à trier"));
      assert!(!is_valid_folder_name(""));
      assert!(!is_valid_folder_name("bad\r\nA1 DELETE INBOX"));
      assert!(!is_valid_folder_name("wild*card"));
      assert!(!is_valid_folder_name("wild%card"));
      assert!(!is_valid_folder_name("quo\"te"));
      assert!(!is_valid_folder_name(&"x".repeat(300)));
   }

   fn empty_envelope() -> Envelope<'static> {
      Envelope {
         date:        None,
         subject:     None,
         from:        None,
         sender:      None,
         reply_to:    None,
         to:          None,
         cc:          None,
         bcc:         None,
         in_reply_to: None,
         message_id:  None,
      }
   }

   #[test]
   fn fetch_requires_uid_date_size_and_envelope() {
      let ts = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
      let envelope = empty_envelope();
      let parts = |uid, internal_date, size, envelope| {
         FetchParts {
            uid,
            internal_date,
            size,
            envelope,
            header: None,
            flags: vec![],
         }
      };
      assert!(matches!(
         decode_parts(parts(Some(9), Some(ts), Some(1), Some(&envelope)), 1),
         FetchDecode::Complete(_)
      ));
      for incomplete in [
         parts(Some(9), None, Some(1), Some(&envelope)),
         parts(Some(9), Some(ts), None, Some(&envelope)),
         parts(Some(9), Some(ts), Some(1), None),
      ] {
         assert!(matches!(
            decode_parts(incomplete, 1),
            FetchDecode::Incomplete { uid: 9 }
         ));
      }
      assert!(matches!(
         decode_parts(parts(None, Some(ts), Some(1), Some(&envelope)), 1),
         FetchDecode::Anonymous
      ));
   }

   #[test]
   fn skipped_uids_hold_back_sync_checkpoints() {
      assert_eq!(skip_checkpoints(&[], Some(100), 200), (200, true));
      assert_eq!(skip_checkpoints(&[150, 180], Some(100), 200), (150, true));
      assert_eq!(skip_checkpoints(&[50], Some(100), 200), (200, false));
      assert_eq!(skip_checkpoints(&[50, 150], Some(100), 200), (150, false));
      assert_eq!(skip_checkpoints(&[5], None, 200), (5, true));
   }

   #[test]
   fn interactive_reconcile_never_touches_history() {
      let periodic = SyncOptions {
         backfill_days:   0,
         gmail_thrid:     false,
         deepen_history:  true,
         detect_expunges: true,
      };
      let interactive = interactive_options(SessionTraits {
         gmail:    false,
         has_move: true,
         uidplus:  true,
      });
      assert!(!interactive.detect_expunges);
      assert!(!interactive.deepen_history);
      assert!(
         interactive_options(SessionTraits {
            gmail:    true,
            has_move: true,
            uidplus:  true,
         })
         .gmail_thrid
      );
      assert!(wants_history_deepening(periodic, Some(250_865)));
      assert!(!wants_history_deepening(interactive, Some(250_865)));
      assert!(!wants_history_deepening(periodic, Some(1)));
      assert_eq!(fresh_backfill_low(periodic, 340_000), 1);
      assert_eq!(
         fresh_backfill_low(interactive, 340_000),
         340_000 - INITIAL_WINDOW
      );
   }

   #[tokio::test]
   async fn move_bookkeeping_updates_placements_without_a_sweep() {
      let Some(pool) = testkit::test_pool().await else {
         return;
      };
      let client = pool.get().await.unwrap();
      client
         .batch_execute(
            "INSERT INTO accounts (id, email, provider, display_name, bearer_token_hash, \
             created_at) VALUES ('gmail', 'a@b.c', 'gmail', 'A', ''::bytea, 0);
             INSERT INTO state (account_id) VALUES ('gmail');
             INSERT INTO mailboxes (id, account_id, name, modseq) VALUES
                ('mb_inbox', 'gmail', 'INBOX', 0),
                ('mb_trash', 'gmail', 'Trash', 0);
             INSERT INTO folders (account_id, imap_name, mailbox_id) VALUES
                ('gmail', 'INBOX', 'mb_inbox'),
                ('gmail', 'Trash', 'mb_trash');
             INSERT INTO messages (account_id, msgid, thrid, received_at, size, modseq) VALUES
                ('gmail', 'm1', 'm1', 0, 1, 0);
             INSERT INTO message_imap (account_id, msgid, folder_id, uid, uidvalidity)
             SELECT 'gmail', 'm1', id, 9, 1 FROM folders WHERE imap_name = 'INBOX';
             INSERT INTO message_mailboxes (account_id, msgid, mailbox_id) VALUES
                ('gmail', 'm1', 'mb_inbox');",
         )
         .await
         .unwrap();
      let inbox_folder: i64 = client
         .query_one("SELECT id FROM folders WHERE imap_name = 'INBOX'", &[])
         .await
         .unwrap()
         .get(0);
      drop(client);

      apply_move_bookkeeping(
         &pool,
         "gmail",
         "m1",
         &[(inbox_folder, "mb_inbox".to_owned())],
         &["mb_trash".to_owned()],
      )
      .await
      .unwrap();

      let client = pool.get().await.unwrap();
      let row = client
         .query_one(
            "SELECT
                (SELECT COUNT(*) FROM message_imap WHERE folder_id = $1),
                (SELECT COUNT(*) FROM message_mailboxes WHERE mailbox_id = 'mb_inbox'),
                (SELECT COUNT(*) FROM message_mailboxes WHERE mailbox_id = 'mb_trash'),
                (SELECT modseq FROM messages WHERE msgid = 'm1'),
                (SELECT email_modseq FROM state WHERE account_id = 'gmail')",
            &[&inbox_folder],
         )
         .await
         .unwrap();
      assert_eq!(row.get::<_, i64>(0), 0, "source placement removed");
      assert_eq!(row.get::<_, i64>(1), 0, "source membership removed");
      assert_eq!(row.get::<_, i64>(2), 1, "destination membership added");
      let message_modseq: i64 = row.get(3);
      let state_modseq: i64 = row.get(4);
      assert_eq!(
         message_modseq, state_modseq,
         "message stamped so Email/changes reports the move"
      );
      assert!(state_modseq > 0);
   }

   #[test]
   fn msgid_header_normalizes_brackets() {
      let first = normalized_msgid(Some("<abc@example.com>"), 1, 1, Utc::now());
      let second = normalized_msgid(Some("abc@example.com"), 1, 1, Utc::now());
      assert_eq!(first, second);
   }

   #[test]
   fn msgid_falls_back_when_missing() {
      let ts = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
      let first = normalized_msgid(None, 42, 100, ts);
      let second = normalized_msgid(None, 42, 100, ts);
      assert_eq!(first, second);
      assert!(first.starts_with("fb_"));
      let third = normalized_msgid(None, 42, 101, ts);
      assert_ne!(first, third);
   }

   #[test]
   fn imap_date_parses_common_forms() {
      assert!(parse_imap_date(Some(b"Mon, 01 Jan 2024 12:34:56 +0000")).is_some());
      assert!(parse_imap_date(Some(b"Tue, 2 Jan 2024 12:34:56 -0500")).is_some());
      assert!(parse_imap_date(None).is_none());
   }

   #[test]
   fn envelope_text_decodes_rfc2047() {
      assert_eq!(
         decode_envelope_text(b"=?UTF-8?Q?Startups_=26_Tech_Events_in_NY?=").as_deref(),
         Some("Startups & Tech Events in NY")
      );
      assert_eq!(
         decode_envelope_text(b"Plain subject").as_deref(),
         Some("Plain subject")
      );
   }

   #[tokio::test]
   async fn cached_metadata_repair_decodes_subject_and_restores_preview() {
      let Some(pool) = testkit::test_pool().await else {
         return;
      };
      db::upsert_account(&pool, "a", "a@example.test", ProviderKind::Imap, "A", b"h")
         .await
         .unwrap();
      db::upsert_message(&pool, "a", &db::MessageEnvelope {
         msgid:              "m1".into(),
         thrid:              "t1".into(),
         flags:              vec![],
         received_at:        Utc::now(),
         sent_at:            None,
         size:               42,
         from:               None,
         to:                 None,
         cc:                 None,
         bcc:                None,
         reply_to:           None,
         subject:            Some("=?UTF-8?Q?Startups_=26_Tech_Events_in_NY?=".into()),
         preview:            None,
         has_attachment:     false,
         message_id_header:  None,
         in_reply_to_header: None,
         references_header:  None,
      })
      .await
      .unwrap();
      let raw = b"From: events@example.test\r\n\
Subject: =?UTF-8?Q?Startups_=26_Tech_Events_in_NY?=\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
Cached body preview";
      queries::raw_messages::upsert_raw_message()
         .bind(
            &db::client(&pool).await.unwrap(),
            &"a",
            &"m1",
            &"{}",
            &"{}",
            &"[]",
            &raw.as_slice(),
         )
         .await
         .unwrap();
      let before = db::get_state(&pool, "a").await.unwrap().email_modseq;

      assert_eq!(repair_cached_body_metadata(&pool, "a").await.unwrap(), 1);
      let row = queries::messages::message_body_metadata()
         .bind(&db::client(&pool).await.unwrap(), &"a", &"m1")
         .one()
         .await
         .unwrap();
      assert_eq!(row.subject.as_deref(), Some("Startups & Tech Events in NY"));
      assert_eq!(row.preview.as_deref(), Some("Cached body preview"));
      assert_eq!(
         db::get_state(&pool, "a").await.unwrap().email_modseq,
         before + 1
      );
      assert_eq!(repair_cached_body_metadata(&pool, "a").await.unwrap(), 0);
   }

   #[test]
   fn display_name_strips_gmail_prefix() {
      assert_eq!(display_name_for("[Gmail]/Sent Mail"), "Sent Mail");
      assert_eq!(display_name_for("INBOX"), "INBOX");
   }

   #[test]
   fn body_projection_uses_mime_part_ids_and_keeps_all_text_values() {
      let raw = b"From: a@example.test\r\n\
Content-Type: multipart/mixed; boundary=m\r\n\
\r\n\
--m\r\n\
Content-Type: multipart/alternative; boundary=a\r\n\
\r\n\
--a\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
plain\r\n\
--a\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>html</p>\r\n\
--a--\r\n\
--m\r\n\
Content-Type: text/plain; name=notes.txt\r\n\
Content-Disposition: attachment; filename=notes.txt\r\n\
\r\n\
notes\r\n\
--m--\r\n";
      let message = mail_parser::MessageParser::default().parse(raw).unwrap();
      let body = project_to_jmap_with_msgid(&message, Some("m1"));

      for part in body
         .text_body
         .iter()
         .chain(&body.html_body)
         .chain(&body.attachments)
      {
         let part_id = part.part_id.as_deref().unwrap();
         assert!(part_id.starts_with('p'));
         assert_eq!(
            part.blob_id.as_ref().unwrap().as_ref(),
            format!("blob-m1~{part_id}")
         );
      }
      assert!(body.body_values.len() >= 3);
      assert!(
         body
            .body_values
            .values()
            .all(|value| !value.value.contains('\r'))
      );
      let attachment_id = body.attachments[0].part_id.as_ref().unwrap();
      assert!(body.body_values.contains_key(attachment_id));
      assert!(
         body.text_body[0]
            .headers
            .as_ref()
            .unwrap()
            .iter()
            .any(|header| header.name.eq_ignore_ascii_case("Content-Type"))
      );
   }
}
