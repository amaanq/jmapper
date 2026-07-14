//! Shared DAV-backed JMAP mechanics: capability/auth checks, lazy initial
//! synchronization, state parsing, property projection, and RFC 8620
//! `PatchObject` application.

use std::{
   collections::{
      BTreeMap,
      HashMap,
      HashSet,
   },
   mem,
};

use dav_sync::{
   error::DavError,
   service::DavHandle,
   store::{
      self,
      DavKind,
      DavState,
   },
};
use jmap_protocol::{
   error::MethodError,
   ids::{
      AccountId,
      Id,
      State,
   },
   methods::{
      AddedItem,
      ChangesRequest,
      ChangesResponse,
      GetRequest,
      GetResponse,
   },
   session::MAX_OBJECTS_IN_GET,
};
use serde::{
   Deserialize,
   Serialize,
};

use super::{
   MethodResult,
   bad_args,
   enforce_get_limit,
   enforce_set_limit,
   object_or_null,
   query_anchor_position,
   query_position,
   require_auth_match,
   server_fail,
};
use crate::state::{
   AccountInfo,
   AppState,
};

pub const MAX_GET_OBJECTS: usize = MAX_OBJECTS_IN_GET;
const SNAPSHOT_TTL_SECONDS: i64 = 3_600;

#[derive(Clone, Copy)]
pub enum DavObject {
   Calendar,
   CalendarEvent,
   AddressBook,
   ContactCard,
}

impl DavObject {
   pub const fn name(self) -> &'static str {
      match self {
         Self::Calendar => "Calendar",
         Self::CalendarEvent => "CalendarEvent",
         Self::AddressBook => "AddressBook",
         Self::ContactCard => "ContactCard",
      }
   }

   pub const fn kind(self) -> DavKind {
      match self {
         Self::Calendar | Self::CalendarEvent => DavKind::CalDav,
         Self::AddressBook | Self::ContactCard => DavKind::CardDav,
      }
   }

   const fn is_collection(self) -> bool {
      matches!(self, Self::Calendar | Self::AddressBook)
   }

   pub const fn state(self, state: &DavState) -> i64 {
      match self {
         Self::Calendar => state.calendar_modseq,
         Self::CalendarEvent => state.calendar_event_modseq,
         Self::AddressBook => state.addressbook_modseq,
         Self::ContactCard => state.contact_card_modseq,
      }
   }
}

#[derive(Debug, Deserialize)]
pub struct DavSetArgs {
   #[serde(rename = "accountId")]
   pub account_id:               AccountId,
   #[serde(rename = "ifInState", default)]
   pub if_in_state:              Option<State>,
   #[serde(default)]
   pub create:                   BTreeMap<String, serde_json::Value>,
   #[serde(default)]
   pub update:                   BTreeMap<String, serde_json::Value>,
   #[serde(default)]
   pub destroy:                  Vec<String>,
   #[serde(rename = "sendSchedulingMessages", default)]
   pub send_scheduling_messages: bool,
}

pub async fn collection_get(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
   object: DavObject,
   render: impl Fn(&store::CollectionRow, bool) -> serde_json::Value,
) -> MethodResult {
   debug_assert!(
      object.is_collection(),
      "collection_get requires a collection object"
   );
   let req = serde_json::from_value::<GetRequest>(args)
      .map_err(|error| bad_args(format!("invalid {}/get args: {error}", object.name())))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, object.kind())?;
   ensure_initial_sync(state, account_id, object.kind(), &handle).await?;
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   }

   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("DAV db pool: {error}")))?;
   let rows = store::list_collections(&client, account_id, object.kind())
      .await
      .map_err(|error| server_fail(format!("loading DAV collections: {error}")))?;
   if req.ids.is_none() {
      enforce_get_limit(rows.len())?;
   }
   let default_id = rows.first().map(|row| row.id.as_str());
   let mut by_id = rows
      .iter()
      .map(|row| {
         (
            row.id.as_str(),
            render(row, default_id == Some(row.id.as_str())),
         )
      })
      .collect::<BTreeMap<&str, serde_json::Value>>();
   let (list, not_found) = select_objects(req.ids.as_deref(), &mut by_id);
   let current = object.state(
      &store::get_state(&client, account_id)
         .await
         .map_err(|error| server_fail(format!("loading DAV state: {error}")))?,
   );
   let mut response = serde_json::to_value(GetResponse::<serde_json::Value> {
      account_id: AccountId(account_id.to_owned()),
      state: State(current.to_string()),
      list,
      not_found,
   })
   .map_err(|error| server_fail(error.to_string()))?;
   project_properties(&mut response, req.properties.as_deref());
   Ok(response)
}

