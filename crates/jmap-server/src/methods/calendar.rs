//! DAV-backed Calendar and `CalendarEvent` methods.

use std::{
   cmp::Ordering,
   collections::BTreeMap,
};

use chrono::{
   DateTime,
   Duration,
   LocalResult,
   NaiveDateTime,
   TimeZone as _,
   Utc,
};
use chrono_tz::Tz;
use dav_sync::{
   convert::calendar::{
      event_to_ical,
      expand_event_occurrences,
   },
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
      rejected_set_response,
      require_dav,
      save_query_snapshot,
      select_objects,
      set_error,
      take_single_membership,
      text_matches_json,
      text_matches_json_values,
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

const SNAPSHOT_KIND: &str = "CalendarEvent";

/// # Errors
///
/// Returns a [`MethodError`] if the account has no `CalDAV` configuration, the
/// initial sync fails, or loading the calendar collections from the DAV store
/// fails.
pub async fn calendar_get(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   collection_get(state, auth, args, DavObject::Calendar, calendar_object).await
}

/// # Errors
///
/// Returns a [`MethodError`] if the account has no `CalDAV` configuration or
/// the DAV store cannot compute the changes since the supplied state.
pub async fn calendar_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   changes(state, auth, args, DavObject::Calendar).await
}

/// # Errors
///
/// Returns a [`MethodError`] if the account has no `CalDAV` configuration. The
/// Calendar collection is read-only, so create, update, and destroy requests
/// are rejected per object in the response rather than as a method error.
pub async fn calendar_set(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   read_only_collection_set(state, auth, args, DavObject::Calendar).await
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, the
/// account has no `CalDAV` configuration, `utcStart` or `utcEnd` is requested
/// together with `recurrenceOverrides`, the requested id count exceeds the
/// limit, or reading events or DAV state from the store fails.
pub async fn event_get(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let req = serde_json::from_value::<EventGetArgs>(args)
      .map_err(|err| bad_args(format!("invalid CalendarEvent/get args: {err}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CalDav)?;
   ensure_initial_sync(state, account_id, DavKind::CalDav, &handle).await?;
   let fallback_timezone = parse_query_timezone(&req.time_zone)?;
   let wants_utc = req.properties.as_ref().is_some_and(|properties| {
      properties
         .iter()
         .any(|property| matches!(property.as_str(), "utcStart" | "utcEnd"))
   });
   if wants_utc
      && req.properties.as_ref().is_some_and(|properties| {
         properties
            .iter()
            .any(|property| property == "recurrenceOverrides")
      })
   {
      return Err(bad_args(
         "recurrenceOverrides cannot be requested with utcStart or utcEnd",
      ));
   }
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   }
   let conn = state
      .pool()
      .get()
      .await
      .map_err(|err| server_fail(format!("DAV db pool: {err}")))?;
   let rows = store::list_resources_by_kind(&conn, account_id, DavKind::CalDav)
      .await
      .map_err(|err| server_fail(format!("loading calendar events: {err}")))?;
   if req.ids.is_none() {
      enforce_get_limit(rows.len())?;
   }
   let mut by_id = rows
      .iter()
      .map(|row| {
         let mut event = event_object(row, auth)?;
         prepare_event_for_get(
            &mut event,
            auth,
            req.recurrence_overrides_before.as_ref(),
            req.recurrence_overrides_after.as_ref(),
            req.reduce_participants,
            fallback_timezone,
            req.properties.as_deref(),
         )?;
         Ok((row.id.as_str(), event))
      })
      .collect::<Result<BTreeMap<&str, serde_json::Value>, MethodError>>()?;
   let (list, not_found) = select_objects(req.ids.as_deref(), &mut by_id);
   let current = store::get_state(&conn, account_id)
      .await
      .map_err(|err| server_fail(format!("loading DAV state: {err}")))?
      .calendar_event_modseq;
   let mut response = serde_json::to_value(GetResponse::<serde_json::Value> {
      account_id: AccountId(account_id.to_owned()),
      state: State(current.to_string()),
      list,
      not_found,
   })
   .map_err(|err| server_fail(err.to_string()))?;
   project_properties(&mut response, req.properties.as_deref());
   Ok(response)
}

/// # Errors
///
/// Returns a [`MethodError`] if the account has no `CalDAV` configuration or
/// the DAV store cannot compute the changes since the supplied state.
pub async fn event_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   changes(state, auth, args, DavObject::CalendarEvent).await
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed, the
/// account has no `CalDAV` configuration, the `ifInState` guard does not match
/// the current state, or the requested change count exceeds the limit. Per-
/// object create, update, and destroy failures are reported in the response
/// rather than as a method error.
pub async fn event_set(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let req = serde_json::from_value::<DavSetArgs>(args)
      .map_err(|err| bad_args(format!("invalid CalendarEvent/set args: {err}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CalDav)?;
   ensure_initial_sync(state, account_id, DavKind::CalDav, &handle).await?;
   let old_state = dav_state(state, account_id).await?.calendar_event_modseq;
   super::dav_common::check_if_in_state(req.if_in_state.as_ref(), &old_state.to_string())?;
   enforce_set_limit(req.create.len(), req.update.len(), req.destroy.len())?;
   if req.send_scheduling_messages {
      let error = set_error(
         "noSupportedScheduleMethods",
         "this DAV bridge does not send scheduling messages",
      );
      return Ok(rejected_set_response(
         account_id,
         &old_state.to_string(),
         req.create.keys(),
         req.update.keys().map(String::as_str),
         req.destroy.iter().map(String::as_str),
         [&error, &error, &error],
      ));
   }

   let mut created = serde_json::Map::new();
   let mut not_created = serde_json::Map::new();
   let mut updated = Vec::<String>::new();
   let mut not_updated = serde_json::Map::new();
   let mut destroyed = Vec::<String>::new();
   let mut not_destroyed = serde_json::Map::new();
   for (creation_id, value) in req.create {
      match create_event(state, auth, account_id, &handle, value).await {
         Ok(row) => {
            created.insert(creation_id, serde_json::json!({"id": row.id}));
         },
         Err(error) => {
            not_created.insert(creation_id, error);
         },
      }
   }
   for (id, patch) in req.update {
      match update_event(state, auth, account_id, &handle, &id, &patch).await {
         Ok(()) => updated.push(id),
         Err(error) => {
            not_updated.insert(id, error);
         },
      }
   }
   for id in req.destroy {
      match destroy_event(state, account_id, &handle, &id).await {
         Ok(()) => destroyed.push(id),
         Err(error) => {
            not_destroyed.insert(id, error);
         },
      }
   }
   let new_state = dav_state(state, account_id).await?.calendar_event_modseq;
   if new_state != old_state {
      state.publish_state_change(StateChange {
         account_id: account_id.to_owned(),
         kind:       StateKind::CalendarEvent,
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
/// Returns a [`MethodError`] if the filter uses unsupported fields, the
/// arguments are malformed, the account has no `CalDAV` configuration,
/// `expandRecurrences` is requested, the query timezone is invalid, or reading
/// events from the store fails.
pub async fn event_query(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_filter_fields(args.get("filter"))?;
   let req = serde_json::from_value::<QueryArgs>(args)
      .map_err(|err| bad_args(format!("invalid CalendarEvent/query args: {err}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CalDav)?;
   ensure_initial_sync(state, account_id, DavKind::CalDav, &handle).await?;
   if req.expand_recurrences {
      match req.filter.as_ref() {
         Some(Filter::Condition(condition))
            if condition.after.is_some() && condition.before.is_some() =>
         {
            return Err(MethodError::ExpandDurationTooLarge);
         },
         _ => {
            return Err(bad_args(
               "expandRecurrences requires a condition with after and before",
            ));
         },
      }
   }
   let query_timezone = parse_query_timezone(&req.time_zone)?;
   let (rows, current) = load_events(state, account_id).await?;
   let ordered = filter_and_sort(
      rows,
      req.filter.as_ref(),
      req.sort.as_deref(),
      query_timezone,
   )?;
   let all_ids = ordered
      .iter()
      .map(|row| row.id.clone())
      .collect::<Vec<String>>();
   let hash = query_hash(
      &(req.filter.as_ref(), req.sort.as_deref(), &req.time_zone),
      SNAPSHOT_KIND,
   )?;
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
   .map_err(|err| server_fail(err.to_string()))
}

/// # Errors
///
/// Returns a [`MethodError`] if the filter uses unsupported fields, the
/// arguments are malformed, the account has no `CalDAV` configuration, `upToId`
/// or `expandRecurrences` is requested (changes cannot be calculated), the
/// query timezone is invalid, or reading events from the store fails.
pub async fn event_query_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_filter_fields(args.get("filter"))?;
   let req = serde_json::from_value::<QueryChangesArgs>(args)
      .map_err(|err| bad_args(format!("invalid CalendarEvent/queryChanges args: {err}")))?;
   let account_id = req.account_id.as_ref();
   let handle = require_dav(state, auth, account_id, DavKind::CalDav)?;
   ensure_initial_sync(state, account_id, DavKind::CalDav, &handle).await?;
   if req.up_to_id.is_some() || req.expand_recurrences {
      return Err(MethodError::CannotCalculateChanges);
   }
   let query_timezone = parse_query_timezone(&req.time_zone)?;
   let (rows, current) = load_events(state, account_id).await?;
   let since = parse_since_state(req.since_query_state.as_ref(), current)?;
   let hash = query_hash(
      &(req.filter.as_ref(), req.sort.as_deref(), &req.time_zone),
      SNAPSHOT_KIND,
   )?;
   let ordered = filter_and_sort(
      rows,
      req.filter.as_ref(),
      req.sort.as_deref(),
      query_timezone,
   )?;
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

async fn create_event(
   state: &AppState,
   auth: &AccountInfo,
   account_id: &str,
   handle: &DavHandle,
   mut value: serde_json::Value,
) -> Result<ResourceRow, serde_json::Value> {
   let supplied_server_set = ["id", "isOrigin", "isDraft", "baseEventId"]
      .into_iter()
      .filter(|property| value.get(*property).is_some_and(|value| !value.is_null()))
      .collect::<Vec<&str>>();
   if !supplied_server_set.is_empty() {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": supplied_server_set,
          "description": "property is server-set"
      }));
   }
   let collection_id =
      take_single_membership(&mut value, "CalendarEvent", "calendarIds", "calendarId")?;
   validate_collection(
      state,
      account_id,
      &collection_id,
      DavKind::CalDav,
      "calendarIds",
      "unknown calendar",
   )
   .await?;
   let now = chrono::Utc::now();
   let now_text = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
   {
      let object = value
         .as_object_mut()
         .ok_or_else(|| set_error("invalidProperties", "CalendarEvent must be an object"))?;
      object.remove("id");
      object
         .entry("@type".to_owned())
         .or_insert_with(|| serde_json::Value::String("Event".to_owned()));
      object
         .entry("uid".to_owned())
         .or_insert_with(|| serde_json::Value::String(random_uid()));
      object
         .entry("created".to_owned())
         .or_insert_with(|| serde_json::Value::String(now_text.clone()));
   }
   if event_is_origin(&value, auth) {
      let object = value.as_object_mut().expect("event object checked above");
      let created_is_future = object
         .get("created")
         .and_then(serde_json::Value::as_str)
         .and_then(|created| DateTime::parse_from_rfc3339(created).ok())
         .is_some_and(|created| created > now);
      if created_is_future {
         object.insert(
            "created".to_owned(),
            serde_json::Value::String(now_text.clone()),
         );
      }
      object.insert("updated".to_owned(), serde_json::Value::String(now_text));
   }
   let raw = event_raw(value)?;
   handle
      .put_resource(DavKind::CalDav, collection_id, None, raw)
      .await
      .map_err(dav_set_error)
}

async fn update_event(
   state: &AppState,
   auth: &AccountInfo,
   account_id: &str,
   handle: &DavHandle,
   id: &str,
   patch: &serde_json::Value,
) -> Result<(), serde_json::Value> {
   let conn = state
      .pool()
      .get()
      .await
      .map_err(|err| set_error("serverFail", format!("DAV db pool: {err}")))?;
   let row = store::get_resource(&conn, account_id, id)
      .await
      .map_err(|err| set_error("serverFail", err.to_string()))?
      .filter(|row| !row.destroyed && row.kind == DavKind::CalDav.resource_kind())
      .ok_or_else(|| set_error("notFound", "calendar event not found"))?;
   drop(conn);
   let current =
      event_object(&row, auth).map_err(|err| set_error("serverFail", format!("{err:?}")))?;
   let mut updated = apply_patch_object(&current, patch, &[
      "id",
      "isOrigin",
      "isDraft",
      "baseEventId",
   ])?;
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
      "CalendarEvent",
      "calendarIds",
      "calendarId",
   )?;
   let collection_id =
      take_single_membership(&mut updated, "CalendarEvent", "calendarIds", "calendarId")?;
   validate_collection(
      state,
      account_id,
      &collection_id,
      DavKind::CalDav,
      "calendarIds",
      "unknown calendar",
   )
   .await?;
   let moving = row.collection_id != collection_id;
   let content_changed = current_content != updated;
   if moving && content_changed {
      return Err(set_error(
         "invalidPatch",
         "moving and editing a DAV event in one update is not atomic; submit separate updates",
      ));
   }
   if !content_changed {
      if moving {
         handle
            .move_resource(DavKind::CalDav, id.to_owned(), collection_id)
            .await
            .map_err(dav_set_error)?;
      }
      return Ok(());
   }
   if event_is_origin(&current, auth) && patch_changes_shared_event_data(patch) {
      let current_sequence = current
         .get("sequence")
         .and_then(serde_json::Value::as_u64)
         .unwrap_or_default();
      let requested_sequence = updated
         .get("sequence")
         .and_then(serde_json::Value::as_u64)
         .unwrap_or_default();
      if requested_sequence <= current_sequence {
         updated.as_object_mut().expect("event object").insert(
            "sequence".to_owned(),
            serde_json::Value::from(current_sequence.saturating_add(1)),
         );
      }
   }
   if event_is_origin(&current, auth) {
      updated.as_object_mut().expect("event object").insert(
         "updated".to_owned(),
         serde_json::Value::String(
            chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
         ),
      );
   }
   let raw = event_raw(updated)?;
   handle
      .put_resource(DavKind::CalDav, collection_id, Some(id.to_owned()), raw)
      .await
      .map_err(dav_set_error)?;
   Ok(())
}

async fn destroy_event(
   state: &AppState,
   account_id: &str,
   handle: &DavHandle,
   id: &str,
) -> Result<(), serde_json::Value> {
   let conn = state
      .pool()
      .get()
      .await
      .map_err(|err| set_error("serverFail", format!("DAV db pool: {err}")))?;
   let found = store::get_resource(&conn, account_id, id)
      .await
      .map_err(|err| set_error("serverFail", err.to_string()))?
      .is_some_and(|row| !row.destroyed && row.kind == DavKind::CalDav.resource_kind());
   drop(conn);
   if !found {
      return Err(set_error("notFound", "calendar event not found"));
   }
   handle
      .delete_resource(DavKind::CalDav, id.to_owned())
      .await
      .map_err(dav_set_error)
}

fn event_object(row: &ResourceRow, auth: &AccountInfo) -> Result<serde_json::Value, MethodError> {
   let mut value = serde_json::from_str::<serde_json::Value>(&row.json)
      .map_err(|err| server_fail(format!("stored CalendarEvent JSON is invalid: {err}")))?;
   let is_origin = event_is_origin(&value, auth);
   let object = value
      .as_object_mut()
      .ok_or_else(|| server_fail("stored CalendarEvent JSON is not an object"))?;
   object.insert("id".to_owned(), serde_json::Value::String(row.id.clone()));
   object.insert(
      "calendarIds".to_owned(),
      serde_json::json!({row.collection_id.clone(): true}),
   );
   object.insert("isDraft".to_owned(), serde_json::Value::Bool(false));
   object.insert("isOrigin".to_owned(), serde_json::Value::Bool(is_origin));
   object.insert("baseEventId".to_owned(), serde_json::Value::Null);
   Ok(value)
}

fn prepare_event_for_get(
   event: &mut serde_json::Value,
   auth: &AccountInfo,
   overrides_before: Option<&DateTime<Utc>>,
   overrides_after: Option<&DateTime<Utc>>,
   reduce_participants: bool,
   fallback_timezone: Tz,
   properties: Option<&[String]>,
) -> Result<(), MethodError> {
   if overrides_before.is_some() || overrides_after.is_some() {
      let timezone = event_timezone(event, fallback_timezone)
         .ok_or_else(|| server_fail("stored CalendarEvent has an invalid timeZone"))?;
      let remove = event
         .get("recurrenceOverrides")
         .and_then(serde_json::Value::as_object)
         .into_iter()
         .flat_map(|overrides| overrides.keys())
         .filter_map(|recurrence_id| {
            let utc = local_datetime_utc(recurrence_id, timezone);
            let keep = utc.is_some_and(|utc| {
               overrides_before.is_none_or(|before| utc < *before)
                  && overrides_after.is_none_or(|after| utc >= *after)
            });
            (!keep).then(|| recurrence_id.clone())
         })
         .collect::<Vec<String>>();
      if let Some(overrides) = event
         .get_mut("recurrenceOverrides")
         .and_then(serde_json::Value::as_object_mut)
      {
         for recurrence_id in remove {
            overrides.remove(&recurrence_id);
         }
      }
   }

   let reduce_participants = reduce_participants
      || (event
         .get("hideAttendees")
         .and_then(serde_json::Value::as_bool)
         == Some(true)
         && !event_is_origin(event, auth));
   if reduce_participants {
      if let Some(participants) = event
         .get_mut("participants")
         .and_then(serde_json::Value::as_object_mut)
      {
         retain_visible_participants(participants, auth);
      }
      if let Some(overrides) = event
         .get_mut("recurrenceOverrides")
         .and_then(serde_json::Value::as_object_mut)
      {
         for patch in overrides.values_mut() {
            if let Some(participants) = patch
               .get_mut("participants")
               .and_then(serde_json::Value::as_object_mut)
            {
               retain_visible_participants(participants, auth);
            }
         }
      }
   }

   let wants_utc_start =
      properties.is_some_and(|properties| properties.iter().any(|property| property == "utcStart"));
   let wants_utc_end =
      properties.is_some_and(|properties| properties.iter().any(|property| property == "utcEnd"));
   if wants_utc_start || wants_utc_end {
      let start = event_start_utc(event, fallback_timezone);
      let end = start.and_then(|start| {
         event
            .get("duration")
            .and_then(serde_json::Value::as_str)
            .and_then(parse_event_duration)
            .and_then(|duration| start.checked_add_signed(duration))
            .or(Some(start))
      });
      let object = event.as_object_mut().expect("event object");
      if wants_utc_start {
         object.insert("utcStart".to_owned(), utc_json_value(start));
      }
      if wants_utc_end {
         object.insert("utcEnd".to_owned(), utc_json_value(end));
      }
   }
   Ok(())
}

fn retain_visible_participants(
   participants: &mut serde_json::Map<String, serde_json::Value>,
   auth: &AccountInfo,
) {
   participants.retain(|_, participant| {
      participant
         .pointer("/roles/owner")
         .and_then(serde_json::Value::as_bool)
         == Some(true)
         || participant_is_self(participant, auth)
   });
}

fn participant_is_self(participant: &serde_json::Value, auth: &AccountInfo) -> bool {
   participant
      .get("email")
      .and_then(serde_json::Value::as_str)
      .is_some_and(|email| email.eq_ignore_ascii_case(&auth.email))
      || participant
         .get("calendarAddress")
         .and_then(serde_json::Value::as_str)
         .is_some_and(|address| {
            address
               .strip_prefix("mailto:")
               .or_else(|| address.strip_prefix("MAILTO:"))
               .unwrap_or(address)
               .eq_ignore_ascii_case(&auth.email)
         })
}

fn utc_json_value(value: Option<DateTime<Utc>>) -> serde_json::Value {
   value.map_or(serde_json::Value::Null, |value| {
      serde_json::Value::String(value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
   })
}

fn event_is_origin(value: &serde_json::Value, auth: &AccountInfo) -> bool {
   value
      .get("organizerCalendarAddress")
      .and_then(serde_json::Value::as_str)
      .is_none_or(|address| {
         address
            .strip_prefix("mailto:")
            .or_else(|| address.strip_prefix("MAILTO:"))
            .unwrap_or(address)
            .eq_ignore_ascii_case(&auth.email)
      })
}

fn patch_changes_shared_event_data(patch: &serde_json::Value) -> bool {
   const NON_SHARED: &[&str] = &[
      "calendarIds",
      "isDraft",
      "updated",
      "keywords",
      "color",
      "freeBusyStatus",
      "useDefaultAlerts",
      "alerts",
   ];
   patch.as_object().is_some_and(|patch| {
      patch.keys().any(|path| {
         let root = path.split('/').next().unwrap_or(path);
         !NON_SHARED.contains(&root)
      })
   })
}

fn calendar_object(row: &store::CollectionRow, is_default: bool) -> serde_json::Value {
   serde_json::json!({
       "id": row.id,
       "name": row.name,
       "description": row.description,
       "color": row.color,
       "sortOrder": 0,
       "isSubscribed": true,
       "isVisible": true,
       "isDefault": is_default,
       "includeInAvailability": "all",
       "defaultAlertsWithTime": null,
       "defaultAlertsWithoutTime": null,
       "timeZone": null,
       "shareWith": null,
       "myRights": {
           "mayReadFreeBusy": true,
           "mayReadItems": true,
           "mayWriteAll": true,
           "mayWriteOwn": true,
           "mayUpdatePrivate": true,
           "mayRSVP": true,
           "mayShare": false,
           "mayDelete": false
       }
   })
}

fn event_raw(mut value: serde_json::Value) -> Result<String, serde_json::Value> {
   let object = value
      .as_object_mut()
      .ok_or_else(|| set_error("invalidProperties", "CalendarEvent must be an object"))?;
   let invalid = ["method", "utcStart", "utcEnd"]
      .into_iter()
      .filter(|property| object.contains_key(*property))
      .collect::<Vec<&str>>();
   if !invalid.is_empty() {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": invalid,
          "description": "this CalendarEvent property cannot be stored by the DAV bridge"
      }));
   }
   for property in ["id", "calendarIds", "isOrigin", "isDraft", "baseEventId"] {
      object.remove(property);
   }
   validate_event_metadata(object)?;
   event_to_ical(&value).map_err(|err| {
      serde_json::json!({
          "type": "invalidProperties",
          "description": format!("cannot convert JSCalendar Event: {err}")
      })
   })
}

fn validate_event_metadata(
   object: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), serde_json::Value> {
   if object.get("@type").and_then(serde_json::Value::as_str) != Some("Event") {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["@type"],
          "description": "@type must be \"Event\""
      }));
   }
   if object.get("version").and_then(serde_json::Value::as_str) != Some("2.0") {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["version"],
          "description": "version must be \"2.0\" for JSCalendar 2.0 fields"
      }));
   }
   Ok(())
}

fn random_uid() -> String {
   format!("{:032x}@jmapper", rand::random::<u128>())
}

#[derive(Debug, Deserialize)]
struct EventGetArgs {
   #[serde(rename = "accountId")]
   account_id:                  AccountId,
   #[serde(default)]
   ids:                         Option<Vec<Id>>,
   #[serde(default)]
   properties:                  Option<Vec<String>>,
   #[serde(rename = "recurrenceOverridesBefore", default)]
   recurrence_overrides_before: Option<DateTime<Utc>>,
   #[serde(rename = "recurrenceOverridesAfter", default)]
   recurrence_overrides_after:  Option<DateTime<Utc>>,
   #[serde(rename = "reduceParticipants", default)]
   reduce_participants:         bool,
   #[serde(rename = "timeZone", default = "default_query_timezone")]
   time_zone:                   String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EventFilter {
   #[serde(default)]
   in_calendar: Option<String>,
   #[serde(default)]
   uid:         Option<String>,
   #[serde(default)]
   after:       Option<NaiveDateTime>,
   #[serde(default)]
   before:      Option<NaiveDateTime>,
   #[serde(default)]
   text:        Option<String>,
   #[serde(default)]
   title:       Option<String>,
   #[serde(default)]
   description: Option<String>,
   #[serde(default)]
   location:    Option<String>,
   #[serde(default)]
   owner:       Option<String>,
   #[serde(default)]
   attendee:    Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryArgs {
   #[serde(rename = "accountId")]
   account_id:         AccountId,
   #[serde(default)]
   filter:             Option<Filter<EventFilter>>,
   #[serde(default)]
   sort:               Option<Vec<Comparator>>,
   #[serde(default)]
   position:           Option<i64>,
   #[serde(default)]
   anchor:             Option<Id>,
   #[serde(rename = "anchorOffset", default)]
   anchor_offset:      Option<i64>,
   #[serde(default)]
   limit:              Option<u32>,
   #[serde(rename = "calculateTotal", default)]
   calculate_total:    bool,
   #[serde(rename = "expandRecurrences", default)]
   expand_recurrences: bool,
   #[serde(rename = "timeZone", default = "default_query_timezone")]
   time_zone:          String,
}

#[derive(Debug, Deserialize)]
struct QueryChangesArgs {
   #[serde(rename = "accountId")]
   account_id:         AccountId,
   #[serde(default)]
   filter:             Option<Filter<EventFilter>>,
   #[serde(default)]
   sort:               Option<Vec<Comparator>>,
   #[serde(rename = "sinceQueryState")]
   since_query_state:  State,
   #[serde(rename = "maxChanges", default)]
   max_changes:        Option<u32>,
   #[serde(rename = "upToId", default)]
   up_to_id:           Option<Id>,
   #[serde(rename = "calculateTotal", default)]
   calculate_total:    bool,
   #[serde(rename = "expandRecurrences", default)]
   expand_recurrences: bool,
   #[serde(rename = "timeZone", default = "default_query_timezone")]
   time_zone:          String,
}

fn default_query_timezone() -> String {
   "Etc/UTC".to_owned()
}

fn validate_filter_fields(filter: Option<&serde_json::Value>) -> Result<(), MethodError> {
   const FIELDS: &[&str] = &[
      "inCalendar",
      "uid",
      "after",
      "before",
      "text",
      "title",
      "description",
      "location",
      "owner",
      "attendee",
   ];
   if filter.is_some_and(|filter| has_unsupported_fields(filter, FIELDS)) {
      Err(MethodError::UnsupportedFilter)
   } else {
      Ok(())
   }
}

async fn load_events(
   state: &AppState,
   account_id: &str,
) -> Result<(Vec<ResourceRow>, i64), MethodError> {
   let conn = state
      .pool()
      .get()
      .await
      .map_err(|err| server_fail(format!("DAV db pool: {err}")))?;
   let rows = store::list_resources_by_kind(&conn, account_id, DavKind::CalDav)
      .await
      .map_err(|err| server_fail(format!("loading events: {err}")))?;
   let current = store::get_state(&conn, account_id)
      .await
      .map_err(|err| server_fail(format!("loading DAV state: {err}")))?
      .calendar_event_modseq;
   Ok((rows, current))
}

fn filter_and_sort(
   rows: Vec<ResourceRow>,
   filter: Option<&Filter<EventFilter>>,
   sort: Option<&[Comparator]>,
   query_timezone: Tz,
) -> Result<Vec<ResourceRow>, MethodError> {
   validate_sort(sort)?;
   let rows = rows
      .into_iter()
      .map(|row| {
         let value = serde_json::from_str::<serde_json::Value>(&row.json).map_err(|error| {
            server_fail(format!(
               "stored CalendarEvent JSON for {} is invalid: {error}",
               row.id
            ))
         })?;
         Ok((row, value))
      })
      .collect::<Result<Vec<_>, MethodError>>()?;
   let mut filtered = Vec::<_>::with_capacity(rows.len());
   for (row, value) in rows {
      if match filter {
         Some(filter) => event_matches(&row, &value, filter, query_timezone)?,
         None => true,
      } {
         filtered.push((row, value));
      }
   }
   let mut rows = filtered;
   rows.sort_by(|(left, left_json), (right, right_json)| {
      compare_events(left, left_json, right, right_json, sort, query_timezone)
   });
   Ok(rows.into_iter().map(|(row, _)| row).collect())
}

fn event_matches(
   row: &ResourceRow,
   value: &serde_json::Value,
   filter: &Filter<EventFilter>,
   query_timezone: Tz,
) -> Result<bool, MethodError> {
   match filter {
      Filter::Operator {
         operator,
         conditions,
      } => {
         match operator {
            FilterOp::And => {
               let mut deferred = None::<MethodError>;
               for child in conditions {
                  match event_matches(row, value, child, query_timezone) {
                     Ok(false) => return Ok(false),
                     Ok(true) => {},
                     Err(error) => deferred = Some(error),
                  }
               }
               deferred.map_or(Ok(true), Err)
            },
            FilterOp::Or | FilterOp::Not => {
               let mut deferred = None::<MethodError>;
               for child in conditions {
                  match event_matches(row, value, child, query_timezone) {
                     Ok(true) => return Ok(matches!(operator, FilterOp::Or)),
                     Ok(false) => {},
                     Err(error) => deferred = Some(error),
                  }
               }
               deferred.map_or_else(|| Ok(matches!(operator, FilterOp::Not)), Err)
            },
         }
      },
      Filter::Condition(condition) => event_condition(row, value, condition, query_timezone),
   }
}

fn event_condition(
   row: &ResourceRow,
   value: &serde_json::Value,
   condition: &EventFilter,
   query_timezone: Tz,
) -> Result<bool, MethodError> {
   let non_date_matches = condition
      .in_calendar
      .as_ref()
      .is_none_or(|id| id == &row.collection_id)
      && condition.uid.as_ref().is_none_or(|uid| uid == &row.uid)
      && event_property_matches(value, "title", condition.title.as_deref())
      && event_property_matches(value, "description", condition.description.as_deref())
      && event_location_matches(value, condition.location.as_deref())
      && participant_matches(value, "owner", condition.owner.as_deref())
      && participant_matches(value, "attendee", condition.attendee.as_deref())
      && condition
         .text
         .as_ref()
         .is_none_or(|needle| text_matches_json(value, needle));
   if !non_date_matches {
      return Ok(false);
   }
   event_date_matches(value, Some(&row.raw), condition, query_timezone)
}

fn event_property_matches(event: &serde_json::Value, property: &str, needle: Option<&str>) -> bool {
   let Some(needle) = needle else {
      return true;
   };
   let mut values = Vec::<&serde_json::Value>::new();
   if let Some(value) = event.get(property) {
      values.push(value);
   }
   if let Some(overrides) = event
      .get("recurrenceOverrides")
      .and_then(serde_json::Value::as_object)
   {
      for patch in overrides.values().filter_map(serde_json::Value::as_object) {
         if patch.get("excluded").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
         }
         values.extend(patch.iter().filter_map(|(path, value)| {
            (path == property
               || path
                  .strip_prefix(property)
                  .is_some_and(|rest| rest.starts_with('/')))
            .then_some(value)
         }));
      }
   }
   text_matches_json_values(&values, needle)
}

fn event_location_matches(event: &serde_json::Value, needle: Option<&str>) -> bool {
   let Some(needle) = needle else {
      return true;
   };
   let mut values = Vec::<&serde_json::Value>::new();
   if let Some(locations) = event.get("locations") {
      collect_location_text(locations, &mut values);
   }
   if let Some(overrides) = event
      .get("recurrenceOverrides")
      .and_then(serde_json::Value::as_object)
   {
      for patch in overrides.values().filter_map(serde_json::Value::as_object) {
         if patch.get("excluded").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
         }
         for (path, value) in patch {
            if path.starts_with("locations/")
               && matches!(path.rsplit('/').next(), Some("name" | "description"))
            {
               values.push(value);
            } else if path == "locations" || path.starts_with("locations/") {
               collect_location_text(value, &mut values);
            }
         }
      }
   }
   text_matches_json_values(&values, needle)
}

fn collect_location_text<'a>(
   value: &'a serde_json::Value,
   values: &mut Vec<&'a serde_json::Value>,
) {
   let Some(object) = value.as_object() else {
      return;
   };
   let mut found_field = false;
   for field in ["name", "description"] {
      if let Some(field_value) = object.get(field) {
         values.push(field_value);
         found_field = true;
      }
   }
   if !found_field {
      for nested_value in object.values() {
         collect_location_text(nested_value, values);
      }
   }
}

