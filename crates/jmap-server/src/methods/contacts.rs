//! RFC 9610 `AddressBook` and `ContactCard` methods backed by `CardDAV`.

use std::{
   cmp::Ordering,
   collections::BTreeMap,
};

use dav_sync::{
   convert::contact::card_to_vcard,
   service::DavHandle,
   store::{
      self,
      DavKind,
      ResourceRow,
   },
};
use jmap_protocol::{
   error::MethodError,
   filter::{
      Comparator,
      Filter,
      FilterOp,
      has_unsupported_fields,
   },
   ids::{
      AccountId,
      Id,
      State,
   },
   methods::{
      GetRequest,
      GetResponse,
      QueryResponse,
   },
};
use serde::{
   Deserialize,
   Serialize,
};

use super::{
   MethodResult,
   bad_args,
   dav_common::{
      DavObject,
      DavSetArgs,
      MAX_GET_OBJECTS,
      QueryChanges,
      apply_patch_object,
      changes,
      collection_get,
      dav_set_error,
      dav_state,
      ensure_initial_sync,
      parse_since_state,
      project_properties,
      query_changes_response,
      query_hash,
      query_window,
      read_only_collection_set,
      require_dav,
      save_query_snapshot,
      select_objects,
      set_error,
      take_single_membership,
      text_matches_json,
      text_matches_json_values,
      text_matches_str,
      validate_collection,
   },
   enforce_get_limit,
   enforce_set_limit,
   ids_or_null,
   object_or_null,
   query_limit,
   server_fail,
};
use crate::state::{
   AccountInfo,
   AppState,
   StateChange,
   StateKind,
};

