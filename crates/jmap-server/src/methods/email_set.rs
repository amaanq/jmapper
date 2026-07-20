//! Email/set methods (RFC 8621 §4.6).

use std::{
   collections::{
      BTreeSet,
      HashMap,
   },
   time::Duration,
};

use futures_util::future;
use imap_sync::{
   account::AccountRequest,
   db::StateKind as DbStateKind,
   sync::{
      is_valid_keyword,
      keyword_to_imap_flag,
   },
};
use jmap_protocol::{
   email::EmailAddress,
   error::MethodError,
   ids::AccountId,
};
use tokio::{
   sync::oneshot,
   time,
};

use super::{
   MethodResult,
   bad_args,
   cached_state_row,
   enforce_set_limit,
   ids_or_null,
   object_or_null,
   publish_imap_state_changes,
   require_auth_match,
   state_value,
};
use crate::{
   mime::{
      Attachment,
      ComposeInput,
      build_rfc5322,
   },
   state::{
      AccountInfo,
      AppState,
   },
};

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments fail to deserialize, the
/// authenticated account does not match `accountId`, the create/update/destroy
/// batch exceeds the configured set limit, the cached state row cannot be read,
/// or `ifInState` does not match the current `Email` state
/// ([`MethodError::StateMismatch`]).
pub async fn set(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id:  AccountId,
      #[serde(default, rename = "ifInState")]
      if_in_state: Option<String>,
      #[serde(default)]
      create:      Option<HashMap<String, serde_json::Value>>,
      #[serde(default)]
      update:      Option<HashMap<String, serde_json::Value>>,
      #[serde(default)]
      destroy:     Option<Vec<String>>,
   }

   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid Email/set args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   enforce_set_limit(
      req.create.as_ref().map_or(0, HashMap::len),
      req.update.as_ref().map_or(0, HashMap::len),
      req.destroy.as_ref().map_or(0, Vec::len),
   )?;

   let before = cached_state_row(state, account_id).await?;
   let old_state = state_value(&before, DbStateKind::Email);
   if let Some(expected) = req.if_in_state.as_deref()
      && expected != old_state.as_ref()
   {
      // RFC 8620 §5.3: state mismatch is a method-level error, not a
      // per-entry SetError.
      return Err(MethodError::StateMismatch);
   }

   let mut created = serde_json::Map::new();
   let mut not_created = serde_json::Map::new();
   if let Some(create) = req.create {
      for (creation_id, payload) in create {
         match apply_create(state, account_id, &payload).await {
            Ok(server_set) => {
               created.insert(creation_id, server_set);
            },
            Err(set_error) => {
               not_created.insert(creation_id, set_error);
            },
         }
      }
   }
   let mut destroyed_ids = Vec::<String>::new();
   let mut not_destroyed = serde_json::Map::new();
   if let Some(destroy) = req.destroy {
      for id in destroy {
         match apply_destroy(state, account_id, &id).await {
            Ok(()) => destroyed_ids.push(id),
            Err(set_error) => {
               not_destroyed.insert(id, set_error);
            },
         }
      }
   }

   let mut updated = serde_json::Map::new();
   let mut not_updated = serde_json::Map::new();
   if let Some(update) = req.update {
      // Concurrent so the account task can coalesce same-target moves into
      // one IMAP pipeline.
      let results = future::join_all(update.iter().map(|(msgid, patch)| {
         async move {
            (
               msgid.clone(),
               apply_update(state, account_id, msgid, patch).await,
            )
         }
      }))
      .await;
      for (msgid, result) in results {
         match result {
            Ok(()) => {
               updated.insert(msgid, serde_json::Value::Null);
            },
            Err(set_error) => {
               not_updated.insert(msgid, set_error);
            },
         }
      }
   }

   let after = cached_state_row(state, account_id).await?;
   let new_state = state_value(&after, DbStateKind::Email);
   publish_imap_state_changes(state, account_id, &before, &after);
   Ok(serde_json::json!({
       "accountId": account_id,
       "oldState": old_state,
       "newState": new_state,
       "created": object_or_null(created),
       "updated": object_or_null(updated),
       "destroyed": ids_or_null(destroyed_ids),
       "notCreated": object_or_null(not_created),
       "notUpdated": object_or_null(not_updated),
       "notDestroyed": object_or_null(not_destroyed),
   }))
}