pub async fn changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
   object: DavObject,
) -> MethodResult {
   let method = format!("{}/changes", object.name());
   let req = serde_json::from_value::<ChangesRequest>(args)
      .map_err(|error| bad_args(format!("invalid {method} args: {error}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, object.kind())?;
   ensure_initial_sync(state, account_id, object.kind(), &handle).await?;
   let current = object.state(&dav_state(state, account_id).await?);
   let since = parse_since_state(req.since_state.as_ref(), current)?;
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("DAV db pool: {error}")))?;
   let changed = if object.is_collection() {
      store::collections_changed_since(&client, account_id, object.kind(), since)
         .await
         .map(|rows| {
            rows
               .into_iter()
               .map(|row| (row.id, row.created_modseq, row.destroyed))
               .collect::<Vec<(String, i64, bool)>>()
         })
   } else {
      store::resources_changed_since(&client, account_id, object.kind(), since)
         .await
         .map(|rows| {
            rows
               .into_iter()
               .map(|row| (row.id, row.created_modseq, row.destroyed))
               .collect::<Vec<(String, i64, bool)>>()
         })
   }
   .map_err(|error| server_fail(format!("loading {method} rows: {error}")))?;
   let (created, updated, destroyed) = classify_changes(changed, since);
   if req
      .max_changes
      .is_some_and(|max| created.len() + updated.len() + destroyed.len() > max as usize)
   {
      return Err(MethodError::CannotCalculateChanges);
   }
   serde_json::to_value(ChangesResponse {
      account_id: AccountId(account_id.to_owned()),
      old_state: req.since_state,
      new_state: State(current.to_string()),
      has_more_changes: false,
      created,
      updated,
      destroyed,
   })
   .map_err(|error| server_fail(error.to_string()))
}

pub async fn read_only_collection_set(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
   object: DavObject,
) -> MethodResult {
   debug_assert!(
      object.is_collection(),
      "read_only_collection_set requires a collection object"
   );
   let req = serde_json::from_value::<DavSetArgs>(args)
      .map_err(|error| bad_args(format!("invalid {}/set args: {error}", object.name())))?;
   let account_id = req.account_id.as_ref();
   require_dav(state, auth, account_id, object.kind())?;
   enforce_set_limit(req.create.len(), req.update.len(), req.destroy.len())?;
   let current = object
      .state(&dav_state(state, account_id).await?)
      .to_string();
   check_if_in_state(req.if_in_state.as_ref(), &current)?;
   let error = set_error("forbidden", "DAV collection mutation is not supported");
   Ok(rejected_set_response(
      account_id,
      &current,
      req.create.keys(),
      req.update.keys().map(String::as_str),
      req.destroy.iter().map(String::as_str),
      [&error, &error, &error],
   ))
}

pub fn check_if_in_state(requested: Option<&State>, current: &str) -> Result<(), MethodError> {
   if requested.is_some_and(|state| state.as_ref() != current) {
      Err(MethodError::StateMismatch)
   } else {
      Ok(())
   }
}

pub fn rejected_set_response<'a>(
   account_id: &str,
   state: &str,
   create: impl Iterator<Item = &'a String>,
   update: impl Iterator<Item = &'a str>,
   destroy: impl Iterator<Item = &'a str>,
   errors: [&serde_json::Value; 3],
) -> serde_json::Value {
   let not_created = create
      .map(|id| (id.clone(), errors[0].clone()))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   let not_updated = update
      .map(|id| (id.to_owned(), errors[1].clone()))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   let not_destroyed = destroy
      .map(|id| (id.to_owned(), errors[2].clone()))
      .collect::<serde_json::Map<String, serde_json::Value>>();
   serde_json::json!({
       "accountId": account_id,
       "oldState": state,
       "newState": state,
       "created": null,
       "notCreated": object_or_null(not_created),
       "updated": null,
       "notUpdated": object_or_null(not_updated),
       "destroyed": null,
       "notDestroyed": object_or_null(not_destroyed),
   })
}