const SNAPSHOT_KIND: &str = "ContactCard";

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, the
/// account has no `CardDAV` handle configured, the initial sync fails, or the
/// address books cannot be loaded from the DAV store.
pub async fn addressbook_get(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   collection_get(
      state,
      auth,
      args,
      DavObject::AddressBook,
      addressbook_object,
   )
   .await
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, the
/// account has no `CardDAV` handle configured, the initial sync fails, or the
/// changes cannot be computed against the requested state.
pub async fn addressbook_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   changes(state, auth, args, DavObject::AddressBook).await
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, the
/// account has no `CardDAV` handle configured, or the `ifInState` precondition
/// does not match. Address books are read-only, so per-object rejections are
/// reported in the response rather than as an error.
pub async fn addressbook_set(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   read_only_collection_set(state, auth, args, DavObject::AddressBook).await
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, more
/// objects are requested than the per-call limit allows, the account has no
/// `CardDAV` handle configured, the initial sync fails, the contact cards or
/// DAV state cannot be loaded from the store, or the response cannot be
/// serialized.
pub async fn contact_get(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let req = serde_json::from_value::<GetRequest>(args)
      .map_err(|error| bad_args(format!("invalid ContactCard/get args: {error}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CardDav)?;
   ensure_initial_sync(state, account_id, DavKind::CardDav, &handle).await?;
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   }

   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("DAV db pool: {error}")))?;
   let rows = store::list_resources_by_kind(&client, account_id, DavKind::CardDav)
      .await
      .map_err(|error| server_fail(format!("loading contact cards: {error}")))?;
   if req.ids.is_none() {
      enforce_get_limit(rows.len())?;
   }
   let mut by_id = rows
      .iter()
      .map(|row| Ok((row.id.as_str(), contact_object(row)?)))
      .collect::<Result<BTreeMap<&str, serde_json::Value>, MethodError>>()?;
   let (list, not_found) = select_objects(req.ids.as_deref(), &mut by_id);
   let state_value = store::get_state(&client, account_id)
      .await
      .map_err(|error| server_fail(format!("loading DAV state: {error}")))?
      .contact_card_modseq;
   let mut response = serde_json::to_value(GetResponse::<serde_json::Value> {
      account_id: AccountId(account_id.to_owned()),
      state: State(state_value.to_string()),
      list,
      not_found,
   })
   .map_err(|error| server_fail(error.to_string()))?;
   project_properties(&mut response, req.properties.as_deref());
   Ok(response)
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, the
/// account has no `CardDAV` handle configured, the initial sync fails, or the
/// changes cannot be computed against the requested state.
pub async fn contact_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   changes(state, auth, args, DavObject::ContactCard).await
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, the
/// account has no `CardDAV` handle configured, the initial sync fails, the
/// `ifInState` precondition does not match, or more changes are requested than
/// the per-call limit allows. Per-object create/update/destroy failures are
/// reported in the response rather than as an error.
pub async fn contact_set(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let req = serde_json::from_value::<DavSetArgs>(args)
      .map_err(|error| bad_args(format!("invalid ContactCard/set args: {error}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CardDav)?;
   ensure_initial_sync(state, account_id, DavKind::CardDav, &handle).await?;
   let old_state = dav_state(state, account_id).await?.contact_card_modseq;
   super::dav_common::check_if_in_state(req.if_in_state.as_ref(), &old_state.to_string())?;
   enforce_set_limit(req.create.len(), req.update.len(), req.destroy.len())?;

   let mut created = serde_json::Map::new();
   let mut not_created = serde_json::Map::new();
   let mut updated = Vec::<String>::new();
   let mut not_updated = serde_json::Map::new();
   let mut destroyed = Vec::<String>::new();
   let mut not_destroyed = serde_json::Map::new();

   for (creation_id, value) in req.create {
      match create_contact(state, account_id, &handle, value).await {
         Ok(row) => {
            created.insert(creation_id, serde_json::json!({"id": row.id}));
         },
         Err(error) => {
            not_created.insert(creation_id, error);
         },
      }
   }
   for (id, patch) in req.update {
      match update_contact(state, account_id, &handle, &id, &patch).await {
         Ok(()) => updated.push(id),
         Err(error) => {
            not_updated.insert(id, error);
         },
      }
   }
   for id in req.destroy {
      match destroy_contact(state, account_id, &handle, &id).await {
         Ok(()) => destroyed.push(id),
         Err(error) => {
            not_destroyed.insert(id, error);
         },
      }
   }

   let new_state = dav_state(state, account_id).await?.contact_card_modseq;
   if new_state != old_state {
      state.publish_state_change(StateChange {
         account_id: account_id.to_owned(),
         kind:       StateKind::ContactCard,
         new_state:  new_state.to_string(),
      });
   }
   Ok(serde_json::json!({
       "accountId": account_id,
       "oldState": old_state.to_string(),
       "newState": new_state.to_string(),
       "created": object_or_null(created),
       "notCreated": object_or_null(not_created),
       "updated": ids_or_null(updated),
       "notUpdated": object_or_null(not_updated),
       "destroyed": ids_or_null(destroyed),
       "notDestroyed": object_or_null(not_destroyed),
   }))
}

/// # Errors
///
/// Returns a [`MethodError`] if the filter references unsupported fields, the
/// request arguments are malformed, the account has no `CardDAV` handle
/// configured, the initial sync fails, the sort references unsupported
/// properties, the contacts cannot be loaded, or the query window/anchor is
/// invalid.
pub async fn contact_query(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_filter_fields(args.get("filter"))?;
   let req = serde_json::from_value::<QueryArgs>(args)
      .map_err(|error| bad_args(format!("invalid ContactCard/query args: {error}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CardDav)?;
   ensure_initial_sync(state, account_id, DavKind::CardDav, &handle).await?;
   let (rows, current) = load_contacts(state, account_id).await?;
   let ordered = filter_and_sort(rows, req.filter.as_ref(), req.sort.as_deref())?;
   let all_ids = ordered
      .iter()
      .map(|row| row.id.clone())
      .collect::<Vec<String>>();
   let hash = query_hash(&(req.filter.as_ref(), req.sort.as_deref()), SNAPSHOT_KIND)?;
   save_query_snapshot(state, account_id, SNAPSHOT_KIND, &hash, current, &all_ids).await?;
   let (limit, response_limit) = query_limit(req.limit, MAX_GET_OBJECTS as u32);
   let (position, ids) = query_window(
      &all_ids,
      req.position,
      req.anchor.as_ref(),
      req.anchor_offset,
      limit,
   )?;
   serde_json::to_value(QueryResponse {
      account_id: AccountId(account_id.to_owned()),
      query_state: State(current.to_string()),
      can_calculate_changes: true,
      position,
      ids: ids.into_iter().map(Id).collect(),
      total: req.calculate_total.then_some(all_ids.len() as u64),
      limit: response_limit,
   })
   .map_err(|error| server_fail(error.to_string()))
}

/// # Errors
///
/// Returns a [`MethodError`] if the filter references unsupported fields, the
/// request arguments are malformed, the account has no `CardDAV` handle
/// configured, the initial sync fails, `upToId` is set (changes cannot be
/// calculated), the `sinceQueryState` is invalid, or the contacts cannot be
/// loaded.
pub async fn contact_query_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_filter_fields(args.get("filter"))?;
   let req = serde_json::from_value::<QueryChangesArgs>(args)
      .map_err(|error| bad_args(format!("invalid ContactCard/queryChanges args: {error}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CardDav)?;
   ensure_initial_sync(state, account_id, DavKind::CardDav, &handle).await?;
   if req.up_to_id.is_some() {
      return Err(MethodError::CannotCalculateChanges);
   }

   let (rows, current) = load_contacts(state, account_id).await?;
   let since = parse_since_state(req.since_query_state.as_ref(), current)?;
   let hash = query_hash(&(req.filter.as_ref(), req.sort.as_deref()), SNAPSHOT_KIND)?;
   let ordered = filter_and_sort(rows, req.filter.as_ref(), req.sort.as_deref())?;
   let new_ids = ordered
      .iter()
      .map(|row| row.id.clone())
      .collect::<Vec<String>>();
   query_changes_response(state, QueryChanges {
      account_id,
      snapshot_kind: SNAPSHOT_KIND,
      hash: &hash,
      since,
      current,
      new_ids: &new_ids,
      max_changes: req.max_changes,
      calculate_total: req.calculate_total,
   })
   .await
}

async fn create_contact(
   state: &AppState,
   account_id: &str,
   handle: &DavHandle,
   mut value: serde_json::Value,
) -> Result<ResourceRow, serde_json::Value> {
   if value.get("id").is_some_and(|id| !id.is_null()) {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["id"],
          "description": "id is server-set"
      }));
   }
   let collection_id =
      take_single_membership(&mut value, "ContactCard", "addressBookIds", "addressBookId")?;
   validate_collection(
      state,
      account_id,
      &collection_id,
      DavKind::CardDav,
      "addressBookIds",
      "unknown address book",
   )
   .await?;
   let object = value
      .as_object_mut()
      .ok_or_else(|| set_error("invalidProperties", "ContactCard must be an object"))?;
   object.remove("id");
   object
      .entry("uid".to_owned())
      .or_insert_with(|| serde_json::Value::String(random_uid()));
   let raw = card_raw(value)?;
   handle
      .put_resource(DavKind::CardDav, collection_id, None, raw)
      .await
      .map_err(dav_set_error)
}

async fn update_contact(
   state: &AppState,
   account_id: &str,
   handle: &DavHandle,
   id: &str,
   patch: &serde_json::Value,
) -> Result<(), serde_json::Value> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| set_error("serverFail", format!("DAV db pool: {error}")))?;
   let row = store::get_resource(&client, account_id, id)
      .await
      .map_err(|error| set_error("serverFail", error.to_string()))?
      .filter(|row| !row.destroyed && row.kind == DavKind::CardDav.resource_kind())
      .ok_or_else(|| set_error("notFound", "contact card not found"))?;
   drop(client);
   let current =
      contact_object(&row).map_err(|error| set_error("serverFail", format!("{error:?}")))?;
   let mut updated = apply_patch_object(&current, patch, &["id"])?;
   if updated.get("uid").and_then(|uid| uid.as_str()) != Some(row.uid.as_str()) {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["uid"],
          "description": "uid is immutable"
      }));
   }
   let mut current_content = current.clone();
   take_single_membership(
      &mut current_content,
      "ContactCard",
      "addressBookIds",
      "addressBookId",
   )?;
   let collection_id = take_single_membership(
      &mut updated,
      "ContactCard",
      "addressBookIds",
      "addressBookId",
   )?;
   validate_collection(
      state,
      account_id,
      &collection_id,
      DavKind::CardDav,
      "addressBookIds",
      "unknown address book",
   )
   .await?;
   let moving = row.collection_id != collection_id;
   let content_changed = current_content != updated;
   if moving && content_changed {
      return Err(set_error(
         "invalidPatch",
         "moving and editing a DAV contact in one update is not atomic; submit separate updates",
      ));
   }
   if !content_changed {
      if moving {
         handle
            .move_resource(DavKind::CardDav, id.to_owned(), collection_id)
            .await
            .map_err(dav_set_error)?;
      }
      return Ok(());
   }
   let raw = card_raw(updated)?;
   handle
      .put_resource(DavKind::CardDav, collection_id, Some(id.to_owned()), raw)
      .await
      .map_err(dav_set_error)?;
   Ok(())
}

async fn destroy_contact(
   state: &AppState,
   account_id: &str,
   handle: &DavHandle,
   id: &str,
) -> Result<(), serde_json::Value> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| set_error("serverFail", format!("DAV db pool: {error}")))?;
   let found = store::get_resource(&client, account_id, id)
      .await
      .map_err(|error| set_error("serverFail", error.to_string()))?
      .is_some_and(|row| !row.destroyed && row.kind == DavKind::CardDav.resource_kind());
   drop(client);
   if !found {
      return Err(set_error("notFound", "contact card not found"));
   }
   handle
      .delete_resource(DavKind::CardDav, id.to_owned())
      .await
      .map_err(dav_set_error)
}

fn contact_object(row: &ResourceRow) -> Result<serde_json::Value, MethodError> {
   let mut value = serde_json::from_str::<serde_json::Value>(&row.json)
      .map_err(|error| server_fail(format!("stored ContactCard JSON is invalid: {error}")))?;
   let object = value
      .as_object_mut()
      .ok_or_else(|| server_fail("stored ContactCard JSON is not an object"))?;
   object.insert("id".to_owned(), serde_json::Value::String(row.id.clone()));
   object.insert(
      "addressBookIds".to_owned(),
      serde_json::json!({row.collection_id.clone(): true}),
   );
   Ok(value)
}

fn addressbook_object(row: &store::CollectionRow, is_default: bool) -> serde_json::Value {
   serde_json::json!({
       "id": row.id,
       "name": row.name,
       "description": row.description,
       "sortOrder": 0,
       "isDefault": is_default,
       "isSubscribed": true,
       "shareWith": null,
       "myRights": {
           "mayRead": true,
           "mayWrite": true,
           "mayShare": false,
           "mayDelete": false,
       }
   })
}

fn card_raw(mut value: serde_json::Value) -> Result<String, serde_json::Value> {
   let object = value
      .as_object_mut()
      .ok_or_else(|| set_error("invalidProperties", "ContactCard must be an object"))?;
   object.remove("id");
   object.remove("addressBookIds");
   validate_card_metadata(object)?;
   card_to_vcard(&value).map_err(|error| {
      serde_json::json!({
          "type": "invalidProperties",
          "description": format!("cannot convert JSContact Card: {error}")
      })
   })
}

fn validate_card_metadata(
   object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), serde_json::Value> {
   if object.get("@type").and_then(serde_json::Value::as_str) != Some("Card") {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["@type"],
          "description": "@type must be \"Card\""
      }));
   }
   if !object
      .get("version")
      .and_then(serde_json::Value::as_str)
      .is_some_and(|version| matches!(version, "1.0" | "2.0"))
   {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["version"],
          "description": "version must be a supported JSContact version (\"1.0\" or \"2.0\")"
      }));
   }
   Ok(())
}