fn parse_query_timezone(value: &str) -> Result<Tz, MethodError> {
   value
      .parse::<Tz>()
      .map_err(|_| bad_args(format!("invalid CalendarEvent query timeZone {value:?}")))
}

fn event_date_matches(
   value: &serde_json::Value,
   raw: Option<&str>,
   condition: &EventFilter,
   query_timezone: Tz,
) -> Result<bool, MethodError> {
   if condition.after.is_none() && condition.before.is_none() {
      return Ok(true);
   }
   if value
      .get("recurrenceRule")
      .is_some_and(|rule| !rule.is_null())
   {
      let raw = raw
         .filter(|raw| !raw.is_empty())
         .ok_or(MethodError::CannotCalculateOccurrences)?;
      let (occurrences, truncated) = expand_event_occurrences(raw, query_timezone, 10_000)
         .map_err(|_| MethodError::CannotCalculateOccurrences)?;
      let timestamp = |value: Option<&NaiveDateTime>| {
         value
            .map(|value| {
               naive_datetime_utc(*value, query_timezone)
                  .map(|value| value.timestamp())
                  .ok_or(MethodError::CannotCalculateOccurrences)
            })
            .transpose()
      };
      let after = timestamp(condition.after.as_ref())?;
      let before = timestamp(condition.before.as_ref())?;
      if occurrences.into_iter().any(|(start, end)| {
         after.is_none_or(|after| end > after) && before.is_none_or(|before| start < before)
      }) {
         return Ok(true);
      }
      return if truncated {
         Err(MethodError::CannotCalculateOccurrences)
      } else {
         Ok(false)
      };
   }
   let Some(start) = event_start_in_timezone(value, query_timezone) else {
      return Ok(false);
   };
   let base_duration = value
      .get("duration")
      .and_then(serde_json::Value::as_str)
      .and_then(parse_event_duration)
      .unwrap_or_else(Duration::zero);
   if interval_matches(start, base_duration, condition) {
      return Ok(true);
   }

   if let Some(overrides) = value
      .get("recurrenceOverrides")
      .and_then(serde_json::Value::as_object)
   {
      for (recurrence_id, patch) in overrides {
         let Some(patch) = patch.as_object() else {
            continue;
         };
         if patch.get("excluded").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
         }
         let start = patch
            .get("start")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(recurrence_id);
         let Some(start) = override_start_in_timezone(value, patch, start, query_timezone) else {
            continue;
         };
         let duration = match patch.get("duration") {
            Some(serde_json::Value::String(duration)) => {
               parse_event_duration(duration).unwrap_or_else(Duration::zero)
            },
            Some(serde_json::Value::Null) => Duration::zero(),
            _ => base_duration,
         };
         if interval_matches(start, duration, condition) {
            return Ok(true);
         }
      }
   }

   Ok(false)
}

