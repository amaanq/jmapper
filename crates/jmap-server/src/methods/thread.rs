//! Thread/* methods backed by the threading cache.

use std::collections::{
   HashMap,
   HashSet,
};

use imap_sync::db::StateKind as DbStateKind;
use jmap_protocol::{
   error::MethodError,
   ids::{
      AccountId,
      Id,
   },
   methods::{
      ChangesRequest,
      ChangesResponse,
      GetRequest,
      GetResponse,
   },
   session::MAX_OBJECTS_IN_GET,
   thread::Thread,
};

use super::{
   MethodResult,
   bad_args,
   cached_state,
   enforce_get_limit,
   pg,
   require_auth_match,
   server_fail,
};
use crate::state::{
   AccountInfo,
   AppState,
};

const THREAD_GET_MAX_IDS: usize = MAX_OBJECTS_IN_GET;

/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, the bearer
/// account does not match the requested `accountId`, the requested id set
/// exceeds the per-`get` object limit, a backing `messages` query fails, an
/// unbounded scan finds more than [`THREAD_GET_MAX_IDS`] threads
/// ([`MethodError::RequestTooLarge`]), or the response cannot be serialized.
pub async fn get(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let req = serde_json::from_value::<GetRequest>(args)
      .map_err(|err| bad_args(format!("invalid Thread/get args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   let client = pg(state).await?;

   let ids = if let Some(ids) = req.ids {
      enforce_get_limit(ids.len())?;
      ids
   } else {
      let probe = client
         .query(
            "SELECT DISTINCT thrid FROM messages WHERE account_id = $1 ORDER BY thrid LIMIT $2",
            &[&account_id, &(THREAD_GET_MAX_IDS as i64 + 1)],
         )
         .await
         .map_err(|err| server_fail(format!("Thread/get (ids=null) scan: {err}")))?
         .into_iter()
         .map(|row| row.get::<_, String>(0))
         .collect::<Vec<String>>();
      if probe.len() > THREAD_GET_MAX_IDS {
         return Err(MethodError::RequestTooLarge);
      }
      probe.into_iter().map(Id).collect::<Vec<Id>>()
   };

   let requested = ids.iter().map(|id| id.0.clone()).collect::<Vec<String>>();
   let rows = client
      .query(
         "SELECT thrid, msgid FROM messages WHERE account_id = $1 AND thrid = ANY($2) ORDER BY \
          thrid, received_at",
         &[&account_id, &requested],
      )
      .await
      .map_err(|err| server_fail(format!("Thread/get: {err}")))?;
   let mut by_thread = HashMap::<String, Vec<Id>>::new();
   for row in rows {
      by_thread
         .entry(row.get(0))
         .or_default()
         .push(Id(row.get(1)));
   }
   let mut list = Vec::with_capacity(ids.len());
   let mut not_found = Vec::new();
   for id in ids {
      match by_thread.remove(id.as_ref()) {
         Some(email_ids) => list.push(Thread { id, email_ids }),
         None => not_found.push(id),
      }
   }

   let state_val = cached_state(state, account_id, DbStateKind::Email).await?;
   let mut resp_value = serde_json::to_value(GetResponse::<Thread> {
      account_id: AccountId(account_id.to_owned()),
      state: state_val,
      list,
      not_found,
   })
   .map_err(|err| server_fail(err.to_string()))?;

   if let Some(props) = req.properties.as_ref() {
      let mut allowed = props.iter().map(String::as_str).collect::<HashSet<&str>>();
      allowed.insert("id");
      if let Some(list) = resp_value
         .get_mut("list")
         .and_then(|value| value.as_array_mut())
      {
         for entry in list.iter_mut() {
            if let Some(map) = entry.as_object_mut() {
               map.retain(|key, _| allowed.contains(key.as_str()));
            }
         }
      }
   }
   Ok(resp_value)
}

/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, the bearer
/// account does not match the requested `accountId`, `sinceState` differs from
/// the current cached state ([`MethodError::CannotCalculateChanges`]), or the
/// response cannot be serialized.
pub async fn changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let req = serde_json::from_value::<ChangesRequest>(args)
      .map_err(|err| bad_args(format!("invalid Thread/changes args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   let current = cached_state(state, account_id, DbStateKind::Email).await?;
   if req.since_state != current {
      return Err(MethodError::CannotCalculateChanges);
   }

   serde_json::to_value(ChangesResponse {
      account_id:       req.account_id,
      old_state:        req.since_state,
      new_state:        current,
      has_more_changes: false,
      created:          vec![],
      updated:          vec![],
      destroyed:        vec![],
   })
   .map_err(|err| server_fail(err.to_string()))
}