fn random_uid() -> String {
   format!("urn:uuid:{:032x}", rand::random::<u128>())
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContactFilter {
   #[serde(default)]
   in_address_book: Option<String>,
   #[serde(default)]
   uid:             Option<String>,
   #[serde(default)]
   has_member:      Option<String>,
   #[serde(default)]
   kind:            Option<String>,
   #[serde(default)]
   created_before:  Option<chrono::DateTime<chrono::Utc>>,
   #[serde(default)]
   created_after:   Option<chrono::DateTime<chrono::Utc>>,
   #[serde(default)]
   updated_before:  Option<chrono::DateTime<chrono::Utc>>,
   #[serde(default)]
   updated_after:   Option<chrono::DateTime<chrono::Utc>>,
   #[serde(default)]
   text:            Option<String>,
   #[serde(default)]
   name:            Option<String>,
   #[serde(rename = "name/given", default)]
   name_given:      Option<String>,
   #[serde(rename = "name/surname", default)]
   name_surname:    Option<String>,
   #[serde(rename = "name/surname2", default)]
   name_surname2:   Option<String>,
   #[serde(default)]
   nickname:        Option<String>,
   #[serde(default)]
   organization:    Option<String>,
   #[serde(default)]
   email:           Option<String>,
   #[serde(default)]
   phone:           Option<String>,
   #[serde(default)]
   online_service:  Option<String>,
   #[serde(default)]
   address:         Option<String>,
   #[serde(default)]
   note:            Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryArgs {
   #[serde(rename = "accountId")]
   account_id:      AccountId,
   #[serde(default)]
   filter:          Option<Filter<ContactFilter>>,
   #[serde(default)]
   sort:            Option<Vec<Comparator>>,
   #[serde(default)]
   position:        Option<i64>,
   #[serde(default)]
   anchor:          Option<Id>,
   #[serde(rename = "anchorOffset", default)]
   anchor_offset:   Option<i64>,
   #[serde(default)]
   limit:           Option<u32>,
   #[serde(rename = "calculateTotal", default)]
   calculate_total: bool,
}

#[derive(Debug, Deserialize)]
struct QueryChangesArgs {
   #[serde(rename = "accountId")]
   account_id:        AccountId,
   #[serde(default)]
   filter:            Option<Filter<ContactFilter>>,
   #[serde(default)]
   sort:              Option<Vec<Comparator>>,
   #[serde(rename = "sinceQueryState")]
   since_query_state: State,
   #[serde(rename = "maxChanges", default)]
   max_changes:       Option<u32>,
   #[serde(rename = "upToId", default)]
   up_to_id:          Option<Id>,
   #[serde(rename = "calculateTotal", default)]
   calculate_total:   bool,
}

fn validate_filter_fields(filter: Option<&serde_json::Value>) -> Result<(), MethodError> {
   const FIELDS: &[&str] = &[
      "inAddressBook",
      "uid",
      "hasMember",
      "kind",
      "createdBefore",
      "createdAfter",
      "updatedBefore",
      "updatedAfter",
      "text",
      "name",
      "name/given",
      "name/surname",
      "name/surname2",
      "nickname",
      "organization",
      "email",
      "phone",
      "onlineService",
      "address",
      "note",
   ];
   if filter.is_some_and(|filter| has_unsupported_fields(filter, FIELDS)) {
      Err(MethodError::UnsupportedFilter)
   } else {
      Ok(())
   }
}

async fn load_contacts(
   state: &AppState,
   account_id: &str,
) -> Result<(Vec<ResourceRow>, i64), MethodError> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("DAV db pool: {error}")))?;
   let rows = store::list_resources_by_kind(&client, account_id, DavKind::CardDav)
      .await
      .map_err(|error| server_fail(format!("loading contacts: {error}")))?;
   let current = store::get_state(&client, account_id)
      .await
      .map_err(|error| server_fail(format!("loading DAV state: {error}")))?
      .contact_card_modseq;
   Ok((rows, current))
}

