//! Mailbox/set methods (RFC 8621 §2.5).

use std::{
   collections::HashMap,
   slice,
   time::Duration,
};

use imap_sync::{
   account::AccountRequest,
   db::{
      MailboxRow,
      StateKind as DbStateKind,
      get_mailboxes_by_ids,
   },
   error::{
      self,
      SyncError,
   },
   sync::is_valid_folder_name,
};
use jmap_protocol::{
   error::MethodError,
   ids::AccountId,
   mailbox::MailboxRights,
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
use crate::state::{
   AccountInfo,
   AppState,
};

/// # Errors
///
/// Returns [`MethodError`] when the arguments fail to deserialize, the
/// authenticated account does not match `accountId`, the create/update/destroy
/// batch exceeds the server limit, or `ifInState` does not match the cached
/// Mailbox state. Per-object create/update/destroy failures are reported inline
/// in the response rather than as method errors.
pub async fn set(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id:               AccountId,
      #[serde(default, rename = "ifInState")]
      if_in_state:              Option<String>,
      #[serde(default)]
      create:                   Option<HashMap<String, serde_json::Value>>,
      #[serde(default)]
      update:                   Option<HashMap<String, serde_json::Value>>,
      #[serde(default)]
      destroy:                  Option<Vec<String>>,
      #[serde(default, rename = "onDestroyRemoveEmails")]
      on_destroy_remove_emails: bool,
   }
   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid Mailbox/set args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   enforce_set_limit(
      req.create.as_ref().map_or(0, HashMap::len),
      req.update.as_ref().map_or(0, HashMap::len),
      req.destroy.as_ref().map_or(0, Vec::len),
   )?;

   let before = cached_state_row(state, account_id).await?;
   let old_state = state_value(&before, DbStateKind::Mailbox);
   if let Some(expected) = req.if_in_state.as_deref()
      && expected != old_state.as_ref()
   {
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

   let mut updated = serde_json::Map::new();
   let mut not_updated = serde_json::Map::new();
   if let Some(update) = req.update {
      for (mailbox_id, patch) in update {
         match apply_update(state, account_id, &mailbox_id, &patch).await {
            Ok(()) => {
               updated.insert(mailbox_id, serde_json::Value::Null);
            },
            Err(set_error) => {
               not_updated.insert(mailbox_id, set_error);
            },
         }
      }
   }

   let mut destroyed_ids = Vec::<String>::new();
   let mut not_destroyed = serde_json::Map::new();
   if let Some(destroy) = req.destroy {
      for id in destroy {
         match apply_destroy(state, account_id, &id, req.on_destroy_remove_emails).await {
            Ok(()) => destroyed_ids.push(id),
            Err(set_error) => {
               not_destroyed.insert(id, set_error);
            },
         }
      }
   }

   let after = cached_state_row(state, account_id).await?;
   let new_state = state_value(&after, DbStateKind::Mailbox);
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

/// Parse + dispatch one create entry. On success returns the server-set
/// properties object (RFC 8620 §5.3: everything the client didn't supply).
async fn apply_create(
   state: &AppState,
   account_id: &str,
   payload: &serde_json::Value,
) -> Result<serde_json::Value, serde_json::Value> {
   let obj = payload.as_object().ok_or_else(|| {
        serde_json::json!({"type": "invalidProperties", "description": "create entry must be an object"})
    })?;

   let mut name = None::<String>;
   let mut parent_id = None::<String>;
   for (key, val) in obj {
      match key.as_str() {
         "name" => {
            name = Some(
               val.as_str()
                  .ok_or_else(|| invalid_prop("name", "name must be a string"))?
                  .to_owned(),
            );
         },
         "parentId" => {
            parent_id = match val {
               serde_json::Value::Null => None,
               serde_json::Value::String(id) => Some(id.clone()),
               _ => return Err(invalid_prop("parentId", "parentId must be an id or null")),
            };
         },
         // Server-managed or cosmetic; accept without persisting so strict
         // clients that echo defaults don't get rejected.
         "isSubscribed" | "sortOrder" => {},
         "role" => {
            if !val.is_null() {
               return Err(invalid_prop(
                  "role",
                  "cannot assign a role; roles come from IMAP special-use",
               ));
            }
         },
         other => {
            return Err(invalid_prop(
               other,
               format!("Mailbox/set create cannot set {other}"),
            ));
         },
      }
   }
   let name = name.ok_or_else(|| invalid_prop("name", "name is required"))?;
   if !is_valid_folder_name(&name) {
      return Err(invalid_prop(
         "name",
         format!("invalid mailbox name {name:?}"),
      ));
   }

   let tx = state
      .account_sender(account_id)
      .ok_or_else(task_unavailable)?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::CreateFolder {
      name,
      parent_mailbox_id: parent_id,
      respond,
   })
   .await
   .map_err(|_| channel_closed())?;
   let mailbox_id = wait_outcome(rx, 30).await??;

   Ok(serde_json::json!({
       "id": mailbox_id,
       "role": serde_json::Value::Null,
       "sortOrder": 10,
       "totalEmails": 0,
       "unreadEmails": 0,
       "totalThreads": 0,
       "unreadThreads": 0,
       "myRights": MailboxRights::writable(false),
       "isSubscribed": true,
   }))
}

async fn apply_update(
   state: &AppState,
   account_id: &str,
   mailbox_id: &str,
   patch: &serde_json::Value,
) -> Result<(), serde_json::Value> {
   let obj = patch.as_object().ok_or_else(
      || serde_json::json!({"type": "invalidPatch", "description": "patch must be an object"}),
   )?;

   let row = load_mailbox(state, account_id, mailbox_id).await?;
   let Some(row) = row else {
      return Err(serde_json::json!({"type": "notFound"}));
   };
   if row.role.is_some() {
      return Err(serde_json::json!({
          "type": "forbidden",
          "description": "role mailboxes (inbox, sent, …) cannot be modified",
      }));
   }

   let mut new_name = None::<String>;
   for (key, val) in obj {
      match key.as_str() {
         "name" => {
            new_name = Some(
               val.as_str()
                  .ok_or_else(|| invalid_prop("name", "name must be a string"))?
                  .to_owned(),
            );
         },
         "isSubscribed" | "sortOrder" => {},
         other => {
            return Err(invalid_prop(
               other,
               format!("Mailbox/set update cannot change {other}"),
            ));
         },
      }
   }
   let Some(new_name) = new_name else {
      return Ok(());
   };
   if !is_valid_folder_name(&new_name) {
      return Err(invalid_prop(
         "name",
         format!("invalid mailbox name {new_name:?}"),
      ));
   }
   if new_name == row.name {
      return Ok(());
   }

   let tx = state
      .account_sender(account_id)
      .ok_or_else(task_unavailable)?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::RenameFolder {
      mailbox_id: mailbox_id.to_owned(),
      new_name,
      respond,
   })
   .await
   .map_err(|_| channel_closed())?;
   wait_outcome(rx, 30).await?
}