fn interval_matches(start: NaiveDateTime, duration: Duration, condition: &EventFilter) -> bool {
   let end = start.checked_add_signed(duration).unwrap_or(start);
   condition.after.as_ref().is_none_or(|after| end > *after)
      && condition
         .before
         .as_ref()
         .is_none_or(|before| start < *before)
}

fn override_start_in_timezone(
   event: &serde_json::Value,
   patch: &serde_json::Map<String, serde_json::Value>,
   start: &str,
   query_timezone: Tz,
) -> Option<NaiveDateTime> {
   let start = NaiveDateTime::parse_from_str(start, "%Y-%m-%dT%H:%M:%S").ok()?;
   let timezone = match patch.get("timeZone") {
      Some(serde_json::Value::String(timezone)) => timezone.parse::<Tz>().ok()?,
      Some(serde_json::Value::Null) => query_timezone,
      _ => event_timezone(event, query_timezone)?,
   };
   naive_datetime_utc(start, timezone)
      .map(|start| start.with_timezone(&query_timezone).naive_local())
}

fn event_start_in_timezone(value: &serde_json::Value, query_timezone: Tz) -> Option<NaiveDateTime> {
   event_start_utc(value, query_timezone)
      .map(|start| start.with_timezone(&query_timezone).naive_local())
}