fn filter_and_sort(
   rows: Vec<ResourceRow>,
   filter: Option<&Filter<ContactFilter>>,
   sort: Option<&[Comparator]>,
) -> Result<Vec<ResourceRow>, MethodError> {
   validate_sort(sort)?;
   let mut rows = rows
      .into_iter()
      .map(|row| {
         let value = serde_json::from_str::<serde_json::Value>(&row.json).map_err(|error| {
            server_fail(format!(
               "stored ContactCard JSON for {} is invalid: {error}",
               row.id
            ))
         })?;
         Ok((row, value))
      })
      .collect::<Result<Vec<_>, MethodError>>()?;
   rows.retain(|(row, value)| filter.is_none_or(|filter| contact_matches(row, value, filter)));
   rows.sort_by(|(left, left_json), (right, right_json)| {
      compare_contacts(left, left_json, right, right_json, sort)
   });
   Ok(rows.into_iter().map(|(row, _)| row).collect())
}

fn contact_matches(
   row: &ResourceRow,
   value: &serde_json::Value,
   filter: &Filter<ContactFilter>,
) -> bool {
   match filter {
      Filter::Operator {
         operator,
         conditions,
      } => {
         match operator {
            FilterOp::And => {
               conditions
                  .iter()
                  .all(|child| contact_matches(row, value, child))
            },
            FilterOp::Or => {
               conditions
                  .iter()
                  .any(|child| contact_matches(row, value, child))
            },
            FilterOp::Not => {
               !conditions
                  .iter()
                  .any(|child| contact_matches(row, value, child))
            },
         }
      },
      Filter::Condition(condition) => contact_condition(row, value, condition),
   }
}

