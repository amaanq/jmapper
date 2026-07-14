//! Email/import methods (RFC 8621 §4.8).

use std::{
   collections::HashMap,
   time::Duration,
};

use imap_sync::{
   account::AccountRequest,
   db::StateKind as DbStateKind,
   sync,
};
use jmap_protocol::{
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
   object_or_null,
   publish_imap_state_changes,
   require_auth_match,
   server_fail,
   state_value,
};
use crate::state::{
   AccountInfo,
   AppState,
};

/// # Errors
///
/// Returns a `bad_args` error if the arguments fail to deserialize, an auth
/// error if `auth` does not match the requested account, a limit error if too
/// many emails are requested, [`MethodError::StateMismatch`] if `ifInState`
/// does not match the current state, and a `server_fail` error if reading the
/// cached state or validating an uploaded blob fails.
///
/// # Panics
///
/// Panics if the invariant that a validated entry has exactly one target
/// mailbox is violated after the length check.
pub async fn import(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct ImportEntry {
      #[serde(rename = "blobId")]
      blob_id:     String,
      #[serde(rename = "mailboxIds")]
      mailbox_ids: HashMap<String, bool>,
      #[serde(default)]
      keywords:    Option<HashMap<String, bool>>,
      #[serde(default, rename = "receivedAt")]
      received_at: Option<chrono::DateTime<chrono::Utc>>,
   }
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id:  AccountId,
      #[serde(default, rename = "ifInState")]
      if_in_state: Option<String>,
      #[serde(default)]
      emails:      Option<HashMap<String, ImportEntry>>,
   }

   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid Email/import args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   enforce_set_limit(req.emails.as_ref().map_or(0, HashMap::len), 0, 0)?;

   let before = cached_state_row(state, account_id).await?;
   let old_state = state_value(&before, DbStateKind::Email);
   if let Some(expected) = req.if_in_state.as_deref()
      && expected != old_state.as_ref()
   {
      return Err(MethodError::StateMismatch);
   }

   let mut created = serde_json::Map::new();
   let mut not_created = serde_json::Map::new();

   if let Some(emails) = req.emails {
      for (client_id, entry) in emails {
         let mailbox_ids = entry
            .mailbox_ids
            .iter()
            .filter(|&(_key, value)| *value)
            .map(|(key, _value)| key.clone())
            .collect::<Vec<String>>();
         if mailbox_ids.len() != 1 {
            not_created.insert(
               client_id,
               serde_json::json!({
                   "type": "invalidProperties",
                   "properties": ["mailboxIds"],
                   "description": "Email/import requires exactly one target mailbox",
               }),
            );
            continue;
         }
         let mailbox_id = mailbox_ids.into_iter().next().unwrap();

         // Validate keyword names — same security gate as Email/set.
         let mut bad_keyword = None::<String>;
         let flags = entry
            .keywords
            .map(|kw| {
               kw.into_iter()
                  .filter_map(|(key, value)| {
                     if !sync::is_valid_keyword(&key) {
                        bad_keyword.get_or_insert_with(|| key.clone());
                        return None;
                     }
                     value.then(|| sync::keyword_to_imap_flag(&key))
                  })
                  .collect::<Vec<String>>()
            })
            .unwrap_or_default();
         if let Some(bad) = bad_keyword {
            not_created.insert(
               client_id,
               serde_json::json!({
                   "type": "invalidProperties",
                   "properties": ["keywords"],
                   "description": format!("invalid keyword {bad:?}"),
               }),
            );
            continue;
         }

         let Some(tx) = state.account_sender(account_id) else {
            not_created.insert(
               client_id,
               serde_json::json!({
                   "type": "serverUnavailable",
                   "description": "sync task is not running",
               }),
            );
            continue;
         };
         if !uploaded_blob_exists(state, account_id, entry.blob_id.as_ref()).await? {
            not_created.insert(
               client_id,
               serde_json::json!({
                   "type": "blobNotFound",
                   "description": format!(
                       "blobId {:?} does not exist or has expired",
                       entry.blob_id
                   ),
               }),
            );
            continue;
         }
         let (respond, rx) = oneshot::channel();
         if tx
            .send(AccountRequest::ImportMessage {
               blob_id: entry.blob_id.clone(),
               mailbox_id,
               flags,
               received_at_secs: entry.received_at.map(|datetime| datetime.timestamp()),
               respond,
            })
            .await
            .is_err()
         {
            not_created.insert(
               client_id,
               serde_json::json!({
                   "type": "serverFail",
                   "description": "account task channel closed",
               }),
            );
            continue;
         }
         let res = match time::timeout(Duration::from_secs(30), rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => {
               not_created.insert(
                  client_id,
                  serde_json::json!({
                      "type": "serverFail",
                      "description": "account task dropped import",
                  }),
               );
               continue;
            },
            Err(_) => {
               not_created.insert(
                  client_id,
                  serde_json::json!({
                      "type": "serverFail",
                      "description": "import timed out",
                  }),
               );
               continue;
            },
         };
         match res {
            Ok(outcome) => {
               created.insert(
                  client_id,
                  serde_json::json!({
                      "id": outcome.msgid,
                      "blobId": format!("blob-{}", outcome.msgid),
                      "threadId": outcome.thread_id,
                      // `size` is not known without re-reading the
                      // stored row. Clients that care can Email/get
                      // the new id immediately.
                  }),
               );
            },
            Err(err) => {
               not_created.insert(
                  client_id,
                  serde_json::json!({
                      "type": "serverFail",
                      "description": err.to_string(),
                  }),
               );
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
       "notCreated": object_or_null(not_created),
   }))
}

async fn uploaded_blob_exists(
   state: &AppState,
   account_id: &str,
   blob_id: &str,
) -> Result<bool, MethodError> {
   state
      .pool()
      .get()
      .await
      .map_err(|err| server_fail(format!("db pool: {err}")))?
      .query_opt(
         "SELECT 1 FROM uploaded_blobs WHERE account_id = $1 AND blob_id = $2 AND expires_at > \
          EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT",
         &[&account_id, &blob_id],
      )
      .await
      .map(|row| row.is_some())
      .map_err(|err| server_fail(format!("validate upload blob: {err}")))
}
