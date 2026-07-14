use jmap_protocol::{
   error::MethodError,
   ids::{
      AccountId,
      Id,
      State,
   },
   methods::{
      ChangesRequest,
      ChangesResponse,
      GetRequest,
      GetResponse,
      SetRequest,
   },
};
use serde::Serialize;

use super::{
   MethodResult,
   bad_args,
   enforce_get_limit,
   enforce_set_limit,
   object_or_null,
   server_fail,
   validate_static_since_state,
};

pub fn state<'a>(namespace: &str, values: impl IntoIterator<Item = &'a str>) -> State {
   use sha1::{
      Digest as _,
      Sha1,
   };

   let mut hash = Sha1::new();
   hash.update(namespace.as_bytes());
   for value in values {
      hash.update((value.len() as u64).to_be_bytes());
      hash.update(value.as_bytes());
   }
   State(hex::encode(&hash.finalize()[..6]))
}

pub fn singleton_get<T>(
   args: serde_json::Value,
   object_name: &str,
   current: State,
   id: Id,
   object: T,
   authorize: impl FnOnce(&str) -> Result<(), MethodError>,
) -> MethodResult
where
   T: Serialize,
{
   let object_id = id.0;
   let req = serde_json::from_value::<GetRequest>(args)
      .map_err(|error| bad_args(format!("invalid {object_name}/get args: {error}")))?;
   let account_id = req.account_id.as_ref();
   authorize(account_id)?;
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   }
   let list = if req
      .ids
      .as_ref()
      .is_none_or(|ids| ids.iter().any(|id| id.as_ref() == object_id))
   {
      vec![object]
   } else {
      vec![]
   };
   let not_found = req
      .ids
      .as_deref()
      .unwrap_or_default()
      .iter()
      .filter(|requested| requested.as_ref() != object_id)
      .cloned()
      .collect::<Vec<Id>>();
   let mut response = serde_json::to_value(GetResponse {
      account_id: AccountId(account_id.to_owned()),
      state: current,
      list,
      not_found,
   })
   .map_err(|error| server_fail(error.to_string()))?;
   super::dav_common::project_properties(&mut response, req.properties.as_deref());
   Ok(response)
}

pub fn empty_get(
   args: serde_json::Value,
   object_name: &str,
   current: State,
   authorize: impl FnOnce(&str) -> Result<(), MethodError>,
) -> MethodResult {
   let req = serde_json::from_value::<GetRequest>(args)
      .map_err(|error| bad_args(format!("invalid {object_name}/get args: {error}")))?;
   let account_id = req.account_id.as_ref();
   authorize(account_id)?;
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   }
   serde_json::to_value(GetResponse::<serde_json::Value> {
      account_id: AccountId(account_id.to_owned()),
      state:      current,
      list:       vec![],
      not_found:  req.ids.unwrap_or_default(),
   })
   .map_err(|error| server_fail(error.to_string()))
}

pub fn empty_changes(
   args: serde_json::Value,
   object_name: &str,
   current: State,
   authorize: impl FnOnce(&str) -> Result<(), MethodError>,
) -> MethodResult {
   let req = serde_json::from_value::<ChangesRequest>(args)
      .map_err(|error| bad_args(format!("invalid {object_name}/changes args: {error}")))?;
   let account_id = req.account_id.as_ref();
   authorize(account_id)?;
   validate_static_since_state(&req.since_state, &current)?;
   serde_json::to_value(ChangesResponse {
      account_id:       AccountId(account_id.to_owned()),
      old_state:        req.since_state,
      new_state:        current,
      has_more_changes: false,
      created:          vec![],
      updated:          vec![],
      destroyed:        vec![],
   })
   .map_err(|error| server_fail(error.to_string()))
}

pub fn rejected_set(
   args: serde_json::Value,
   object_name: &str,
   current: State,
   errors: [serde_json::Value; 3],
   authorize: impl FnOnce(&str) -> Result<(), MethodError>,
) -> MethodResult {
   let State(current) = current;
   let [create_error, update_error, destroy_error] = errors;
   let req = serde_json::from_value::<SetRequest<serde_json::Value>>(args)
      .map_err(|error| bad_args(format!("invalid {object_name}/set args: {error}")))?;
   let account_id = req.account_id.as_ref();
   authorize(account_id)?;
   let create = req.create.unwrap_or_default();
   let update = req.update.unwrap_or_default();
   let destroy = req.destroy.unwrap_or_default();
   enforce_set_limit(create.len(), update.len(), destroy.len())?;
   if req
      .if_in_state
      .as_ref()
      .is_some_and(|state| state.as_ref() != current)
   {
      return Err(MethodError::StateMismatch);
   }
   let not_created = create
      .into_keys()
      .map(|id| (id, create_error.clone()))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   let not_updated = update
      .into_keys()
      .map(|id| (id.0, update_error.clone()))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   let not_destroyed = destroy
      .into_iter()
      .map(|id| (id.0, destroy_error.clone()))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   Ok(serde_json::json!({
       "accountId": account_id,
       "oldState": current,
       "newState": current,
       "created": null,
       "notCreated": object_or_null(not_created),
       "updated": null,
       "notUpdated": object_or_null(not_updated),
       "destroyed": null,
       "notDestroyed": object_or_null(not_destroyed),
   }))
}