fn contact_condition(
   row: &ResourceRow,
   value: &serde_json::Value,
   condition: &ContactFilter,
) -> bool {
   let contains = |haystack: &serde_json::Value, needle: &Option<String>| {
      needle
         .as_ref()
         .is_none_or(|needle| text_matches_json(haystack, needle))
   };
   let created = json_datetime(value, "created");
   let updated = json_datetime(value, "updated");
   condition
      .in_address_book
      .as_ref()
      .is_none_or(|id| id == &row.collection_id)
      && condition.uid.as_ref().is_none_or(|uid| uid == &row.uid)
      && condition.has_member.as_ref().is_none_or(|uid| {
         value
            .get("members")
            .and_then(serde_json::Value::as_object)
            .and_then(|members| members.get(uid))
            .and_then(serde_json::Value::as_bool)
            == Some(true)
      })
      && condition
         .kind
         .as_ref()
         .is_none_or(|kind| value.get("kind").and_then(serde_json::Value::as_str) == Some(kind))
      && condition
         .created_before
         .as_ref()
         .is_none_or(|before| created.as_ref().is_some_and(|value| value < before))
      && condition
         .created_after
         .as_ref()
         .is_none_or(|after| created.as_ref().is_some_and(|value| value >= after))
      && condition
         .updated_before
         .as_ref()
         .is_none_or(|before| updated.as_ref().is_some_and(|value| value < before))
      && condition
         .updated_after
         .as_ref()
         .is_none_or(|after| updated.as_ref().is_some_and(|value| value >= after))
      && contains(value, &condition.text)
      && name_matches(value, condition.name.as_deref())
      && string_contains(
         name_component(value, "given"),
         condition.name_given.as_deref(),
      )
      && string_contains(
         name_component(value, "surname"),
         condition.name_surname.as_deref(),
      )
      && string_contains(
         name_component(value, "surname2"),
         condition.name_surname2.as_deref(),
      )
      && map_fields_match(value, "nicknames", &["name"], condition.nickname.as_deref())
      && map_fields_match(
         value,
         "organizations",
         &["name"],
         condition.organization.as_deref(),
      )
      && map_fields_match(
         value,
         "emails",
         &["address", "label"],
         condition.email.as_deref(),
      )
      && map_fields_match(
         value,
         "phones",
         &["number", "label"],
         condition.phone.as_deref(),
      )
      && map_fields_match(
         value,
         "onlineServices",
         &["service", "uri", "user", "label"],
         condition.online_service.as_deref(),
      )
      && address_matches(value, condition.address.as_deref())
      && map_fields_match(value, "notes", &["note"], condition.note.as_deref())
}