/// `Email/set create` — compose a message from the JMAP Email object and
/// APPEND it via the account task. Returns the server-set properties.
async fn apply_create(
   state: &AppState,
   account_id: &str,
   payload: &serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
   #[derive(serde::Deserialize)]
   struct BodyPartRef {
      #[serde(rename = "partId")]
      part_id:    Option<String>,
      #[serde(rename = "type", default)]
      _mime_type: Option<String>,
   }
   #[derive(serde::Deserialize)]
   struct BodyValue {
      value: String,
   }
   #[derive(serde::Deserialize)]
   struct AttachmentRef {
      #[serde(rename = "blobId")]
      blob_id:   String,
      #[serde(rename = "type", default)]
      mime_type: Option<String>,
      #[serde(default)]
      name:      Option<String>,
   }
   #[derive(Default, serde::Deserialize)]
   #[serde(rename_all = "camelCase")]
   struct CreateEmail {
      mailbox_ids:    HashMap<String, bool>,
      #[serde(default)]
      keywords:       HashMap<String, bool>,
      #[serde(default)]
      from:           Option<Vec<EmailAddress>>,
      #[serde(default)]
      to:             Option<Vec<EmailAddress>>,
      #[serde(default)]
      cc:             Option<Vec<EmailAddress>>,
      #[serde(default)]
      bcc:            Option<Vec<EmailAddress>>,
      #[serde(default)]
      reply_to:       Option<Vec<EmailAddress>>,
      #[serde(default)]
      subject:        Option<String>,
      #[serde(default)]
      in_reply_to:    Option<Vec<String>>,
      #[serde(default)]
      references:     Option<Vec<String>>,
      #[serde(default)]
      sent_at:        Option<chrono::DateTime<chrono::Utc>>,
      #[serde(default)]
      received_at:    Option<chrono::DateTime<chrono::Utc>>,
      #[serde(default)]
      body_values:    HashMap<String, BodyValue>,
      #[serde(default)]
      text_body:      Vec<BodyPartRef>,
      #[serde(default)]
      html_body:      Vec<BodyPartRef>,
      #[serde(default)]
      attachments:    Vec<AttachmentRef>,
      // Composed messages must use the simple textBody/htmlBody shape; an
      // explicit bodyStructure could describe nestings the builder can't
      // reproduce, and silently flattening it would corrupt the message.
      #[serde(default)]
      body_structure: Option<serde_json::Value>,
      #[serde(default, rename = "messageId")]
      message_id:     Option<Vec<String>>,
   }

   let invalid = |prop: &str, msg: String| {
      serde_json::json!({
          "type": "invalidProperties",
          "properties": [prop],
          "description": msg,
      })
   };
   let email = serde_json::from_value::<CreateEmail>(payload.clone()).map_err(|err| {
      serde_json::json!({
          "type": "invalidProperties",
          "description": format!("invalid Email create object: {err}"),
      })
   })?;
   if email.body_structure.is_some() {
      return Err(invalid(
         "bodyStructure",
         "use textBody/htmlBody + attachments; explicit bodyStructure is not supported".into(),
      ));
   }

   let mailbox_ids = email
      .mailbox_ids
      .iter()
      .filter(|(_, value)| **value)
      .map(|(key, _)| key.clone())
      .collect::<Vec<String>>();
   if mailbox_ids.len() != 1 {
      return Err(invalid(
         "mailboxIds",
         "Email/set create requires exactly one target mailbox".into(),
      ));
   }
   let mailbox_id = mailbox_ids.into_iter().next().unwrap();

   let mut flags = Vec::<String>::new();
   for (key, value) in &email.keywords {
      if !is_valid_keyword(key) {
         return Err(invalid("keywords", format!("invalid keyword {key:?}")));
      }
      if *value {
         flags.push(keyword_to_imap_flag(key));
      }
   }

   let body_of = |refs: &[BodyPartRef], prop: &str| -> Result<Option<String>, serde_json::Value> {
      match refs {
         [] => Ok(None),
         [one] => {
            let part_id = one
               .part_id
               .as_deref()
               .ok_or_else(|| invalid(prop, "body part must reference a partId".into()))?;
            email
               .body_values
               .get(part_id)
               .map(|bv| Some(bv.value.clone()))
               .ok_or_else(|| invalid(prop, format!("partId {part_id:?} missing from bodyValues")))
         },
         _ => Err(invalid(prop, "at most one part per body kind".into())),
      }
   };
   let text_body = body_of(&email.text_body, "textBody")?;
   let html_body = body_of(&email.html_body, "htmlBody")?;

   let mut attachments = Vec::with_capacity(email.attachments.len());
   for att in &email.attachments {
      // Only freshly-uploaded blobs; message-part blob ids would need the
      // source message's raw bytes which may not be cached. Clients
      // forwarding attachments re-upload them.
      let row = state
         .pool()
         .get()
         .await
         .map_err(
            |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
         )?
         .query_opt(
            "SELECT bytes, content_type FROM uploaded_blobs WHERE account_id = $1 AND blob_id = \
             $2 AND expires_at > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT",
            &[&account_id, &att.blob_id],
         )
         .await
         .map_err(
            |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
         )?
         .map(|row| (row.get::<_, Vec<u8>>(0), row.get::<_, String>(1)));
      let Some((bytes, stored_type)) = row else {
         return Err(serde_json::json!({
             "type": "blobNotFound",
             "description": format!(
                 "attachment blobId {:?} is not an uploaded blob (upload attachments via POST /upload)",
                 att.blob_id
             ),
         }));
      };
      attachments.push(Attachment {
         bytes,
         content_type: att.mime_type.clone().unwrap_or(stored_type),
         name: att.name.clone(),
      });
   }

   let sent_at = email.sent_at.unwrap_or_else(chrono::Utc::now);
   let message_id = match email.message_id.as_ref().and_then(|value| value.first()) {
      Some(id) if !id.trim().is_empty() => id.trim().trim_matches(['<', '>']).to_owned(),
      _ => {
         use sha2::{
            Digest as _,
            Sha256,
         };
         let mut hasher = Sha256::new();
         hasher.update(account_id.as_bytes());
         hasher.update(payload.to_string().as_bytes());
         hasher.update(
            chrono::Utc::now()
               .timestamp_nanos_opt()
               .unwrap_or(0)
               .to_be_bytes(),
         );
         format!("{}@jmapper", hex::encode(&hasher.finalize()[..16]))
      },
   };

   let input = ComposeInput {
      from: email.from.unwrap_or_default(),
      to: email.to.unwrap_or_default(),
      cc: email.cc.unwrap_or_default(),
      bcc: email.bcc.unwrap_or_default(),
      reply_to: email.reply_to.unwrap_or_default(),
      subject: email.subject,
      in_reply_to: email.in_reply_to.unwrap_or_default(),
      references: email.references.unwrap_or_default(),
      sent_at,
      message_id,
      text_body,
      html_body,
      attachments,
   };
   let bytes = build_rfc5322(&input);
   let size = bytes.len();

   // Stage exactly like POST /upload so the import path needs no new code.
   let blob_id = {
      use sha2::{
         Digest as _,
         Sha256,
      };
      let mut hasher = Sha256::new();
      hasher.update(&bytes);
      format!("blob-upload-{}", hex::encode(hasher.finalize()))
   };
   let now = chrono::Utc::now().timestamp();
   state
      .pool()
      .get()
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?
      .execute(
         "INSERT INTO uploaded_blobs (account_id, blob_id, content_type, bytes, uploaded_at, \
          expires_at) VALUES ($1, $2, 'message/rfc822', $3, $4, $5) ON CONFLICT (account_id, \
          blob_id) DO UPDATE SET uploaded_at = EXCLUDED.uploaded_at, expires_at = \
          EXCLUDED.expires_at",
         &[&account_id, &blob_id, &bytes, &now, &(now + 3600)],
      )
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?;

   let tx = state.account_sender(account_id).ok_or_else(|| {
      serde_json::json!({
          "type": "serverUnavailable",
          "description": "sync task is not running for this account",
      })
   })?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::ImportMessage {
      blob_id,
      mailbox_id,
      flags,
      received_at_secs: email.received_at.map(|dt| dt.timestamp()),
      respond,
   })
   .await
   .map_err(
      |_| serde_json::json!({"type": "serverFail", "description": "account task channel closed"}),
   )?;
   let outcome = time::timeout(Duration::from_secs(30), rx)
        .await
        .map_err(|_| serde_json::json!({"type": "serverFail", "description": "create timed out"}))?
        .map_err(|_| {
            serde_json::json!({"type": "serverFail", "description": "account task dropped create"})
        })?
        .map_err(|err| {
            serde_json::json!({"type": "serverFail", "description": err.to_string()})
        })?;

   Ok(serde_json::json!({
       "id": outcome.msgid,
       "blobId": format!("blob-{}", outcome.msgid),
       "threadId": outcome.thread_id,
       "size": size,
   }))
}

