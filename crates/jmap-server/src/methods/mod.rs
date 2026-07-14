//! JMAP method implementations.

pub mod calendar;
pub mod calendar_aux;
pub mod contacts;
pub mod copy;
pub(crate) mod dav_common;
pub mod email;
pub mod email_import;
pub mod email_parse;
pub mod email_props;
pub mod email_set;
pub mod email_submission;
pub mod identity;
pub mod mailbox;
pub mod mailbox_set;
pub mod push_subscription;
pub mod quota;
pub mod search_snippet;
mod static_object;
pub mod thread;
pub mod vacation;

use imap_sync::db;
use jmap_protocol::{
   error::MethodError,
   ids::State,
   session::{
      MAX_OBJECTS_IN_GET,
      MAX_OBJECTS_IN_SET,
   },
};
use tokio_postgres::types::ToSql;

use crate::state::{
   AccountInfo,
   AppState,
   StateChange,
   StateKind,
};

pub type MethodResult = Result<serde_json::Value, MethodError>;

pub(crate) async fn pg(state: &AppState) -> Result<deadpool_postgres::Object, MethodError> {
   state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("db pool: {error}")))
}

pub(crate) async fn cached_state(
   state: &AppState,
   account_id: &str,
   kind: db::StateKind,
) -> Result<State, MethodError> {
   cached_state_row(state, account_id)
      .await
      .map(|state| state_value(&state, kind))
}

pub(crate) async fn cached_state_row(
   state: &AppState,
   account_id: &str,
) -> Result<db::StateRow, MethodError> {
   db::get_state(state.pool(), account_id)
      .await
      .map_err(|error| server_fail(format!("state: {error}")))
}

pub(crate) fn state_value(state: &db::StateRow, kind: db::StateKind) -> State {
   State(state.modseq(kind).to_string())
}

pub(crate) fn publish_imap_state_changes(
   state: &AppState,
   account_id: &str,
   before: &db::StateRow,
   after: &db::StateRow,
) {
   for (kind, old, new) in [
      (StateKind::Email, before.email_modseq, after.email_modseq),
      (
         StateKind::Mailbox,
         before.mailbox_modseq,
         after.mailbox_modseq,
      ),
      (
         StateKind::EmailSubmission,
         before.submission_modseq,
         after.submission_modseq,
      ),
   ] {
      if old != new {
         state.publish_state_change(StateChange {
            account_id: account_id.to_owned(),
            kind,
            new_state: new.to_string(),
         });
      }
   }
}

pub(crate) fn escape_like(value: &str) -> String {
   value
      .replace('\\', r"\\")
      .replace('%', r"\%")
      .replace('_', r"\_")
}

/// Renumber `?` placeholders into PostgreSQL's `$1..$N`. The dynamic query
/// builders (filter compiler, sort variants) emit `?` for readability; no
/// literal question mark ever appears inside the SQL text itself (user
/// input only reaches queries through bind parameters).
pub(crate) fn pg_numbered(sql: &str) -> String {
   let mut out = String::with_capacity(sql.len() + 8);
   let mut n = 0;
   for ch in sql.chars() {
      if ch == '?' {
         n += 1;
         out.push('$');
         out.push_str(&n.to_string());
      } else {
         out.push(ch);
      }
   }
   out
}

/// Owned bind value for dynamically-built SQL.
#[derive(Debug, Clone)]
pub(crate) enum SqlParam {
   Str(String),
   Int(i64),
}

impl SqlParam {
   pub(crate) fn as_dyn(&self) -> &(dyn ToSql + Sync) {
      match self {
         Self::Str(text) => text,
         Self::Int(i) => i,
      }
   }
}

/// Helpers shared across method handlers.
pub(crate) fn bad_args(msg: impl Into<String>) -> MethodError {
   MethodError::InvalidArguments {
      description: Some(msg.into()),
   }
}

pub(crate) fn server_fail(msg: impl Into<String>) -> MethodError {
   MethodError::ServerFail {
      description: Some(msg.into()),
   }
}

pub(crate) const fn enforce_get_limit(count: usize) -> Result<(), MethodError> {
   if count > MAX_OBJECTS_IN_GET {
      Err(MethodError::RequestTooLarge)
   } else {
      Ok(())
   }
}

pub(crate) const fn enforce_set_limit(
   create: usize,
   update: usize,
   destroy: usize,
) -> Result<(), MethodError> {
   if create.saturating_add(update).saturating_add(destroy) > MAX_OBJECTS_IN_SET {
      Err(MethodError::RequestTooLarge)
   } else {
      Ok(())
   }
}