fn name_matches(value: &serde_json::Value, needle: Option<&str>) -> bool {
   let Some(needle) = needle else {
      return true;
   };
   let mut values = Vec::<&serde_json::Value>::new();
   if let Some(full) = value.pointer("/name/full") {
      values.push(full);
   }
   if let Some(components) = value
      .pointer("/name/components")
      .and_then(serde_json::Value::as_array)
   {
      values.extend(
         components
            .iter()
            .filter_map(|component| component.get("value")),
      );
   }
   text_matches_json_values(&values, needle)
}

fn map_fields_match(
   value: &serde_json::Value,
   property: &str,
   fields: &[&str],
   needle: Option<&str>,
) -> bool {
   let Some(needle) = needle else {
      return true;
   };
   let values = value
      .get(property)
      .and_then(serde_json::Value::as_object)
      .into_iter()
      .flat_map(|objects| objects.values())
      .flat_map(|object| fields.iter().filter_map(|field| object.get(*field)))
      .collect::<Vec<&serde_json::Value>>();
   text_matches_json_values(&values, needle)
}

fn address_matches(value: &serde_json::Value, needle: Option<&str>) -> bool {
   let Some(needle) = needle else {
      return true;
   };
   let mut values = Vec::<&serde_json::Value>::new();
   if let Some(addresses) = value
      .get("addresses")
      .and_then(serde_json::Value::as_object)
   {
      for address in addresses.values() {
         if let Some(full) = address.get("full") {
            values.push(full);
         }
         if let Some(components) = address
            .get("components")
            .and_then(serde_json::Value::as_array)
         {
            values.extend(
               components
                  .iter()
                  .filter_map(|component| component.get("value")),
            );
         }
      }
   }
   text_matches_json_values(&values, needle)
}

fn string_contains(haystack: &str, needle: Option<&str>) -> bool {
   needle.is_none_or(|needle| text_matches_str(haystack, needle))
}

fn json_datetime(
   value: &serde_json::Value,
   property: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
   value
      .get(property)
      .and_then(serde_json::Value::as_str)
      .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
      .map(|value| value.with_timezone(&chrono::Utc))
}

fn name_component<'a>(value: &'a serde_json::Value, kind: &str) -> &'a str {
   value
      .pointer("/name/components")
      .and_then(|value| value.as_array())
      .and_then(|components| {
         components
            .iter()
            .find(|component| component.get("kind").and_then(|value| value.as_str()) == Some(kind))
      })
      .and_then(|component| component.get("value"))
      .and_then(|value| value.as_str())
      .unwrap_or("")
}

fn validate_sort(sort: Option<&[Comparator]>) -> Result<(), MethodError> {
   for comparator in sort.unwrap_or(&[]) {
      if comparator.collation.is_some()
         || !comparator.extra.is_empty()
         || !matches!(
            comparator.property.as_str(),
            "created"
               | "updated"
               | "name"
               | "name/given"
               | "name/surname"
               | "name/surname2"
               | "uid"
         )
      {
         return Err(MethodError::UnsupportedSort);
      }
   }
   Ok(())
}