pub fn select_objects<T>(ids: Option<&[Id]>, by_id: &mut BTreeMap<&str, T>) -> (Vec<T>, Vec<Id>) {
   let Some(ids) = ids else {
      return (mem::take(by_id).into_values().collect(), Vec::new());
   };
   let mut list = Vec::with_capacity(ids.len());
   let mut not_found = Vec::new();
   for id in ids {
      match by_id.remove(id.as_ref()) {
         Some(value) => list.push(value),
         None => not_found.push(id.clone()),
      }
   }
   (list, not_found)
}

pub fn require_dav(
   state: &AppState,
   auth: &AccountInfo,
   account_id: &str,
   kind: DavKind,
) -> Result<DavHandle, MethodError> {
   require_auth_match(auth, account_id)?;
   let available = state.dav_availability(account_id);
   let supported = match kind {
      DavKind::CalDav => available.calendars,
      DavKind::CardDav => available.contacts,
   };
   if !supported {
      return Err(MethodError::AccountNotSupportedByMethod);
   }
   state
      .dav_handle(account_id)
      .ok_or(MethodError::ServerUnavailable)
}

/// Ensure a newly configured endpoint has populated its cache before the
/// first read. Steady-state freshness comes from the daemon's periodic loop.
pub async fn ensure_initial_sync(
   state: &AppState,
   account_id: &str,
   kind: DavKind,
   handle: &DavHandle,
) -> Result<(), MethodError> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|err| server_fail(format!("DAV db pool: {err}")))?;
   let endpoint = store::get_endpoint(&client, account_id, kind)
      .await
      .map_err(|err| server_fail(format!("loading DAV endpoint: {err}")))?
      .ok_or(MethodError::AccountNotSupportedByMethod)?;
   if endpoint.last_sync_at.is_some() {
      return Ok(());
   }
   drop(client);
   handle
      .sync_now(kind, false)
      .await
      .map_err(|err| server_fail(format!("initial {} sync: {err}", kind.as_str())))?;
   Ok(())
}

pub async fn dav_state(state: &AppState, account_id: &str) -> Result<DavState, MethodError> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|err| server_fail(format!("DAV db pool: {err}")))?;
   store::get_state(&client, account_id)
      .await
      .map_err(|err| server_fail(format!("loading DAV state: {err}")))
}

pub fn parse_since_state(value: &str, current: i64) -> Result<i64, MethodError> {
   let parsed = value
      .parse::<i64>()
      .ok()
      .filter(|value| *value >= 0 && *value <= current)
      .ok_or(MethodError::CannotCalculateChanges)?;
   Ok(parsed)
}

pub fn project_properties(value: &mut serde_json::Value, properties: Option<&[String]>) {
   let Some(properties) = properties else {
      return;
   };
   let mut allowed = properties
      .iter()
      .map(String::as_str)
      .collect::<HashSet<&str>>();
   allowed.insert("id");
   if let Some(list) = value
      .get_mut("list")
      .and_then(serde_json::Value::as_array_mut)
   {
      for object in list {
         if let Some(object) = object.as_object_mut() {
            object.retain(|key, _| allowed.contains(key.as_str()));
         }
      }
   }
}

pub fn set_error(kind: &str, description: impl Into<String>) -> serde_json::Value {
   serde_json::json!({"type": kind, "description": description.into()})
}

pub fn dav_set_error(error: DavError) -> serde_json::Value {
   match error {
      DavError::PreconditionFailed { .. } => {
         set_error(
            "serverFail",
            "the remote object changed concurrently; refresh and retry",
         )
      },
      other => set_error("serverFail", other.to_string()),
   }
}