pub(crate) async fn apply_update(
   state: &AppState,
   account_id: &str,
   msgid: &str,
   patch: &serde_json::Value,
) -> Result<(), serde_json::Value> {
   let current = load_current(state, account_id, msgid).await?;
   let Some(Loaded { flags, mailboxes }) = current else {
      return Err(serde_json::json!({"type": "notFound"}));
   };

   let plan = compute_update_plan(&flags, &mailboxes, patch)?;

   let touches_flags = !plan.flag_add.is_empty() || !plan.flag_remove.is_empty();
   let touches_mailboxes = !plan.mailbox_add.is_empty() || !plan.mailbox_remove.is_empty();

   // A mixed patch dispatches two AccountRequest ops that aren't atomic on
   // the IMAP side. Flags go first (idempotent, cheap to reconcile); if the
   // mailbox half then fails we report exactly which half applied instead
   // of pretending the whole patch failed. Real clients send mixed patches
   // routinely (the post-send "move draft to sent + clear $draft" update),
   // so rejecting them outright is not an option.
   if touches_flags {
      send_store_flags(state, account_id, msgid, plan.flag_add, plan.flag_remove).await?;
   }
   if touches_mailboxes {
      send_mutate_mailboxes(
         state,
         account_id,
         msgid,
         plan.mailbox_add,
         plan.mailbox_remove,
      )
      .await
      .map_err(|err| {
         if touches_flags {
            serde_json::json!({
                "type": "serverFail",
                "description": format!(
                    "keywords were applied but the mailboxIds change failed: {}",
                    err.get("description").and_then(|desc| desc.as_str()).unwrap_or("unknown")
                ),
            })
         } else {
            err
         }
      })?;
   }
   Ok(())
}