fn compare_contacts(
   left: &ResourceRow,
   left_json: &serde_json::Value,
   right: &ResourceRow,
   right_json: &serde_json::Value,
   sort: Option<&[Comparator]>,
) -> Ordering {
   for comparator in sort.unwrap_or(&[]) {
      let ordering = match comparator.property.as_str() {
         "created" => {
            json_datetime(left_json, "created").cmp(&json_datetime(right_json, "created"))
         },
         "updated" => {
            json_datetime(left_json, "updated").cmp(&json_datetime(right_json, "updated"))
         },
         "uid" => left.uid.cmp(&right.uid),
         "name" => json_string(left_json, "/name/full").cmp(&json_string(right_json, "/name/full")),
         "name/given" => {
            name_component(left_json, "given").cmp(name_component(right_json, "given"))
         },
         "name/surname" => {
            name_component(left_json, "surname").cmp(name_component(right_json, "surname"))
         },
         "name/surname2" => {
            name_component(left_json, "surname2").cmp(name_component(right_json, "surname2"))
         },
         _ => Ordering::Equal,
      };
      let ordering = if comparator.is_ascending {
         ordering
      } else {
         ordering.reverse()
      };
      if ordering != Ordering::Equal {
         return ordering;
      }
   }
   left.id.cmp(&right.id)
}

