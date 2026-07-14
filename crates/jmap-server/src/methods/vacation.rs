//! Disabled `VacationResponse` methods (RFC 8621 §8).

use jmap_protocol::ids::Id;
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
struct VacationResponse {
   id:         Id,
   #[serde(rename = "isEnabled")]
   is_enabled: bool,
   #[serde(rename = "fromDate")]
   from_date:  Option<chrono::DateTime<chrono::Utc>>,
   #[serde(rename = "toDate")]
   to_date:    Option<chrono::DateTime<chrono::Utc>>,
   subject:    Option<String>,
   #[serde(rename = "textBody")]
   text_body:  Option<String>,
   #[serde(rename = "htmlBody")]
   html_body:  Option<String>,
}

/// # Errors
///
/// Returns a `MethodError` if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account.
pub fn get(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let response = VacationResponse {
      id:         Id("singleton".into()),
      is_enabled: false,
      from_date:  None,
      to_date:    None,
      subject:    None,
      text_body:  None,
      html_body:  None,
   };
   static_object::singleton_get(
      args,
      "VacationResponse",
      static_object::state("vacation", [auth.id.as_str()]),
      response.id.clone(),
      response,
      |account_id| require_auth_match(auth, account_id),
   )
}

/// # Errors
///
/// Returns a `MethodError` if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account.
pub fn changes(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   static_object::empty_changes(
      args,
      "VacationResponse",
      static_object::state("vacation", [auth.id.as_str()]),
      |account_id| require_auth_match(auth, account_id),
   )
}

/// # Errors
///
/// Returns a `MethodError` if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account. Vacation
/// responses are not configurable, so create, update, and destroy attempts are
/// rejected per-object in the response rather than as an error.
pub fn set(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let error = serde_json::json!({
       "type": "forbidden",
       "description": "vacation responses are not configurable"
   });
   static_object::rejected_set(
      args,
      "VacationResponse",
      static_object::state("vacation", [auth.id.as_str()]),
      [error.clone(), error.clone(), error],
      |account_id| require_auth_match(auth, account_id),
   )
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn disabled_vacation_response_keeps_nullable_properties() {
      let value = serde_json::to_value(VacationResponse {
         id:         Id("singleton".into()),
         is_enabled: false,
         from_date:  None,
         to_date:    None,
         subject:    None,
         text_body:  None,
         html_body:  None,
      })
      .unwrap();

      for property in ["fromDate", "toDate", "subject", "textBody", "htmlBody"] {
         assert!(
            value.get(property).is_some_and(serde_json::Value::is_null),
            "missing nullable property {property}: {value}"
         );
      }
   }
}