fn event_start_utc(value: &serde_json::Value, fallback_timezone: Tz) -> Option<DateTime<Utc>> {
   let start = value
      .get("start")
      .and_then(serde_json::Value::as_str)
      .and_then(|start| NaiveDateTime::parse_from_str(start, "%Y-%m-%dT%H:%M:%S").ok())?;
   naive_datetime_utc(start, event_timezone(value, fallback_timezone)?)
}

fn event_timezone(value: &serde_json::Value, fallback_timezone: Tz) -> Option<Tz> {
   value
      .get("timeZone")
      .and_then(serde_json::Value::as_str)
      .map(str::parse::<Tz>)
      .transpose()
      .ok()?
      .or(Some(fallback_timezone))
}

fn local_datetime_utc(value: &str, timezone: Tz) -> Option<DateTime<Utc>> {
   let local = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S").ok()?;
   naive_datetime_utc(local, timezone)
}

fn naive_datetime_utc(local: NaiveDateTime, timezone: Tz) -> Option<DateTime<Utc>> {
   let zoned = match timezone.from_local_datetime(&local) {
      LocalResult::Single(value) => value,
      LocalResult::Ambiguous(earlier, _) => earlier,
      LocalResult::None => return None,
   };
   Some(zoned.with_timezone(&Utc))
}