fn json_string(value: &serde_json::Value, pointer: &str) -> String {
   value
      .pointer(pointer)
      .and_then(|value| value.as_str())
      .unwrap_or("")
      .to_lowercase()
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn contact_filter_handles_nested_boolean_trees() {
      let value = serde_json::json!({"name":{"full":"Ada Lovelace"}});
      let row = ResourceRow {
         account_id:     "a".into(),
         id:             "1".into(),
         collection_id:  "book".into(),
         kind:           "card".into(),
         href:           "/c.vcf".into(),
         etag:           None,
         uid:            "u1".into(),
         raw:            String::new(),
         json:           value.to_string(),
         created_modseq: 1,
         modseq:         1,
         destroyed:      false,
      };
      let filter = Filter::Operator {
         operator:   FilterOp::And,
         conditions: vec![
            Filter::Condition(ContactFilter {
               in_address_book: Some("book".into()),
               ..ContactFilter::default()
            }),
            Filter::Condition(ContactFilter {
               name: Some("ada".into()),
               ..ContactFilter::default()
            }),
         ],
      };
      assert!(contact_matches(&row, &value, &filter));
   }

   #[test]
   fn contact_filter_requires_every_unquoted_token() {
      let value = serde_json::json!({
          "name": {"full": "Ada Lovelace"},
          "organizations": {"one": {"name": "Royal Society"}}
      });
      let row = ResourceRow {
         account_id:     "a".into(),
         id:             "1".into(),
         collection_id:  "book".into(),
         kind:           "card".into(),
         href:           "/c.vcf".into(),
         etag:           None,
         uid:            "u1".into(),
         raw:            String::new(),
         json:           value.to_string(),
         created_modseq: 1,
         modseq:         1,
         destroyed:      false,
      };

      assert!(contact_condition(&row, &value, &ContactFilter {
         text: Some("society ada".into()),
         ..ContactFilter::default()
      }));
      assert!(!contact_condition(&row, &value, &ContactFilter {
         text: Some("society grace".into()),
         ..ContactFilter::default()
      }));
   }

   #[test]
   fn contact_field_filters_ignore_structural_and_unrelated_values() {
      let value = serde_json::json!({
          "name": {
              "@type": "Name",
              "components": [{"kind": "given", "value": "Ada"}]
          },
          "organizations": {
              "one": {
                  "@type": "Organization",
                  "name": "Analytical Engines",
                  "units": [{"name": "Royal Society"}]
              }
          },
          "emails": {
              "one": {
                  "@type": "EmailAddress",
                  "address": "ada@example.test",
                  "contexts": {"private": true}
              }
          }
      });
      let row = ResourceRow {
         account_id:     "a".into(),
         id:             "1".into(),
         collection_id:  "book".into(),
         kind:           "card".into(),
         href:           "/c.vcf".into(),
         etag:           None,
         uid:            "u1".into(),
         raw:            String::new(),
         json:           value.to_string(),
         created_modseq: 1,
         modseq:         1,
         destroyed:      false,
      };

      for filter in [
         ContactFilter {
            name: Some("Name".into()),
            ..ContactFilter::default()
         },
         ContactFilter {
            organization: Some("Royal Society".into()),
            ..ContactFilter::default()
         },
         ContactFilter {
            email: Some("EmailAddress".into()),
            ..ContactFilter::default()
         },
      ] {
         assert!(!contact_condition(&row, &value, &filter));
      }
   }

   #[test]
   fn contact_filter_supports_the_rfc_9610_condition_surface() {
      let value = serde_json::json!({
          "uid": "group-1",
          "kind": "group",
          "created": "2026-01-02T03:04:05Z",
          "updated": "2026-02-03T04:05:06Z",
          "members": {"member-1": true},
          "name": {
              "full": "Analytical Engine Group",
              "components": [
                  {"kind": "given", "value": "Ada"},
                  {"kind": "surname", "value": "Lovelace"}
              ]
          },
          "nicknames": {"n": {"name": "Enchantress of Numbers"}},
          "organizations": {"o": {"name": "Royal Society"}},
          "emails": {"e": {"address": "ada@example.test"}},
          "phones": {"p": {"number": "+44 20 0000"}},
          "onlineServices": {"s": {"service": "example", "user": "ada"}},
          "addresses": {"a": {"full": "London"}},
          "notes": {"n": {"note": "Mathematician"}}
      });
      let row = ResourceRow {
         account_id:     "a".into(),
         id:             "1".into(),
         collection_id:  "book".into(),
         kind:           "card".into(),
         href:           "/c.vcf".into(),
         etag:           None,
         uid:            "group-1".into(),
         raw:            String::new(),
         json:           value.to_string(),
         created_modseq: 99,
         modseq:         100,
         destroyed:      false,
      };
      let filter = ContactFilter {
         in_address_book: Some("book".into()),
         uid: Some("group-1".into()),
         has_member: Some("member-1".into()),
         kind: Some("group".into()),
         created_before: Some("2026-01-03T00:00:00Z".parse().unwrap()),
         created_after: Some("2026-01-01T00:00:00Z".parse().unwrap()),
         updated_before: Some("2026-02-04T00:00:00Z".parse().unwrap()),
         updated_after: Some("2026-02-03T00:00:00Z".parse().unwrap()),
         text: Some("mathematician".into()),
         name: Some("engine".into()),
         name_given: Some("ada".into()),
         name_surname: Some("love".into()),
         nickname: Some("numbers".into()),
         organization: Some("royal".into()),
         email: Some("example.test".into()),
         phone: Some("20 0000".into()),
         online_service: Some("example".into()),
         address: Some("london".into()),
         note: Some("matic".into()),
         ..ContactFilter::default()
      };

      assert!(contact_condition(&row, &value, &filter));
   }

   #[test]
   fn contact_timestamp_sort_uses_jscontact_values_not_modseqs() {
      let newer = ResourceRow {
         account_id:     "a".into(),
         id:             "newer".into(),
         collection_id:  "book".into(),
         kind:           "card".into(),
         href:           "/newer.vcf".into(),
         etag:           None,
         uid:            "newer".into(),
         raw:            String::new(),
         json:           String::new(),
         created_modseq: 1,
         modseq:         1,
         destroyed:      false,
      };
      let older = ResourceRow {
         id: "older".into(),
         uid: "older".into(),
         href: "/older.vcf".into(),
         created_modseq: 2,
         modseq: 2,
         ..newer.clone()
      };
      let sort = [Comparator {
         property:     "created".into(),
         is_ascending: true,
         collation:    None,
         extra:        serde_json::Map::new(),
      }];

      assert_eq!(
         compare_contacts(
            &newer,
            &serde_json::json!({"created":"2026-02-01T00:00:00Z"}),
            &older,
            &serde_json::json!({"created":"2026-01-01T00:00:00Z"}),
            Some(&sort),
         ),
         Ordering::Greater
      );
   }

   #[test]
   fn contact_card_requires_client_supplied_type_and_supported_version() {
      let missing_type = card_raw(serde_json::json!({
          "version": "2.0",
          "uid": "uid"
      }))
      .unwrap_err();
      assert_eq!(missing_type["properties"], serde_json::json!(["@type"]));

      let missing_version = card_raw(serde_json::json!({
          "@type": "Card",
          "uid": "uid"
      }))
      .unwrap_err();
      assert_eq!(
         missing_version["properties"],
         serde_json::json!(["version"])
      );

      for version in ["1.0", "2.0"] {
         card_raw(serde_json::json!({
             "@type": "Card",
             "version": version,
             "uid": "uid",
             "name": {"full": "Ada"}
         }))
         .unwrap();
      }
   }
}
