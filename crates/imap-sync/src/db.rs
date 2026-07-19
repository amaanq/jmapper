//! Database access through the generated `jmapper-codegen` queries.

use chrono::{
   DateTime,
   Utc,
};
use deadpool_postgres::Pool;
use jmap_protocol::email::EmailAddress;
use jmapper_codegen::queries;
use serde::{
   Deserialize,
   Serialize,
};

use crate::{
   error::Result,
   provider::ProviderKind,
};

/// # Errors
///
/// Returns an error when acquiring a database connection fails.
#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn client(pool: &Pool) -> Result<deadpool_postgres::Object> {
   Ok(pool.get().await?)
}

// ================= Accounts =================

#[derive(Debug, Clone)]
pub struct AccountRow {
   pub id:                String,
   pub email:             String,
   pub provider:          String,
   pub display_name:      String,
   pub bearer_token_hash: Vec<u8>,
   pub created_at:        i64,
}

impl AccountRow {
   #[must_use]
   #[inline]
   pub fn provider_kind(&self) -> ProviderKind {
      match self.provider.as_str() {
         "gmail" => ProviderKind::Gmail,
         _ => ProviderKind::Imap,
      }
   }
}

impl From<queries::accounts::AccountRow> for AccountRow {
   #[inline]
   fn from(row: queries::accounts::AccountRow) -> Self {
      Self {
         id:                row.id,
         email:             row.email,
         provider:          row.provider,
         display_name:      row.display_name,
         bearer_token_hash: row.bearer_token_hash,
         created_at:        row.created_at,
      }
   }
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn upsert_account(
   pool: &Pool,
   id: &str,
   email: &str,
   provider: ProviderKind,
   display_name: &str,
   bearer_token_hash: &[u8],
) -> Result<()> {
   let provider_str = match provider {
      ProviderKind::Gmail => "gmail",
      ProviderKind::Imap => "imap",
   };
   let client = client(pool).await?;
   queries::accounts::upsert_account()
      .bind(
         &client,
         &id,
         &email,
         &provider_str,
         &display_name,
         &bearer_token_hash,
      )
      .await?;
   queries::accounts::ensure_state_row()
      .bind(&client, &id)
      .await?;
   Ok(())
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn get_account(pool: &Pool, id: &str) -> Result<Option<AccountRow>> {
   let client = client(pool).await?;
   Ok(queries::accounts::get_account()
      .bind(&client, &id)
      .opt()
      .await?
      .map(Into::into))
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn list_accounts(pool: &Pool) -> Result<Vec<AccountRow>> {
   let client = client(pool).await?;
   Ok(queries::accounts::list_accounts()
      .bind(&client)
      .all()
      .await?
      .into_iter()
      .map(Into::into)
      .collect())
}

// ================= OAuth =================

#[derive(Debug, Clone)]
pub struct OAuthTokenRow {
   pub account_id:    String,
   pub access_token:  Option<String>,
   pub refresh_token: String,
   pub expires_at:    Option<i64>,
}

impl From<queries::oauth::OAuthTokenRow> for OAuthTokenRow {
   #[inline]
   fn from(row: queries::oauth::OAuthTokenRow) -> Self {
      Self {
         account_id:    row.account_id,
         access_token:  row.access_token,
         refresh_token: row.refresh_token,
         expires_at:    row.expires_at,
      }
   }
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn upsert_oauth(
   pool: &Pool,
   account_id: &str,
   access_token: Option<&str>,
   refresh_token: &str,
   expires_at: Option<DateTime<Utc>>,
) -> Result<()> {
   let client = client(pool).await?;
   queries::oauth::upsert_oauth()
      .bind(
         &client,
         &account_id,
         &access_token,
         &refresh_token,
         &expires_at.map(|timestamp| timestamp.timestamp()),
      )
      .await?;
   Ok(())
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn get_oauth(pool: &Pool, account_id: &str) -> Result<Option<OAuthTokenRow>> {
   let client = client(pool).await?;
   Ok(queries::oauth::get_oauth()
      .bind(&client, &account_id)
      .opt()
      .await?
      .map(Into::into))
}

// ================= Folders =================

#[derive(Debug, Clone)]
pub struct FolderRow {
   pub id:            i64,
   pub account_id:    String,
   pub imap_name:     String,
   pub uidvalidity:   i64,
   pub uidnext:       i64,
   pub uidfirst:      i64,
   pub highestmodseq: i64,
   pub role:          Option<String>,
   pub last_sync_at:  Option<i64>,
   pub mailbox_id:    String,
}

impl From<queries::folders::FolderRow> for FolderRow {
   #[inline]
   fn from(row: queries::folders::FolderRow) -> Self {
      Self {
         id:            row.id,
         account_id:    row.account_id,
         imap_name:     row.imap_name,
         uidvalidity:   row.uidvalidity,
         uidnext:       row.uidnext,
         uidfirst:      row.uidfirst,
         highestmodseq: row.highestmodseq,
         role:          row.role,
         last_sync_at:  row.last_sync_at,
         mailbox_id:    row.mailbox_id,
      }
   }
}

pub struct FolderUpsert<'a> {
   pub account_id:    &'a str,
   pub imap_name:     &'a str,
   pub uidvalidity:   u32,
   pub uidnext:       u32,
   pub highestmodseq: u64,
   pub role:          Option<&'a str>,
   pub mailbox_id:    &'a str,
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn upsert_folder(pool: &Pool, input: FolderUpsert<'_>) -> Result<i64> {
   let client = client(pool).await?;
   Ok(queries::folders::upsert_folder()
      .bind(
         &client,
         &input.account_id,
         &input.imap_name,
         &i64::from(input.uidvalidity),
         &i64::from(input.uidnext),
         &(input.highestmodseq as i64),
         &input.role,
         &input.mailbox_id,
      )
      .one()
      .await?)
}

/// Purge all messages, mailbox mappings, imap-uid rows, and raw bodies
/// belonging to a single folder.
///
/// Used when the server rotates UIDVALIDITY — the cached UID map becomes
/// meaningless and the folder must be re-ingested from scratch. Also resets
/// the folder's uidvalidity/uidnext/highestmodseq so the next sync pass treats
/// it as brand new.
///
/// `folder_mailbox_id` is the JMAP mailbox id that corresponds to this folder;
/// messages that survive in other folders must lose their mapping to THIS
/// folder's mailbox (otherwise Mailbox/get would still report them as members
/// of a folder they no longer live in).
#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn purge_folder_for_uidvalidity_change(
   pool: &Pool,
   account_id: &str,
   folder_id: i64,
   folder_mailbox_id: &str,
) -> Result<u64> {
   let mut client = client(pool).await?;
   let tx = client.transaction().await?;

   // Find the msgids scoped to this folder. A message may also live in other
   // folders (Gmail label semantics); we only drop the messages whose ONLY
   // folder membership is this one. Everything else just loses its mapping
   // to this folder via the message_imap delete.
   let affected = queries::messages::msgids_in_folder()
      .bind(&tx, &account_id, &folder_id)
      .all()
      .await?;

   queries::messages::delete_message_imap_for_folder()
      .bind(&tx, &account_id, &folder_id)
      .await?;

   let mut deleted_count = 0_u64;
   for msgid in &affected {
      let still_elsewhere = queries::messages::count_message_imap()
         .bind(&tx, &account_id, &msgid.as_str())
         .one()
         .await?;
      if still_elsewhere == 0 {
         // Message only lived in this folder — nuke every trace of it.
         // CASCADE on the FKs takes care of message_mailboxes + raw_messages.
         queries::messages::delete_message()
            .bind(&tx, &account_id, &msgid.as_str())
            .await?;
         deleted_count += 1;
      } else {
         // Message survives in another folder. Drop just the mapping to
         // THIS folder's mailbox so Mailbox/get stops reporting it as a
         // member. Bump the message's modseq so Email/changes can tell a
         // client that its mailboxIds list shrank.
         let removed = queries::messages::remove_message_mailbox()
            .bind(&tx, &account_id, &msgid.as_str(), &folder_mailbox_id)
            .await?;
         if removed > 0 {
            let new_modseq = queries::state::bump_email_modseq()
               .bind(&tx, &account_id)
               .one()
               .await?;
            queries::messages::set_message_modseq()
               .bind(&tx, &new_modseq, &account_id, &msgid.as_str())
               .await?;
         }
      }
   }

   queries::folders::reset_folder_sync_state()
      .bind(&tx, &folder_id, &account_id)
      .await?;

   tx.commit().await?;
   Ok(deleted_count)
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn list_folders(pool: &Pool, account_id: &str) -> Result<Vec<FolderRow>> {
   let client = client(pool).await?;
   Ok(queries::folders::list_folders()
      .bind(&client, &account_id)
      .all()
      .await?
      .into_iter()
      .map(Into::into)
      .collect())
}

// ================= Mailboxes =================

#[derive(Debug, Clone)]
pub struct MailboxRow {
   pub id:             String,
   pub account_id:     String,
   pub name:           String,
   pub parent_id:      Option<String>,
   pub role:           Option<String>,
   pub total_emails:   i64,
   pub unread_emails:  i64,
   pub total_threads:  i64,
   pub unread_threads: i64,
   pub sort_order:     i64,
   pub modseq:         i64,
}

impl From<queries::mailboxes::MailboxRow> for MailboxRow {
   #[inline]
   fn from(row: queries::mailboxes::MailboxRow) -> Self {
      Self {
         id:             row.id,
         account_id:     row.account_id,
         name:           row.name,
         parent_id:      row.parent_id,
         role:           row.role,
         total_emails:   row.total_emails,
         unread_emails:  row.unread_emails,
         total_threads:  row.total_threads,
         unread_threads: row.unread_threads,
         sort_order:     row.sort_order,
         modseq:         row.modseq,
      }
   }
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn upsert_mailbox(
   pool: &Pool,
   id: &str,
   account_id: &str,
   name: &str,
   parent_id: Option<&str>,
   role: Option<&str>,
   sort_order: u32,
) -> Result<()> {
   let mut client = client(pool).await?;
   let tx = client.transaction().await?;
   let sort_order = i64::from(sort_order);
   let unchanged = queries::mailboxes::mailbox_metadata()
      .bind(&tx, &id, &account_id)
      .opt()
      .await?
      .is_some_and(|row| {
         row.name == name
            && row.parent_id.as_deref() == parent_id
            && row.role.as_deref() == role
            && row.sort_order == sort_order
      });
   if unchanged {
      return Ok(());
   }
   let modseq = queries::state::bump_mailbox_modseq()
      .bind(&tx, &account_id)
      .one()
      .await?;
   queries::mailboxes::upsert_mailbox()
      .bind(
         &tx,
         &id,
         &account_id,
         &name,
         &parent_id,
         &role,
         &sort_order,
         &modseq,
      )
      .await?;
   tx.commit().await?;
   Ok(())
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn list_mailboxes(pool: &Pool, account_id: &str) -> Result<Vec<MailboxRow>> {
   let client = client(pool).await?;
   Ok(queries::mailboxes::list_mailboxes()
      .bind(&client, &account_id)
      .all()
      .await?
      .into_iter()
      .map(Into::into)
      .collect())
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn get_mailboxes_by_ids(
   pool: &Pool,
   account_id: &str,
   ids: &[String],
) -> Result<Vec<MailboxRow>> {
   if ids.is_empty() {
      return Ok(Vec::new());
   }
   let client = client(pool).await?;
   Ok(queries::mailboxes::get_mailboxes_by_ids()
      .bind(&client, &account_id, &ids)
      .all()
      .await?
      .into_iter()
      .map(Into::into)
      .collect())
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn recompute_mailbox_counts(pool: &Pool, account_id: &str) -> Result<()> {
   let client = client(pool).await?;
   queries::mailboxes::recompute_mailbox_counts()
      .bind(&client, &account_id)
      .await?;
   Ok(())
}

// ================= Messages =================

#[derive(Debug, Clone)]
pub struct MessageRow {
   pub account_id:         String,
   pub msgid:              String,
   pub thrid:              String,
   pub flags_json:         String,
   pub received_at:        i64,
   pub sent_at:            Option<i64>,
   pub size:               i64,
   pub from_json:          Option<String>,
   pub to_json:            Option<String>,
   pub cc_json:            Option<String>,
   pub bcc_json:           Option<String>,
   pub reply_to_json:      Option<String>,
   pub subject:            Option<String>,
   pub preview:            Option<String>,
   pub has_attachment:     i64,
   pub message_id_header:  Option<String>,
   pub in_reply_to_header: Option<String>,
   pub references_header:  Option<String>,
   pub modseq:             i64,
}

impl MessageRow {
   /// Manual mapping for the dynamic `SELECT m.* ...` queries in
   /// `jmap-server` that codegen can't type.
   ///
   /// # Errors
   ///
   /// Returns an error if any expected column is absent from `row` or holds a
   /// value whose type does not match the target field.
   #[inline]
   pub fn from_pg_row(row: &tokio_postgres::Row) -> Result<Self> {
      Ok(Self {
         account_id:         row.try_get("account_id")?,
         msgid:              row.try_get("msgid")?,
         thrid:              row.try_get("thrid")?,
         flags_json:         row.try_get("flags_json")?,
         received_at:        row.try_get("received_at")?,
         sent_at:            row.try_get("sent_at")?,
         size:               row.try_get("size")?,
         from_json:          row.try_get("from_json")?,
         to_json:            row.try_get("to_json")?,
         cc_json:            row.try_get("cc_json")?,
         bcc_json:           row.try_get("bcc_json")?,
         reply_to_json:      row.try_get("reply_to_json")?,
         subject:            row.try_get("subject")?,
         preview:            row.try_get("preview")?,
         has_attachment:     row.try_get("has_attachment")?,
         message_id_header:  row.try_get("message_id_header")?,
         in_reply_to_header: row.try_get("in_reply_to_header")?,
         references_header:  row.try_get("references_header")?,
         modseq:             row.try_get("modseq")?,
      })
   }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
   pub msgid:              String,
   pub thrid:              String,
   pub flags:              Vec<String>,
   pub received_at:        DateTime<Utc>,
   pub sent_at:            Option<DateTime<Utc>>,
   pub size:               u64,
   pub from:               Option<Vec<EmailAddress>>,
   pub to:                 Option<Vec<EmailAddress>>,
   pub cc:                 Option<Vec<EmailAddress>>,
   pub bcc:                Option<Vec<EmailAddress>>,
   pub reply_to:           Option<Vec<EmailAddress>>,
   pub subject:            Option<String>,
   pub preview:            Option<String>,
   pub has_attachment:     bool,
   /// Unbracketed Message-ID header value, if present.
   pub message_id_header:  Option<String>,
   /// In-Reply-To header value.
   pub in_reply_to_header: Option<String>,
   /// References header value (space-separated).
   pub references_header:  Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
   Inserted,
   Updated,
   Unchanged,
}

/// Upsert an envelope and advance state only when persisted data changes.
#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn upsert_message(
   pool: &Pool,
   account_id: &str,
   env: &MessageEnvelope,
) -> Result<UpsertOutcome> {
   let flags_json = serde_json::to_string(&env.flags).unwrap_or_else(|_| "[]".into());
   let to_opt_json = |value: &Option<Vec<EmailAddress>>| {
      value
         .as_ref()
         .map(|item| serde_json::to_string(item).unwrap_or_default())
   };
   let from_s = to_opt_json(&env.from);
   let to_s = to_opt_json(&env.to);
   let cc_s = to_opt_json(&env.cc);
   let bcc_s = to_opt_json(&env.bcc);
   let reply_to_s = to_opt_json(&env.reply_to);

   let client = client(pool).await?;
   let existing = queries::messages::get_envelope_for_compare()
      .bind(&client, &account_id, &env.msgid.as_str())
      .opt()
      .await?;
   let preview = env.preview.clone().or_else(|| {
      let row = existing.as_ref()?;
      row.preview.clone()
   });
   let has_attachment =
      env.has_attachment || existing.as_ref().is_some_and(|row| row.has_attachment != 0);

   let outcome = if let Some(cur) = existing.as_ref() {
      let same = cur.thrid == env.thrid
         && cur.flags_json == flags_json
         && cur.received_at == env.received_at.timestamp()
         && cur.sent_at == env.sent_at.map(|timestamp| timestamp.timestamp())
         && cur.size == env.size as i64
         && cur.from_json == from_s
         && cur.to_json == to_s
         && cur.cc_json == cc_s
         && cur.bcc_json == bcc_s
         && cur.reply_to_json == reply_to_s
         && cur.subject == env.subject
         && cur.preview == preview
         && cur.has_attachment == i64::from(has_attachment)
         && cur.message_id_header == env.message_id_header
         && cur.in_reply_to_header == env.in_reply_to_header
         && cur.references_header == env.references_header;
      if same {
         return Ok(UpsertOutcome::Unchanged);
      }
      UpsertOutcome::Updated
   } else {
      UpsertOutcome::Inserted
   };

   let modseq = bump_modseq(pool, account_id, StateKind::Email).await?;

   queries::messages::upsert_message()
      .bind(
         &client,
         &account_id,
         &env.msgid.as_str(),
         &env.thrid.as_str(),
         &flags_json.as_str(),
         &env.received_at.timestamp(),
         &env.sent_at.map(|timestamp| timestamp.timestamp()),
         &(env.size as i64),
         &from_s.as_deref(),
         &to_s.as_deref(),
         &cc_s.as_deref(),
         &bcc_s.as_deref(),
         &reply_to_s.as_deref(),
         &env.subject.as_deref(),
         &preview.as_deref(),
         &i64::from(has_attachment),
         &env.message_id_header.as_deref(),
         &env.in_reply_to_header.as_deref(),
         &env.references_header.as_deref(),
         &(modseq as i64),
      )
      .await?;
   Ok(outcome)
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn set_message_mailboxes(
   pool: &Pool,
   account_id: &str,
   msgid: &str,
   mailbox_ids: &[String],
) -> Result<()> {
   let mut client = client(pool).await?;
   let tx = client.transaction().await?;
   queries::messages::clear_message_mailboxes()
      .bind(&tx, &account_id, &msgid)
      .await?;
   for mbid in mailbox_ids {
      queries::messages::add_message_mailbox()
         .bind(&tx, &account_id, &msgid, &mbid.as_str())
         .await?;
   }
   tx.commit().await?;
   Ok(())
}

/// Upsert a folder placement, evicting any occupant of the UID slot.
///
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn upsert_message_imap(
   pool: &Pool,
   account_id: &str,
   msgid: &str,
   folder_id: i64,
   uid: u32,
   uidvalidity: u32,
   mailbox_id: &str,
) -> Result<()> {
   let mut conn = client(pool).await?;
   let tx = conn.transaction().await?;
   let occupants = queries::messages::msgid_for_folder_uid()
      .bind(&tx, &account_id, &folder_id, &i64::from(uid))
      .all()
      .await?;
   let stale = occupants
      .into_iter()
      .filter(|occupant| occupant != msgid)
      .collect::<Vec<String>>();
   if !stale.is_empty() {
      queries::messages::delete_message_imap_by_uid()
         .bind(&tx, &account_id, &folder_id, &i64::from(uid))
         .await?;
      for old_msgid in &stale {
         remove_folder_placement(&tx, account_id, old_msgid, mailbox_id).await?;
      }
   }
   queries::messages::upsert_message_imap()
      .bind(
         &tx,
         &account_id,
         &msgid,
         &folder_id,
         &i64::from(uid),
         &i64::from(uidvalidity),
      )
      .await?;
   tx.commit().await?;
   Ok(())
}

/// Clean up after removing a message's folder placement.
///
/// # Errors
///
/// Returns an error when the database operation fails.
pub(crate) async fn remove_folder_placement(
   tx: &deadpool_postgres::Transaction<'_>,
   account_id: &str,
   msgid: &str,
   mailbox_id: &str,
) -> Result<()> {
   let still_elsewhere = queries::messages::count_message_imap()
      .bind(tx, &account_id, &msgid)
      .one()
      .await?;
   if still_elsewhere == 0 {
      // CASCADE clears mailbox and raw-message rows.
      queries::state::bump_email_modseq()
         .bind(tx, &account_id)
         .one()
         .await?;
      queries::messages::delete_message()
         .bind(tx, &account_id, &msgid)
         .await?;
   } else {
      // Advance the row modseq so Email/changes exposes the mailbox removal.
      queries::messages::remove_message_mailbox()
         .bind(tx, &account_id, &msgid, &mailbox_id)
         .await?;
      let new_modseq = queries::state::bump_email_modseq()
         .bind(tx, &account_id)
         .one()
         .await?;
      queries::messages::set_message_modseq()
         .bind(tx, &new_modseq, &account_id, &msgid)
         .await?;
   }
   Ok(())
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn get_message_imap(
   pool: &Pool,
   account_id: &str,
   msgid: &str,
   folder_id: Option<i64>,
) -> Result<Option<(i64, u32, u32)>> {
   let client = client(pool).await?;
   let row = match folder_id {
      Some(fid) => {
         queries::messages::get_message_imap_in_folder()
            .bind(&client, &account_id, &msgid, &fid)
            .opt()
            .await?
      },
      None => {
         queries::messages::get_message_imap_any()
            .bind(&client, &account_id, &msgid)
            .opt()
            .await?
      },
   };
   Ok(row.map(|row| (row.folder_id, row.uid as u32, row.uidvalidity as u32)))
}

// ================= State / modseq =================

#[derive(Debug, Clone, Copy)]
pub enum StateKind {
   Email,
   Mailbox,
   Submission,
}

/// Mark an account's initial sync as complete. Called exactly once per
/// account (idempotent on repeat), after the first `initial_sync` pass
/// walks every synced folder. Drives the `/readyz` signal.
#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn mark_initial_sync_done(pool: &Pool, account_id: &str) -> Result<()> {
   let client = client(pool).await?;
   queries::state::mark_initial_sync_done()
      .bind(&client, &account_id)
      .await?;
   Ok(())
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn bump_modseq(pool: &Pool, account_id: &str, kind: StateKind) -> Result<u64> {
   let client = client(pool).await?;
   let modseq = match kind {
      StateKind::Email => {
         queries::state::bump_email_modseq()
            .bind(&client, &account_id)
            .one()
            .await?
      },
      StateKind::Mailbox => {
         queries::state::bump_mailbox_modseq()
            .bind(&client, &account_id)
            .one()
            .await?
      },
      StateKind::Submission => {
         queries::state::bump_submission_modseq()
            .bind(&client, &account_id)
            .one()
            .await?
      },
   };
   Ok(modseq as u64)
}

#[derive(Debug, Clone, Copy)]
pub struct StateRow {
   pub email_modseq:      i64,
   pub mailbox_modseq:    i64,
   pub submission_modseq: i64,
}

impl StateRow {
   #[must_use]
   pub const fn modseq(&self, kind: StateKind) -> i64 {
      match kind {
         StateKind::Email => self.email_modseq,
         StateKind::Mailbox => self.mailbox_modseq,
         StateKind::Submission => self.submission_modseq,
      }
   }
}

#[inline]
/// # Errors
///
/// Returns an error when the database operation fails.
pub async fn get_state(pool: &Pool, account_id: &str) -> Result<StateRow> {
   let client = client(pool).await?;
   let row = queries::state::get_state()
      .bind(&client, &account_id)
      .one()
      .await?;
   Ok(StateRow {
      email_modseq:      row.email_modseq,
      mailbox_modseq:    row.mailbox_modseq,
      submission_modseq: row.submission_modseq,
   })
}

#[cfg(test)]
mod tests {
   use super::*;
   use crate::testkit;

   #[tokio::test]
   async fn envelope_refresh_preserves_body_metadata() {
      let Some(pool) = testkit::test_pool().await else {
         return;
      };
      upsert_account(&pool, "a", "a@example.test", ProviderKind::Imap, "A", b"h")
         .await
         .unwrap();
      let envelope = MessageEnvelope {
         msgid:              "m1".into(),
         thrid:              "t1".into(),
         flags:              vec![],
         received_at:        chrono::Utc::now(),
         sent_at:            None,
         size:               42,
         from:               None,
         to:                 None,
         cc:                 None,
         bcc:                None,
         reply_to:           None,
         subject:            Some("subject".into()),
         preview:            None,
         has_attachment:     false,
         message_id_header:  None,
         in_reply_to_header: None,
         references_header:  None,
      };
      upsert_message(&pool, "a", &envelope).await.unwrap();
      queries::messages::update_message_body_cache()
         .bind(
            &client(&pool).await.unwrap(),
            &"cached preview",
            &1_i64,
            &0_i64,
            &"a",
            &"m1",
         )
         .await
         .unwrap();
      let before = get_state(&pool, "a").await.unwrap().email_modseq;

      assert_eq!(
         upsert_message(&pool, "a", &envelope).await.unwrap(),
         UpsertOutcome::Unchanged
      );
      let row = queries::messages::message_body_metadata()
         .bind(&client(&pool).await.unwrap(), &"a", &"m1")
         .one()
         .await
         .unwrap();
      assert_eq!(row.preview.as_deref(), Some("cached preview"));
      assert_eq!(row.has_attachment, 1);
      assert_eq!(get_state(&pool, "a").await.unwrap().email_modseq, before);
   }

   #[tokio::test]
   async fn unchanged_mailbox_does_not_advance_state() {
      let Some(pool) = testkit::test_pool().await else {
         return;
      };
      upsert_account(&pool, "a", "a@example.test", ProviderKind::Imap, "A", b"h")
         .await
         .unwrap();
      upsert_mailbox(&pool, "inbox", "a", "Inbox", None, Some("inbox"), 0)
         .await
         .unwrap();
      let before = get_state(&pool, "a").await.unwrap().mailbox_modseq;

      upsert_mailbox(&pool, "inbox", "a", "Inbox", None, Some("inbox"), 0)
         .await
         .unwrap();
      assert_eq!(get_state(&pool, "a").await.unwrap().mailbox_modseq, before);

      upsert_mailbox(&pool, "inbox", "a", "Renamed", None, Some("inbox"), 0)
         .await
         .unwrap();
      assert_eq!(
         get_state(&pool, "a").await.unwrap().mailbox_modseq,
         before + 1,
      );
   }

   #[tokio::test]
   async fn mailbox_counts_distinct_threads() {
      let Some(pool) = testkit::test_pool().await else {
         return;
      };
      let client = pool.get().await.unwrap();
      client
         .batch_execute(
            r#"
            INSERT INTO accounts VALUES ('a', 'a@example.test', 'imap', 'A', '\x00', 0);
            INSERT INTO state (account_id) VALUES ('a');
            INSERT INTO mailboxes (id, account_id, name) VALUES ('inbox', 'a', 'Inbox');
            INSERT INTO messages (account_id, msgid, thrid, flags_json, received_at, size, modseq)
            VALUES
                ('a', 'm1', 'thread-a', '[]', 0, 1, 0),
                ('a', 'm2', 'thread-a', '[]', 0, 1, 0),
                ('a', 'm3', 'thread-b', '["$seen"]', 0, 1, 0);
            INSERT INTO message_mailboxes VALUES
                ('a', 'm1', 'inbox'),
                ('a', 'm2', 'inbox'),
                ('a', 'm3', 'inbox');
            "#,
         )
         .await
         .unwrap();
      drop(client);

      recompute_mailbox_counts(&pool, "a").await.unwrap();
      let row = list_mailboxes(&pool, "a").await.unwrap().pop().unwrap();
      let count_state = get_state(&pool, "a").await.unwrap().mailbox_modseq;
      assert_eq!(
         (
            row.total_emails,
            row.unread_emails,
            row.total_threads,
            row.unread_threads,
         ),
         (3, 2, 2, 1),
      );
      assert_eq!(row.modseq, count_state);

      recompute_mailbox_counts(&pool, "a").await.unwrap();
      assert_eq!(
         get_state(&pool, "a").await.unwrap().mailbox_modseq,
         count_state,
      );
   }
}
