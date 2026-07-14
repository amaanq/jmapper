//! Disabled webhook `PushSubscription` methods (RFC 8620 §7.2).

use std::collections::BTreeMap;

use jmap_protocol::{
   error::MethodError,
   ids::Id,
};
use serde::Deserialize;

use super::{
   MethodResult,
   bad_args,
   enforce_get_limit,
   enforce_set_limit,
   object_or_null,
};

#[derive(Deserialize)]
struct GetArgs {
   #[serde(default)]
   ids:        Option<Vec<Id>>,
   #[serde(default)]
   properties: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct SetArgs {
   #[serde(default)]
   create:  BTreeMap<String, serde_json::Value>,
   #[serde(default)]
   update:  BTreeMap<String, serde_json::Value>,
   #[serde(default)]
   destroy: Vec<String>,
}

/// # Errors
///
/// Returns a [`MethodError`] if the arguments are malformed, more ids are
/// requested than the server limit allows, or the private `url`/`keys`
/// properties are requested (which is [`MethodError::Forbidden`]).
pub fn get(args: serde_json::Value) -> MethodResult {
   let req = serde_json::from_value::<GetArgs>(args)
      .map_err(|error| bad_args(format!("invalid PushSubscription/get args: {error}")))?;
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   }
   if req.properties.as_ref().is_some_and(|properties| {
      properties
         .iter()
         .any(|property| matches!(property.as_str(), "url" | "keys"))
   }) {
      return Err(MethodError::Forbidden);
   }
   Ok(serde_json::json!({
       "list": [],
       "notFound": req.ids.unwrap_or_default(),
   }))
}

/// # Errors
///
/// Returns a [`MethodError`] if the arguments are malformed or the combined
/// create/update/destroy count exceeds the server set limit. Webhook push
/// subscriptions are disabled, so individual changes are rejected per-object
/// in the response rather than as an error.
pub fn set(args: serde_json::Value) -> MethodResult {
   let req = serde_json::from_value::<SetArgs>(args)
      .map_err(|error| bad_args(format!("invalid PushSubscription/set args: {error}")))?;
   enforce_set_limit(req.create.len(), req.update.len(), req.destroy.len())?;
   let not_created = req
      .create
      .into_keys()
      .map(|id| {
         (
            id,
            serde_json::json!({
                "type": "forbidden",
                "description": "webhook push subscriptions are disabled"
            }),
         )
      })
      .collect::<serde_json::Map<String, serde_json::Value>>();
   let not_updated = req
      .update
      .into_keys()
      .map(|id| (id, serde_json::json!({"type": "notFound"})))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   let not_destroyed = req
      .destroy
      .into_iter()
      .map(|id| (id, serde_json::json!({"type": "notFound"})))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   Ok(serde_json::json!({
       "created": null,
       "notCreated": object_or_null(not_created),
       "updated": null,
       "notUpdated": object_or_null(not_updated),
       "destroyed": null,
       "notDestroyed": object_or_null(not_destroyed),
   }))
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn private_webhook_properties_are_forbidden() {
      assert_eq!(
         get(serde_json::json!({"properties": ["url"]})),
         Err(MethodError::Forbidden)
      );
   }

   #[test]
   fn disabled_webhooks_return_per_object_errors() {
      let value = set(serde_json::json!({
          "create": {"new": {"url": "https://example.test/push"}},
          "update": {"missing": {"expires": "2030-01-01T00:00:00Z"}},
          "destroy": ["gone"]
      }))
      .unwrap();
      assert_eq!(value["notCreated"]["new"]["type"], "forbidden");
      assert_eq!(value["notUpdated"]["missing"]["type"], "notFound");
      assert_eq!(value["notDestroyed"]["gone"]["type"], "notFound");
   }
}