/// Resolve RFC 8620's signed query position against the total result count.
/// Negative positions count back from the end and clamp to zero.
pub(crate) fn query_position(position: Option<i64>, total: usize) -> usize {
   match position.unwrap_or(0) {
      position if position < 0 => {
         total.saturating_sub(usize::try_from(position.unsigned_abs()).unwrap_or(usize::MAX))
      },
      position => usize::try_from(position).unwrap_or(usize::MAX),
   }
}

/// Apply an anchor offset without signed overflow or lossy casts.
pub(crate) fn query_anchor_position(index: usize, offset: Option<i64>) -> usize {
   match offset.unwrap_or(0) {
      offset if offset < 0 => {
         index.saturating_sub(usize::try_from(offset.unsigned_abs()).unwrap_or(usize::MAX))
      },
      offset => index.saturating_add(usize::try_from(offset).unwrap_or(usize::MAX)),
   }
}

/// Return the effective query limit and the optional response field. JMAP
/// only echoes `limit` when the server supplied or reduced it.
pub(crate) const fn query_limit(requested: Option<u32>, maximum: u32) -> (usize, Option<u32>) {
   match requested {
      Some(requested) if requested <= maximum => (requested as usize, None),
      _ => (maximum as usize, Some(maximum)),
   }
}

pub(crate) fn validate_static_since_state(
   since_state: &State,
   current_state: &State,
) -> Result<(), MethodError> {
   if since_state == current_state {
      Ok(())
   } else {
      Err(MethodError::CannotCalculateChanges)
   }
}

pub(crate) fn object_or_null(map: serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
   if map.is_empty() {
      serde_json::Value::Null
   } else {
      serde_json::Value::Object(map)
   }
}

pub(crate) fn ids_or_null(ids: Vec<String>) -> serde_json::Value {
   if ids.is_empty() {
      serde_json::Value::Null
   } else {
      serde_json::Value::Array(ids.into_iter().map(serde_json::Value::String).collect())
   }
}

/// Core authz predicate: the `accountId` the request is asking about must be
/// the one the bearer token authenticates for. Returns `AccountNotFound` on
/// mismatch so we don't leak which accounts exist in the process.
pub(crate) fn require_auth_match(auth: &AccountInfo, requested: &str) -> Result<(), MethodError> {
   if auth.id == requested {
      Ok(())
   } else {
      Err(MethodError::AccountNotFound)
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn empty_set_collections_serialize_as_null() {
      assert_eq!(
         object_or_null(serde_json::Map::new()),
         serde_json::Value::Null
      );
      assert_eq!(ids_or_null(Vec::new()), serde_json::Value::Null);

      let mut map = serde_json::Map::new();
      map.insert("id".into(), serde_json::Value::Null);
      assert!(object_or_null(map).is_object());
      assert!(ids_or_null(vec!["id".into()]).is_array());
   }

   #[test]
   fn query_positions_follow_rfc_8620_signed_offsets() {
      assert_eq!(query_position(None, 10), 0);
      assert_eq!(query_position(Some(3), 10), 3);
      assert_eq!(query_position(Some(-1), 10), 9);
      assert_eq!(query_position(Some(-20), 10), 0);
      assert_eq!(query_anchor_position(2, Some(-10)), 0);
      assert_eq!(query_anchor_position(2, Some(3)), 5);
   }

   #[test]
   fn query_limit_is_only_echoed_when_the_server_clamps_it() {
      assert_eq!(query_limit(Some(10), 500), (10, None));
      assert_eq!(query_limit(Some(900), 500), (500, Some(500)));
      assert_eq!(query_limit(None, 500), (500, Some(500)));
   }

   #[test]
   fn static_changes_only_accept_the_current_state() {
      let current = State("current".into());
      validate_static_since_state(&current, &current).unwrap();
      assert_eq!(
         validate_static_since_state(&State("stale".into()), &current),
         Err(MethodError::CannotCalculateChanges)
      );
   }

   #[test]
   fn advertised_object_limits_use_request_too_large() {
      enforce_get_limit(MAX_OBJECTS_IN_GET).unwrap();
      assert_eq!(
         enforce_get_limit(MAX_OBJECTS_IN_GET + 1),
         Err(MethodError::RequestTooLarge)
      );
      assert_eq!(
         enforce_set_limit(200, 200, 101),
         Err(MethodError::RequestTooLarge)
      );
      assert_eq!(
         serde_json::to_value(MethodError::RequestTooLarge).unwrap(),
         serde_json::json!({"type": "requestTooLarge"})
      );
   }
}