pub fn classify_changes(rows: Vec<(String, i64, bool)>, since: i64) -> (Vec<Id>, Vec<Id>, Vec<Id>) {
   let mut created = Vec::new();
   let mut updated = Vec::new();
   let mut destroyed = Vec::new();
   for (id, created_modseq, is_destroyed) in rows {
      if is_destroyed {
         if created_modseq <= since {
            destroyed.push(Id(id));
         }
      } else if created_modseq > since {
         created.push(Id(id));
      } else {
         updated.push(Id(id));
      }
   }
   (created, updated, destroyed)
}

pub fn take_single_membership(
   value: &mut serde_json::Value,
   object_name: &str,
   property: &str,
   member_name: &str,
) -> Result<String, serde_json::Value> {
   let object = value.as_object_mut().ok_or_else(|| {
      set_error(
         "invalidProperties",
         format!("{object_name} must be an object"),
      )
   })?;
   let memberships = object.remove(property).ok_or_else(|| {
      serde_json::json!({
          "type": "invalidProperties",
          "properties": [property],
          "description": format!("exactly one {member_name} is required"),
      })
   })?;
   let memberships = memberships.as_object().ok_or_else(|| {
      set_error(
         "invalidProperties",
         format!("{property} must be an object of id to boolean"),
      )
   })?;
   let selected = memberships
      .iter()
      .filter_map(|(id, enabled)| enabled.as_bool().map(|enabled| (id, enabled)))
      .filter(|(_, enabled)| *enabled)
      .map(|(id, _)| id.clone())
      .collect::<Vec<String>>();
   if memberships.values().any(|enabled| !enabled.is_boolean()) {
      return Err(set_error(
         "invalidProperties",
         format!("{property} values must be booleans"),
      ));
   }
   if selected.len() != 1 {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": [property],
          "description": format!("exactly one {member_name} must be true"),
      }));
   }
   Ok(selected.into_iter().next().expect("length checked"))
}

pub async fn validate_collection(
   state: &AppState,
   account_id: &str,
   collection_id: &str,
   kind: DavKind,
   property: &str,
   description: &str,
) -> Result<(), serde_json::Value> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| set_error("serverFail", format!("DAV db pool: {error}")))?;
   let valid = store::get_collection(&client, account_id, collection_id)
      .await
      .map_err(|error| set_error("serverFail", error.to_string()))?
      .is_some_and(|row| !row.destroyed && row.kind == kind.collection_kind());
   if valid {
      Ok(())
   } else {
      Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": [property],
          "description": description,
      }))
   }
}

pub fn query_window(
   ids: &[String],
   position: Option<i64>,
   anchor: Option<&Id>,
   anchor_offset: Option<i64>,
   limit: usize,
) -> Result<(i64, Vec<String>), MethodError> {
   let start = if let Some(anchor) = anchor {
      let anchor = ids
         .iter()
         .position(|id| id == anchor.as_ref())
         .ok_or(MethodError::AnchorNotFound)?;
      query_anchor_position(anchor, anchor_offset)
   } else {
      query_position(position, ids.len())
   };
   Ok((
      i64::try_from(start).unwrap_or(i64::MAX),
      ids.iter().skip(start).take(limit).cloned().collect(),
   ))
}

pub fn query_hash<T>(value: &T, object_name: &str) -> Result<String, MethodError>
where
   T: Serialize,
{
   use sha1::{
      Digest as _,
      Sha1,
   };
   let encoded = serde_json::to_vec(value)
      .map_err(|error| server_fail(format!("serializing {object_name} query: {error}")))?;
   let mut hash = Sha1::new();
   hash.update(encoded);
   Ok(hex::encode(hash.finalize()))
}

pub async fn save_query_snapshot(
   state: &AppState,
   account_id: &str,
   snapshot_kind: &str,
   hash: &str,
   modseq: i64,
   ids: &[String],
) -> Result<(), MethodError> {
   let now = chrono::Utc::now().timestamp();
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("DAV db pool: {error}")))?;
   store::save_query_snapshot(
      &client,
      account_id,
      snapshot_kind,
      hash,
      modseq,
      ids,
      now,
      now + SNAPSHOT_TTL_SECONDS,
   )
   .await
   .map_err(|error| server_fail(format!("saving {snapshot_kind} query snapshot: {error}")))
}