pub(crate) async fn apply_destroy(
   state: &AppState,
   account_id: &str,
   msgid: &str,
) -> Result<(), serde_json::Value> {
   let tx = state.account_sender(account_id).ok_or_else(|| {
      serde_json::json!({
          "type": "serverUnavailable",
          "description": "sync task is not running for this account",
      })
   })?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::DestroyMessage {
      msgid: msgid.to_owned(),
      respond,
   })
   .await
   .map_err(
      |_| serde_json::json!({"type": "serverFail", "description": "account task channel closed"}),
   )?;
   let outcome = time::timeout(Duration::from_secs(15), rx)
        .await
        .map_err(|_| {
            serde_json::json!({"type": "serverFail", "description": "destroy timed out"})
        })?
        .map_err(|_| {
            serde_json::json!({"type": "serverFail", "description": "account task dropped destroy"})
        })?;
   outcome.map_err(|err| serde_json::json!({"type": "serverFail", "description": err.to_string()}))
}

/// Current membership + keyword view of a message, loaded from the cache.
struct Loaded {
   flags:     BTreeSet<String>,
   mailboxes: BTreeSet<String>,
}

async fn load_current(
   state: &AppState,
   account_id: &str,
   msgid: &str,
) -> Result<Option<Loaded>, serde_json::Value> {
   let conn = state.pool().get().await.map_err(
      |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
   )?;
   let row = conn
      .query_opt(
         "SELECT flags_json FROM messages WHERE account_id = $1 AND msgid = $2",
         &[&account_id, &msgid],
      )
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?
      .map(|row| row.get::<_, String>(0));
   let Some(flags_json) = row else {
      return Ok(None);
   };
   let mb_ids = conn
      .query(
         "SELECT mailbox_id FROM message_mailboxes WHERE account_id = $1 AND msgid = $2",
         &[&account_id, &msgid],
      )
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?
      .into_iter()
      .map(|row| row.get::<_, String>(0))
      .collect::<Vec<String>>();
   let flags = serde_json::from_str::<Vec<String>>(&flags_json).unwrap_or_default();
   Ok(Some(Loaded {
      flags:     flags.into_iter().collect(),
      mailboxes: mb_ids.into_iter().collect(),
   }))
}