async fn apply_destroy(
   state: &AppState,
   account_id: &str,
   mailbox_id: &str,
   on_destroy_remove_emails: bool,
) -> Result<(), serde_json::Value> {
   let row = load_mailbox(state, account_id, mailbox_id).await?;
   let Some(row) = row else {
      return Err(serde_json::json!({"type": "notFound"}));
   };
   if row.role.is_some() {
      return Err(serde_json::json!({
          "type": "forbidden",
          "description": "role mailboxes (inbox, sent, …) cannot be destroyed",
      }));
   }
   if row.total_emails > 0 && !on_destroy_remove_emails {
      return Err(serde_json::json!({
          "type": "mailboxHasEmail",
          "description": format!(
              "mailbox contains {} emails; pass onDestroyRemoveEmails: true",
              row.total_emails
          ),
      }));
   }

   let tx = state
      .account_sender(account_id)
      .ok_or_else(task_unavailable)?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::DeleteFolder {
      mailbox_id: mailbox_id.to_owned(),
      respond,
   })
   .await
   .map_err(|_| channel_closed())?;
   wait_outcome(rx, 30).await?
}

async fn load_mailbox(
   state: &AppState,
   account_id: &str,
   mailbox_id: &str,
) -> Result<Option<MailboxRow>, serde_json::Value> {
   let rows = get_mailboxes_by_ids(
      state.pool(),
      account_id,
      slice::from_ref(&mailbox_id.to_owned()),
   )
   .await
   .map_err(|err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}))?;
   Ok(rows.into_iter().next())
}

/// Await an account-task oneshot with a timeout, translating channel and
/// sync-layer failures into `SetError` objects.
async fn wait_outcome<T>(
   rx: oneshot::Receiver<error::Result<T>>,
   secs: u64,
) -> Result<Result<T, serde_json::Value>, serde_json::Value> {
   let outcome = time::timeout(Duration::from_secs(secs), rx)
        .await
        .map_err(|_| {
            serde_json::json!({"type": "serverFail", "description": "mailbox operation timed out"})
        })?
        .map_err(|_| {
            serde_json::json!({"type": "serverFail", "description": "account task dropped request"})
        })?;
   Ok(outcome.map_err(|err| {
      match err {
         SyncError::MailboxHasChild(_) => {
            serde_json::json!({
                "type": "mailboxHasChild",
                "description": err.to_string(),
            })
         },
         other => serde_json::json!({"type": "serverFail", "description": other.to_string()}),
      }
   }))
}

fn invalid_prop(prop: &str, description: impl Into<String>) -> serde_json::Value {
   serde_json::json!({
       "type": "invalidProperties",
       "properties": [prop],
       "description": description.into(),
   })
}

fn task_unavailable() -> serde_json::Value {
   serde_json::json!({
       "type": "serverUnavailable",
       "description": "sync task is not running for this account",
   })
}

fn channel_closed() -> serde_json::Value {
   serde_json::json!({"type": "serverFail", "description": "account task channel closed"})
}