fn parse_event_duration(value: &str) -> Option<Duration> {
   let mut in_time = false;
   let mut digits = String::new();
   let mut seconds = 0_i64;
   let mut saw_component = false;
   for character in value.strip_prefix('P')?.chars() {
      if character == 'T' {
         if in_time || !digits.is_empty() {
            return None;
         }
         in_time = true;
         continue;
      }
      if character.is_ascii_digit() {
         digits.push(character);
         continue;
      }
      let amount = digits.parse::<i64>().ok()?;
      digits.clear();
      let multiplier = match (in_time, character) {
         (false, 'W') => 7 * 86_400,
         (false, 'D') => 86_400,
         (true, 'H') => 3_600,
         (true, 'M') => 60,
         (true, 'S') => 1,
         _ => return None,
      };
      seconds = seconds.checked_add(amount.checked_mul(multiplier)?)?;
      saw_component = true;
   }
   if !digits.is_empty() || !saw_component {
      return None;
   }
   Some(Duration::seconds(seconds))
}

fn participant_matches(event: &serde_json::Value, role: &str, needle: Option<&str>) -> bool {
   let Some(needle) = needle else {
      return true;
   };
   let matches = |participants: &serde_json::Value| {
      participants.as_object().is_some_and(|participants| {
         participants.values().any(|participant| {
            let has_role = participant
               .get("roles")
               .and_then(serde_json::Value::as_object)
               .and_then(|roles| roles.get(role))
               .and_then(serde_json::Value::as_bool)
               == Some(true);
            let values = [participant.get("name"), participant.get("email")]
               .into_iter()
               .flatten()
               .collect::<Vec<&serde_json::Value>>();
            has_role && text_matches_json_values(&values, needle)
         })
      })
   };
   event.get("participants").is_some_and(&matches)
      || event
         .get("recurrenceOverrides")
         .and_then(serde_json::Value::as_object)
         .is_some_and(|overrides| {
            overrides
               .values()
               .any(|patch| patch.get("participants").is_some_and(&matches))
         })
}