/// Computed patch application: which IMAP flags to add/remove and which
/// mailboxes to add/remove. Pure — unit-tested below.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct UpdatePlan {
   pub flag_add:       Vec<String>,
   pub flag_remove:    Vec<String>,
   pub mailbox_add:    Vec<String>,
   pub mailbox_remove: Vec<String>,
}

pub(crate) fn compute_update_plan(
   current_keywords: &BTreeSet<String>,
   current_mailboxes: &BTreeSet<String>,
   patch: &serde_json::Value,
) -> Result<UpdatePlan, serde_json::Value> {
   let obj = patch.as_object().ok_or_else(
      || serde_json::json!({"type": "invalidPatch", "description": "patch must be an object"}),
   )?;

   let mut desired_keywords = current_keywords.clone();
   let mut desired_mailboxes = current_mailboxes.clone();
   let mut touched_keywords = false;
   let mut touched_mailboxes = false;

   for (key, val) in obj {
      if key == "keywords" {
         let map = val.as_object().ok_or_else(|| {
            serde_json::json!({
                "type": "invalidPatch",
                "description": "keywords must be an object of $flag→bool",
            })
         })?;
         // Validate every requested keyword name. Reject the whole
         // patch if any is malformed — silently dropping a bad
         // keyword could leak data (e.g. accept `bad) \Deleted (`
         // and run UID STORE +FLAGS (bad) \Deleted ()).
         for key in map.keys() {
            if !is_valid_keyword(key) {
               return Err(serde_json::json!({
                   "type": "invalidProperties",
                   "properties": ["keywords"],
                   "description": format!("invalid keyword {key:?}"),
               }));
            }
         }
         desired_keywords = map
            .iter()
            .filter(|&(_k, value)| value.as_bool().unwrap_or(false))
            .map(|(key, _v)| key.clone())
            .collect();
         touched_keywords = true;
      } else if let Some(flag) = key.strip_prefix("keywords/") {
         if !is_valid_keyword(flag) {
            return Err(serde_json::json!({
                "type": "invalidProperties",
                "properties": [key.clone()],
                "description": format!("invalid keyword {flag:?}"),
            }));
         }
         if val.as_bool().unwrap_or(false) {
            desired_keywords.insert(flag.to_owned());
         } else {
            desired_keywords.remove(flag);
         }
         touched_keywords = true;
      } else if key == "mailboxIds" {
         let map = val.as_object().ok_or_else(|| {
            serde_json::json!({
                "type": "invalidPatch",
                "description": "mailboxIds must be an object of {id→true}",
            })
         })?;
         desired_mailboxes = map
            .iter()
            .filter(|&(_k, value)| value.as_bool().unwrap_or(false))
            .map(|(key, _v)| key.clone())
            .collect();
         touched_mailboxes = true;
      } else if let Some(mb_id) = key.strip_prefix("mailboxIds/") {
         if val.as_bool().unwrap_or(false) {
            desired_mailboxes.insert(mb_id.to_owned());
         } else {
            desired_mailboxes.remove(mb_id);
         }
         touched_mailboxes = true;
      } else {
         return Err(serde_json::json!({
             "type": "invalidProperties",
             "properties": [key],
             "description": format!("Email/set update cannot change property {key} yet"),
         }));
      }
   }

   let mut plan = UpdatePlan::default();
   if touched_keywords {
      plan.flag_add = desired_keywords
         .difference(current_keywords)
         .map(|key| keyword_to_imap_flag(key))
         .collect();
      plan.flag_remove = current_keywords
         .difference(&desired_keywords)
         .map(|key| keyword_to_imap_flag(key))
         .collect();
   }
   if touched_mailboxes {
      // RFC 8621 §4.6.4 forbids removing a message from every mailbox;
      // mailboxIds: {} is a real no-op from the client's perspective
      // (they'd use destroy). Fail fast rather than dispatch a move
      // with an empty add set.
      if desired_mailboxes.is_empty() {
         return Err(serde_json::json!({
             "type": "invalidProperties",
             "properties": ["mailboxIds"],
             "description": "mailboxIds cannot be empty; use destroy to delete",
         }));
      }
      plan.mailbox_add = desired_mailboxes
         .difference(current_mailboxes)
         .cloned()
         .collect();
      plan.mailbox_remove = current_mailboxes
         .difference(&desired_mailboxes)
         .cloned()
         .collect();
   }
   Ok(plan)
}

