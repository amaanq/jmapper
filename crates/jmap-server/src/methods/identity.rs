//! Identity/get, /changes, and read-only /set (RFC 8621 §6).

use jmap_protocol::{
   email::EmailAddress,
   ids::{
      Id,
      State,
   },
};
use serde::{
   Deserialize,
   Serialize,
};

use super::{
   MethodResult,
   require_auth_match,
   static_object,
};
use crate::state::{
   AccountInfo,
   AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Identity {
   id:             Id,
   name:           String,
   email:          String,
   #[serde(rename = "replyTo")]
   reply_to:       Option<Vec<EmailAddress>>,
   bcc:            Option<Vec<EmailAddress>>,
   #[serde(rename = "textSignature")]
   text_signature: String,
   #[serde(rename = "htmlSignature")]
   html_signature: String,
   #[serde(rename = "mayDelete")]
   may_delete:     bool,
}

/// # Errors
///
/// Returns a `MethodError` if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account.
pub fn get(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let identity = Identity {
      id:             Id(format!("ident-{}", auth.id)),
      name:           auth.display_name.clone(),
      email:          auth.email.clone(),
      reply_to:       None,
      bcc:            None,
      text_signature: String::new(),
      html_signature: String::new(),
      may_delete:     false,
   };
   static_object::singleton_get(
      args,
      "Identity",
      identity_state(auth),
      identity.id.clone(),
      identity,
      |account_id| require_auth_match(auth, account_id),
   )
}

/// # Errors
///
/// Returns a `MethodError` if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account.
pub fn changes(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   static_object::empty_changes(args, "Identity", identity_state(auth), |account_id| {
      require_auth_match(auth, account_id)
   })
}

/// # Errors
///
/// Returns a `MethodError` if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account. Identities
/// are read-only, so attempts to create, update, or destroy them are rejected
/// per-object in the response rather than as an error.
pub fn set(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let error = serde_json::json!({
       "type": "forbidden",
       "description": "identities are derived from account configuration"
   });
   static_object::rejected_set(
      args,
      "Identity",
      identity_state(auth),
      [error.clone(), error.clone(), error],
      |account_id| require_auth_match(auth, account_id),
   )
}

fn identity_state(auth: &AccountInfo) -> State {
   static_object::state("identity", [
      auth.id.as_str(),
      auth.email.as_str(),
      auth.display_name.as_str(),
   ])
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn nullable_identity_properties_are_not_omitted() {
      let value = serde_json::to_value(Identity {
         id:             Id("id".into()),
         name:           "Example".into(),
         email:          "example@example.test".into(),
         reply_to:       None,
         bcc:            None,
         text_signature: String::new(),
         html_signature: String::new(),
         may_delete:     false,
      })
      .unwrap();

      assert!(value.get("replyTo").is_some_and(serde_json::Value::is_null));
      assert!(value.get("bcc").is_some_and(serde_json::Value::is_null));
   }

   #[test]
   fn identity_state_tracks_config_backed_properties() {
      let before =
         AccountInfo::from_bearer_token("account", "before@example.test", "Example", "token");
      let after =
         AccountInfo::from_bearer_token("account", "after@example.test", "Example", "token");
      assert_ne!(identity_state(&before), identity_state(&after));
   }
}