fn validate_sort(sort: Option<&[Comparator]>) -> Result<(), MethodError> {
   for comparator in sort.unwrap_or(&[]) {
      if comparator.collation.is_some()
         || !comparator.extra.is_empty()
         || !matches!(
            comparator.property.as_str(),
            "start" | "uid" | "recurrenceId" | "created" | "updated"
         )
      {
         return Err(MethodError::UnsupportedSort);
      }
   }
   Ok(())
}

fn compare_events(
   left: &ResourceRow,
   left_json: &serde_json::Value,
   right: &ResourceRow,
   right_json: &serde_json::Value,
   sort: Option<&[Comparator]>,
   query_timezone: Tz,
) -> Ordering {
   for comparator in sort.unwrap_or(&[]) {
      let ordering = match comparator.property.as_str() {
         "start" => {
            event_start_in_timezone(left_json, query_timezone)
               .cmp(&event_start_in_timezone(right_json, query_timezone))
         },
         "uid" => left.uid.cmp(&right.uid),
         "recurrenceId" => {
            json_string(left_json, "recurrenceId").cmp(&json_string(right_json, "recurrenceId"))
         },
         "created" => {
            json_utc_datetime(left_json, "created").cmp(&json_utc_datetime(right_json, "created"))
         },
         "updated" => {
            json_utc_datetime(left_json, "updated").cmp(&json_utc_datetime(right_json, "updated"))
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

fn json_utc_datetime(value: &serde_json::Value, property: &str) -> Option<DateTime<Utc>> {
   value
      .get(property)
      .and_then(serde_json::Value::as_str)
      .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
      .map(|value| value.with_timezone(&Utc))
}

fn json_string(value: &serde_json::Value, key: &str) -> String {
   value
      .get(key)
      .and_then(|value| value.as_str())
      .unwrap_or("")
      .to_owned()
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn event_filter_supports_boolean_composition() {
      let value = serde_json::json!({"title":"Planning"});
      let row = ResourceRow {
         account_id:     "a".into(),
         id:             "1".into(),
         collection_id:  "cal".into(),
         kind:           "event".into(),
         href:           "/e.ics".into(),
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
            Filter::Condition(EventFilter {
               in_calendar: Some("cal".into()),
               ..EventFilter::default()
            }),
            Filter::Condition(EventFilter {
               title: Some("plan".into()),
               ..EventFilter::default()
            }),
         ],
      };
      assert!(event_matches(&row, &value, &filter, chrono_tz::UTC).unwrap());
   }

   #[test]
   fn event_filter_uses_interval_overlap_in_the_requested_timezone() {
      let value = serde_json::json!({
          "start": "2026-07-15T09:00:00",
          "timeZone": "America/New_York",
          "duration": "PT1H"
      });
      let matching = EventFilter {
         after: Some("2026-07-15T12:30:00".parse().unwrap()),
         before: Some("2026-07-15T13:30:00".parse().unwrap()),
         ..EventFilter::default()
      };
      let not_matching = EventFilter {
         after: Some("2026-07-15T14:00:00".parse().unwrap()),
         ..EventFilter::default()
      };

      assert!(event_date_matches(&value, None, &matching, chrono_tz::UTC).unwrap());
      assert!(!event_date_matches(&value, None, &not_matching, chrono_tz::UTC).unwrap());
   }

   #[test]
   fn recurring_date_filter_expands_icalendar() {
      let value = serde_json::json!({
          "start": "2026-07-02T09:00:00",
          "timeZone": "Etc/UTC",
          "duration": "PT1H",
          "recurrenceRule": {"@type": "RecurrenceRule", "frequency": "weekly"},
          "recurrenceOverrides": {
              "2026-07-16T09:00:00": {"start": "2026-07-16T10:00:00"}
          }
      });
      let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:r1\r\nDTSTART:\
                 20260702T090000Z\r\nDURATION:PT1H\r\nRRULE:FREQ=WEEKLY\r\nEND:VEVENT\r\nBEGIN:\
                 VEVENT\r\nUID:r1\r\nRECURRENCE-ID:20260716T090000Z\r\nDTSTART:20260716T100000Z\r\\
                 \
                 nDURATION:PT1H\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
      let explicit_override = EventFilter {
         after: Some("2026-07-16T09:30:00".parse().unwrap()),
         before: Some("2026-07-16T10:30:00".parse().unwrap()),
         ..EventFilter::default()
      };
      let generated_instance = EventFilter {
         after: Some("2026-08-06T09:30:00".parse().unwrap()),
         before: Some("2026-08-06T10:30:00".parse().unwrap()),
         ..EventFilter::default()
      };

      assert!(event_date_matches(&value, Some(raw), &explicit_override, chrono_tz::UTC).unwrap());
      assert!(event_date_matches(&value, Some(raw), &generated_instance, chrono_tz::UTC).unwrap());
   }

   #[test]
   fn event_filters_match_tokens_and_recurrence_overrides() {
      let value = serde_json::json!({
          "title": "Weekly sync",
          "recurrenceOverrides": {
              "2026-07-15T09:00:00": {
                  "title": "Royal Society planning",
                  "participants": {
                      "owner": {
                          "name": "Ada Lovelace",
                          "email": "ada@example.test",
                          "roles": {"owner": true}
                      }
                  }
              }
          }
      });

      assert!(event_property_matches(
         &value,
         "title",
         Some("planning royal")
      ));
      assert!(!event_property_matches(
         &value,
         "title",
         Some("planning grace")
      ));
      assert!(participant_matches(&value, "owner", Some("lovelace ada")));

      let locations = serde_json::json!({
          "locations": {
              "one": {
                  "@type": "Location",
                  "name": "Room 4",
                  "coordinates": "geo:52.5,13.4"
              }
          },
          "recurrenceOverrides": {
              "2026-07-15T09:00:00": {
                  "locations": {
                      "two": {"description": "Royal Society annex"}
                  }
              }
          }
      });
      assert!(event_location_matches(&locations, Some("annex royal")));
      assert!(!event_location_matches(&locations, Some("geo:52.5")));
   }

   #[test]
   fn participant_filters_honor_roles() {
      let value = serde_json::json!({
          "participants": {
              "owner": {
                  "name": "Ada Owner",
                  "roles": {"owner": true}
              },
              "guest": {
                  "name": "Grace Guest",
                  "roles": {"attendee": true}
              }
          }
      });

      assert!(participant_matches(&value, "owner", Some("ada")));
      assert!(!participant_matches(&value, "owner", Some("grace")));
      assert!(participant_matches(&value, "attendee", Some("grace")));
   }

   #[test]
   fn event_duration_parser_is_checked() {
      assert_eq!(
         parse_event_duration("P1DT2H3M4S").unwrap().num_seconds(),
         93_784
      );
      assert_eq!(parse_event_duration("P2W").unwrap().num_days(), 14);
      assert!(parse_event_duration("PT").is_none());
      assert!(parse_event_duration("P1H").is_none());
   }

   #[test]
   fn event_origin_is_derived_from_the_organizer() {
      let auth = AccountInfo::from_bearer_token(
         "account",
         "owner@example.test",
         "Owner",
         "long-enough-test-token",
      );

      assert!(event_is_origin(&serde_json::json!({}), &auth));
      assert!(event_is_origin(
         &serde_json::json!({"organizerCalendarAddress":"mailto:OWNER@example.test"}),
         &auth
      ));
      assert!(!event_is_origin(
         &serde_json::json!({"organizerCalendarAddress":"mailto:other@example.test"}),
         &auth
      ));
   }

   #[test]
   fn only_shared_event_changes_advance_sequence() {
      assert!(patch_changes_shared_event_data(
         &serde_json::json!({"title":"new"})
      ));
      assert!(!patch_changes_shared_event_data(
         &serde_json::json!({"alerts/a/trigger": {"@type":"OffsetTrigger"}})
      ));
      assert!(!patch_changes_shared_event_data(
         &serde_json::json!({"calendarIds": {"next": true}})
      ));
   }

   #[test]
   fn event_get_computes_utc_and_reduces_participants() {
      let auth = AccountInfo::from_bearer_token(
         "account",
         "self@example.test",
         "Self",
         "long-enough-test-token",
      );
      let mut event = serde_json::json!({
          "start": "2026-07-15T09:00:00",
          "timeZone": "America/New_York",
          "duration": "PT1H",
          "participants": {
              "owner": {"name": "Owner", "roles": {"owner": true}},
              "self": {"calendarAddress": "mailto:self@example.test", "roles": {"attendee": true}},
              "other": {"calendarAddress": "mailto:other@example.test", "roles": {"attendee": true}}
          }
      });

      prepare_event_for_get(
         &mut event,
         &auth,
         None,
         None,
         true,
         chrono_tz::UTC,
         Some(&["utcStart".into(), "utcEnd".into()]),
      )
      .unwrap();

      assert_eq!(event["utcStart"], "2026-07-15T13:00:00Z");
      assert_eq!(event["utcEnd"], "2026-07-15T14:00:00Z");
      assert!(event["participants"].get("owner").is_some());
      assert!(event["participants"].get("self").is_some());
      assert!(event["participants"].get("other").is_none());
   }

   #[test]
   fn hidden_attendees_are_reduced_in_base_and_override_participants() {
      let auth = AccountInfo::from_bearer_token(
         "account",
         "self@example.test",
         "Self",
         "long-enough-test-token",
      );
      let mut event = serde_json::json!({
          "organizerCalendarAddress": "mailto:owner@example.test",
          "hideAttendees": true,
          "participants": {
              "owner": {"email": "owner@example.test", "roles": {"owner": true}},
              "self": {"email": "self@example.test", "roles": {"attendee": true}},
              "other": {"email": "other@example.test", "roles": {"attendee": true}}
          },
          "recurrenceOverrides": {
              "2026-07-15T09:00:00": {
                  "participants": {
                      "owner": {"email": "owner@example.test", "roles": {"owner": true}},
                      "self": {"email": "self@example.test", "roles": {"attendee": true}},
                      "other": {"email": "other@example.test", "roles": {"attendee": true}}
                  }
              }
          }
      });

      prepare_event_for_get(&mut event, &auth, None, None, false, chrono_tz::UTC, None).unwrap();

      assert!(event["participants"].get("other").is_none());
      assert!(
         event["recurrenceOverrides"]["2026-07-15T09:00:00"]["participants"]
            .get("other")
            .is_none()
      );
   }

   #[test]
   fn scheduling_requests_fail_per_object_without_mutation() {
      let creates = BTreeMap::from([("new".to_owned(), serde_json::json!({}))]);
      let updates = BTreeMap::from([("old".to_owned(), serde_json::json!({}))]);
      let destroys = ["gone".to_owned()];
      let error = set_error(
         "noSupportedScheduleMethods",
         "this DAV bridge does not send scheduling messages",
      );
      let response = rejected_set_response(
         "account",
         "7",
         creates.keys(),
         updates.keys().map(String::as_str),
         destroys.iter().map(String::as_str),
         [&error, &error, &error],
      );

      assert_eq!(response["oldState"], "7");
      assert_eq!(response["newState"], "7");
      assert_eq!(
         response["notCreated"]["new"]["type"],
         "noSupportedScheduleMethods"
      );
      assert_eq!(
         response["notUpdated"]["old"]["type"],
         "noSupportedScheduleMethods"
      );
      assert_eq!(
         response["notDestroyed"]["gone"]["type"],
         "noSupportedScheduleMethods"
      );
   }

   #[test]
   fn event_get_bounds_recurrence_overrides() {
      let auth = AccountInfo::from_bearer_token(
         "account",
         "self@example.test",
         "Self",
         "long-enough-test-token",
      );
      let mut event = serde_json::json!({
          "timeZone": "Etc/UTC",
          "recurrenceOverrides": {
              "2026-07-01T09:00:00": {},
              "2026-07-15T09:00:00": {},
              "2026-08-01T09:00:00": {}
          }
      });
      let after = "2026-07-10T00:00:00Z".parse().unwrap();
      let before = "2026-08-01T00:00:00Z".parse().unwrap();

      prepare_event_for_get(
         &mut event,
         &auth,
         Some(&before),
         Some(&after),
         false,
         chrono_tz::UTC,
         None,
      )
      .unwrap();

      let overrides = event["recurrenceOverrides"].as_object().unwrap();
      assert_eq!(overrides.len(), 1);
      assert!(overrides.contains_key("2026-07-15T09:00:00"));
   }

   #[test]
   fn calendar_event_requires_jscalendar_2_metadata() {
      let missing_type = event_raw(serde_json::json!({
          "version": "2.0",
          "uid": "uid",
          "start": "2026-07-15T09:00:00"
      }))
      .unwrap_err();
      assert_eq!(missing_type["properties"], serde_json::json!(["@type"]));

      let old_version = event_raw(serde_json::json!({
          "@type": "Event",
          "version": "1.0",
          "uid": "uid",
          "start": "2026-07-15T09:00:00"
      }))
      .unwrap_err();
      assert_eq!(old_version["properties"], serde_json::json!(["version"]));

      event_raw(serde_json::json!({
          "@type": "Event",
          "version": "2.0",
          "uid": "uid",
          "start": "2026-07-15T09:00:00"
      }))
      .unwrap();
   }
}