pub struct QueryChanges<'a> {
   pub account_id:      &'a str,
   pub snapshot_kind:   &'a str,
   pub hash:            &'a str,
   pub since:           i64,
   pub current:         i64,
   pub new_ids:         &'a [String],
   pub max_changes:     Option<u32>,
   pub calculate_total: bool,
}

pub async fn query_changes_response(state: &AppState, query: QueryChanges<'_>) -> MethodResult {
   let QueryChanges {
      account_id,
      snapshot_kind,
      hash,
      since,
      current,
      new_ids,
      max_changes,
      calculate_total,
   } = query;
   let now = chrono::Utc::now().timestamp();
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("DAV db pool: {error}")))?;
   let old = store::get_query_snapshot(&client, account_id, snapshot_kind, hash, since, now)
      .await
      .map_err(|error| server_fail(format!("loading query snapshot: {error}")))?
      .ok_or(MethodError::CannotCalculateChanges)?;
   let (removed, added) = query_changes_diff(&old.ids, new_ids);
   if max_changes.is_some_and(|max| removed.len() + added.len() > max as usize) {
      return Err(MethodError::TooManyChanges);
   }
   store::save_query_snapshot(
      &client,
      account_id,
      snapshot_kind,
      hash,
      current,
      new_ids,
      now,
      now + SNAPSHOT_TTL_SECONDS,
   )
   .await
   .map_err(|error| server_fail(format!("saving {snapshot_kind} query snapshot: {error}")))?;
   Ok(serde_json::json!({
       "accountId": account_id,
       "oldQueryState": since.to_string(),
       "newQueryState": current.to_string(),
       "total": calculate_total.then_some(new_ids.len() as u64),
       "removed": removed,
       "added": added,
   }))
}

pub fn text_matches_json(value: &serde_json::Value, query: &str) -> bool {
   text_matches_json_values(&[value], query)
}

pub fn text_matches_json_values(values: &[&serde_json::Value], query: &str) -> bool {
   search_terms(query)
      .iter()
      .all(|term| values.iter().any(|value| json_contains_term(value, term)))
}

pub fn text_matches_str(value: &str, query: &str) -> bool {
   let value = value.to_lowercase();
   search_terms(query).iter().all(|term| value.contains(term))
}

fn search_terms(query: &str) -> Vec<String> {
   let mut terms = Vec::<String>::new();
   let mut current = String::new();
   let mut quote = None::<char>;
   let mut escaped = false;

   for character in query.chars() {
      if escaped {
         current.push(character);
         escaped = false;
         continue;
      }
      if character == '\\' {
         escaped = true;
         continue;
      }
      if let Some(delimiter) = quote {
         if character == delimiter {
            push_search_term(&mut terms, &mut current);
            quote = None;
         } else {
            current.push(character);
         }
      } else if matches!(character, '\'' | '"') {
         push_search_term(&mut terms, &mut current);
         quote = Some(character);
      } else if character.is_whitespace() {
         push_search_term(&mut terms, &mut current);
      } else {
         current.push(character);
      }
   }
   if escaped {
      current.push('\\');
   }
   push_search_term(&mut terms, &mut current);
   terms
}

fn push_search_term(terms: &mut Vec<String>, current: &mut String) {
   if !current.is_empty() {
      terms.push(mem::take(current).to_lowercase());
   }
}

fn json_contains_term(value: &serde_json::Value, lowercase_term: &str) -> bool {
   match value {
      serde_json::Value::String(value) => value.to_lowercase().contains(lowercase_term),
      serde_json::Value::Array(values) => {
         values
            .iter()
            .any(|value| json_contains_term(value, lowercase_term))
      },
      serde_json::Value::Object(values) => {
         values
            .values()
            .any(|value| json_contains_term(value, lowercase_term))
      },
      _ => false,
   }
}