async fn send_store_flags(
   state: &AppState,
   account_id: &str,
   msgid: &str,
   add: Vec<String>,
   remove: Vec<String>,
) -> Result<(), serde_json::Value> {
   if add.is_empty() && remove.is_empty() {
      return Ok(());
   }
   let tx = state.account_sender(account_id).ok_or_else(|| {
      serde_json::json!({
          "type": "serverUnavailable",
          "description": "sync task is not running for this account",
      })
   })?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::StoreFlags {
      msgid: msgid.to_owned(),
      add,
      remove,
      respond,
   })
   .await
   .map_err(
      |_| serde_json::json!({"type": "serverFail", "description": "account task channel closed"}),
   )?;
   let outcome = time::timeout(Duration::from_secs(10), rx)
      .await
      .map_err(|_| serde_json::json!({"type": "serverFail", "description": "store timed out"}))?
      .map_err(
         |_| serde_json::json!({"type": "serverFail", "description": "account task dropped store"}),
      )?;
   outcome.map_err(|err| serde_json::json!({"type": "serverFail", "description": err.to_string()}))
}

async fn send_mutate_mailboxes(
   state: &AppState,
   account_id: &str,
   msgid: &str,
   add: Vec<String>,
   remove: Vec<String>,
) -> Result<(), serde_json::Value> {
   let tx = state.account_sender(account_id).ok_or_else(|| {
      serde_json::json!({
          "type": "serverUnavailable",
          "description": "sync task is not running for this account",
      })
   })?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::MutateMailboxes {
      msgid: msgid.to_owned(),
      add,
      remove,
      respond,
   })
   .await
   .map_err(
      |_| serde_json::json!({"type": "serverFail", "description": "account task channel closed"}),
   )?;
   let outcome = time::timeout(Duration::from_secs(30), rx)
      .await
      .map_err(|_| serde_json::json!({"type": "serverFail", "description": "move timed out"}))?
      .map_err(
         |_| serde_json::json!({"type": "serverFail", "description": "account task dropped move"}),
      )?;
   outcome.map_err(|err| serde_json::json!({"type": "serverFail", "description": err.to_string()}))
}