/// Return the remove/add edit set that transforms one ordered query result
/// into another. IDs retained by a longest common subsequence are not
/// reported merely because an insertion or deletion shifted their index;
/// IDs that genuinely moved are both removed and added.
pub fn query_changes_diff(old: &[String], new: &[String]) -> (Vec<Id>, Vec<AddedItem>) {
   let old_positions = old
      .iter()
      .enumerate()
      .map(|(index, id)| (id.as_str(), index))
      .collect::<HashMap<&str, usize>>();
   let common = new
      .iter()
      .enumerate()
      .filter_map(|(new_index, id)| {
         old_positions
            .get(id.as_str())
            .map(|old_index| (new_index, *old_index))
      })
      .collect::<Vec<(usize, usize)>>();

   // Query IDs are unique, so an LIS over their old positions is an LCS of
   // the old/new ID lists. `tails` stores indexes into `common`.
   let mut tails = Vec::<usize>::new();
   let mut predecessors = vec![None; common.len()];
   for (index, &(_, old_index)) in common.iter().enumerate() {
      let insert_at = tails.partition_point(|tail| common[*tail].1 < old_index);
      if insert_at > 0 {
         predecessors[index] = Some(tails[insert_at - 1]);
      }
      if insert_at == tails.len() {
         tails.push(index);
      } else {
         tails[insert_at] = index;
      }
   }

   let mut retained_new_indexes = HashSet::<usize>::new();
   let mut cursor = tails.last().copied();
   while let Some(index) = cursor {
      retained_new_indexes.insert(common[index].0);
      cursor = predecessors[index];
   }
   let retained_ids = retained_new_indexes
      .iter()
      .map(|index| new[*index].as_str())
      .collect::<HashSet<&str>>();

   let removed = old
      .iter()
      .filter(|id| !retained_ids.contains(id.as_str()))
      .cloned()
      .map(Id)
      .collect::<Vec<Id>>();
   let added = new
      .iter()
      .enumerate()
      .filter(|(index, _)| !retained_new_indexes.contains(index))
      .map(|(index, id)| {
         AddedItem {
            id:    Id(id.clone()),
            index: index as u32,
         }
      })
      .collect::<Vec<AddedItem>>();
   (removed, added)
}

/// Apply an RFC 8620 `PatchObject` to a JSON object. Paths use JSON Pointer
/// escaping with an implicit leading slash and may not descend into arrays.
pub fn apply_patch_object(
   current: &serde_json::Value,
   patch: &serde_json::Value,
   server_set: &[&str],
) -> Result<serde_json::Value, serde_json::Value> {
   let patch = patch
      .as_object()
      .ok_or_else(|| set_error("invalidPatch", "PatchObject must be a JSON object"))?;
   let paths = patch
      .keys()
      .map(|path| decode_pointer(path))
      .collect::<Result<Vec<Vec<String>>, serde_json::Value>>()?;

   for (index, path) in paths.iter().enumerate() {
      for other in paths.iter().skip(index + 1) {
         if is_prefix(path, other) || is_prefix(other, path) {
            return Err(set_error(
               "invalidPatch",
               "PatchObject contains overlapping paths",
            ));
         }
      }
   }

   let mut output = current.clone();
   for ((raw_path, replacement), path) in patch.iter().zip(paths) {
      if path.len() == 1 && server_set.contains(&path[0].as_str()) {
         if current.get(&path[0]) == Some(replacement) {
            continue;
         }
         return Err(serde_json::json!({
             "type": "invalidProperties",
             "properties": [raw_path],
             "description": format!("{} is server-set", path[0]),
         }));
      }
      let (last, parents) = path
         .split_last()
         .ok_or_else(|| set_error("invalidPatch", "empty patch path"))?;
      let mut parent = &mut output;
      for segment in parents {
         parent = parent
            .as_object_mut()
            .and_then(|object| object.get_mut(segment))
            .ok_or_else(|| {
               set_error(
                  "invalidPatch",
                  format!("parent of patch path {raw_path:?} does not exist"),
               )
            })?;
         if parent.is_array() {
            return Err(set_error(
               "invalidPatch",
               format!("patch path {raw_path:?} references inside an array"),
            ));
         }
      }
      let parent = parent.as_object_mut().ok_or_else(|| {
         set_error(
            "invalidPatch",
            format!("parent of patch path {raw_path:?} is not an object"),
         )
      })?;
      if replacement.is_null() {
         parent.remove(last);
      } else {
         parent.insert(last.clone(), replacement.clone());
      }
   }
   Ok(output)
}

fn decode_pointer(path: &str) -> Result<Vec<String>, serde_json::Value> {
   if path.is_empty() {
      return Err(set_error("invalidPatch", "patch paths must not be empty"));
   }
   path
      .split('/')
      .map(|segment| {
         let mut decoded = String::with_capacity(segment.len());
         let mut chars = segment.chars();
         while let Some(ch) = chars.next() {
            if ch != '~' {
               decoded.push(ch);
               continue;
            }
            match chars.next() {
               Some('0') => decoded.push('~'),
               Some('1') => decoded.push('/'),
               _ => {
                  return Err(set_error(
                     "invalidPatch",
                     format!("patch path {path:?} has invalid JSON Pointer escaping"),
                  ));
               },
            }
         }
         Ok(decoded)
      })
      .collect()
}

fn is_prefix(left: &[String], right: &[String]) -> bool {
   left.len() < right.len() && left.iter().zip(right).all(|(lhs, rhs)| lhs == rhs)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn patch_object_applies_nested_paths_and_rejects_overlap() {
      let current = serde_json::json!({
          "id": "one",
          "name": {"full": "Old", "parts": ["Old"]},
          "emails": {"work/a": {"address": "old@example.test"}}
      });
      let patched = apply_patch_object(
         &current,
         &serde_json::json!({
             "name/full": "New",
             "emails/work~1a/address": "new@example.test"
         }),
         &["id"],
      )
      .unwrap();
      assert_eq!(patched["name"]["full"], "New");
      assert_eq!(patched["emails"]["work/a"]["address"], "new@example.test");

      apply_patch_object(
         &current,
         &serde_json::json!({"name": {}, "name/full": "x"}),
         &["id"],
      )
      .unwrap_err();
      apply_patch_object(&current, &serde_json::json!({"name/parts/0": "x"}), &["id"]).unwrap_err();
   }

   #[test]
   fn query_diff_ignores_index_shifts_from_insertions_and_deletions() {
      let old = vec!["a".into(), "b".into(), "c".into()];
      let new = vec!["x".into(), "a".into(), "c".into()];

      let (removed, added) = query_changes_diff(&old, &new);
      assert_eq!(removed, vec![Id("b".into())]);
      assert_eq!(added.len(), 1);
      assert_eq!(added[0].id, Id("x".into()));
      assert_eq!(added[0].index, 0);
   }

   #[test]
   fn query_diff_reports_a_sort_order_move() {
      let old = vec!["a".into(), "b".into()];
      let new = vec!["b".into(), "a".into()];

      let (removed, added) = query_changes_diff(&old, &new);
      assert_eq!(removed, vec![Id("b".into())]);
      assert_eq!(added.len(), 1);
      assert_eq!(added[0].id, Id("b".into()));
      assert_eq!(added[0].index, 0);
   }

   #[test]
   fn changes_omit_objects_created_and_destroyed_since_the_old_state() {
      let (created, updated, destroyed) = classify_changes(
         vec![
            ("ephemeral".into(), 3, true),
            ("old-destroyed".into(), 1, true),
            ("new".into(), 4, false),
            ("changed".into(), 1, false),
         ],
         2,
      );
      assert_eq!(created, vec![Id("new".into())]);
      assert_eq!(updated, vec![Id("changed".into())]);
      assert_eq!(destroyed, vec![Id("old-destroyed".into())]);
   }

   #[test]
   fn text_matching_supports_tokens_phrases_and_escapes() {
      let value = serde_json::json!({
          "name": "Ada Lovelace",
          "organization": "Royal Society",
          "note": "the 'engine'"
      });

      assert!(text_matches_json(&value, "society ada"));
      assert!(text_matches_json(&value, "\"Ada Lovelace\" royal"));
      assert!(text_matches_json(&value, "'the \\'engine\\''"));
      assert!(!text_matches_json(&value, "\"Lovelace Royal\""));
   }
}