#[cfg(test)]
mod tests {
   use serde_json::json;

   use super::*;

   fn set_of<'a>(items: impl IntoIterator<Item = &'a str>) -> BTreeSet<String> {
      items.into_iter().map(ToOwned::to_owned).collect()
   }

   #[test]
   fn full_replace_keywords() {
      let current = set_of(["$seen", "$flagged"]);
      let patch = json!({"keywords": {"$seen": true}});
      let plan = compute_update_plan(&current, &BTreeSet::new(), &patch).unwrap();
      assert!(plan.flag_add.is_empty(), "{plan:?}");
      assert_eq!(plan.flag_remove, vec![r"\Flagged"]);
   }

   #[test]
   fn pointer_keyword_patch() {
      let current = set_of(["$flagged"]);
      let patch = json!({"keywords/$seen": true, "keywords/$flagged": false});
      let plan = compute_update_plan(&current, &BTreeSet::new(), &patch).unwrap();
      assert_eq!(plan.flag_add, vec![r"\Seen"]);
      assert_eq!(plan.flag_remove, vec![r"\Flagged"]);
   }

   #[test]
   fn empty_patch_is_noop() {
      let plan = compute_update_plan(&set_of(["$seen"]), &set_of(["mb1"]), &json!({})).unwrap();
      assert_eq!(plan, UpdatePlan::default());
   }

   #[test]
   fn rejects_unknown_property() {
      let err = compute_update_plan(
         &BTreeSet::new(),
         &BTreeSet::new(),
         &json!({"subject": "foo"}),
      )
      .unwrap_err();
      assert_eq!(err["type"], "invalidProperties");
      assert_eq!(err["properties"][0], "subject");
   }

   #[test]
   fn rejects_bad_shape() {
      let err = compute_update_plan(&BTreeSet::new(), &BTreeSet::new(), &json!([])).unwrap_err();
      assert_eq!(err["type"], "invalidPatch");
   }

   #[test]
   fn mailbox_pointer_patch() {
      let current = set_of(["mb-inbox"]);
      let patch = json!({"mailboxIds/mb-archive": true, "mailboxIds/mb-inbox": false});
      let plan = compute_update_plan(&BTreeSet::new(), &current, &patch).unwrap();
      assert_eq!(plan.mailbox_add, vec!["mb-archive"]);
      assert_eq!(plan.mailbox_remove, vec!["mb-inbox"]);
   }

   #[test]
   fn mailbox_full_replace() {
      let current = set_of(["mb-inbox"]);
      let patch = json!({"mailboxIds": {"mb-archive": true}});
      let plan = compute_update_plan(&BTreeSet::new(), &current, &patch).unwrap();
      assert_eq!(plan.mailbox_add, vec!["mb-archive"]);
      assert_eq!(plan.mailbox_remove, vec!["mb-inbox"]);
   }

   #[test]
   fn keyword_injection_attempt_rejected() {
      // Audit fix (#5): an unvalidated keyword name reaches IMAP STORE
      // verbatim. Reject malformed keywords at the patch boundary.
      let current = set_of([]);
      let patch = json!({"keywords": {"bad) \\Deleted (": true}});
      let err = compute_update_plan(&current, &BTreeSet::new(), &patch).unwrap_err();
      assert_eq!(err["type"], "invalidProperties");

      let patch_pointer = json!({"keywords/bad space": true});
      let err = compute_update_plan(&current, &BTreeSet::new(), &patch_pointer).unwrap_err();
      assert_eq!(err["type"], "invalidProperties");
   }

   #[test]
   fn empty_mailbox_set_rejected() {
      let current = set_of(["mb-inbox"]);
      let patch = json!({"mailboxIds": {}});
      let err = compute_update_plan(&BTreeSet::new(), &current, &patch).unwrap_err();
      assert_eq!(err["type"], "invalidProperties");
      assert_eq!(err["properties"][0], "mailboxIds");
   }
}
