//! Static Calendar capability methods.

use std::collections::BTreeMap;

use dav_sync::store::DavKind;
use jmap_protocol::{
   error::MethodError,
   filter::{
      Comparator,
      Filter,
      has_unsupported_fields,
   },
   ids::{
      AccountId,
      Id,
      State,
   },
   methods::{
      QueryChangesRequest,
      QueryChangesResponse,
      QueryResponse,
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
   dav_common::{
      require_dav,
      set_error,
   },
   enforce_set_limit,
   query_limit,
   query_position,
   server_fail,
   static_object,
   validate_static_since_state,
};
use crate::state::{
   AccountInfo,
   AppState,
};

const MAX_QUERY_RESULTS: u32 = MAX_OBJECTS_IN_GET as u32;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ParticipantIdentity {
   id:               Id,
   name:             String,
   calendar_address: String,
   is_default:       bool,
}

/// # Errors
///
/// Returns [`MethodError`] if the request arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, the requested id set exceeds
/// the per-`get` object limit, or the response cannot be serialized.
pub fn participant_get(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let identity = participant_identity(auth);
   static_object::singleton_get(
      args,
      "ParticipantIdentity",
      participant_state(auth),
      identity.id.clone(),
      identity,
      |account_id| require_dav(state, auth, account_id, DavKind::CalDav).map(|_| ()),
   )
}

/// # Errors
///
/// Returns [`MethodError`] if the request arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, `sinceState` differs from the
/// current state ([`MethodError::CannotCalculateChanges`]), or the response
/// cannot be serialized.
pub fn participant_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   static_object::empty_changes(
      args,
      "ParticipantIdentity",
      participant_state(auth),
      |account_id| require_dav(state, auth, account_id, DavKind::CalDav).map(|_| ()),
   )
}

/// # Errors
///
/// Returns [`MethodError`] if the request arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, the change set exceeds the
/// per-`set` object limit, or `ifInState` does not match the current state
/// ([`MethodError::StateMismatch`]). Individual changes are always rejected in
/// the response body, since participant identities are read-only.
pub fn participant_set(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let error = set_error(
      "forbidden",
      "participant identities are derived from account configuration",
   );
   static_object::rejected_set(
      args,
      "ParticipantIdentity",
      participant_state(auth),
      [error.clone(), error.clone(), error],
      |account_id| require_dav(state, auth, account_id, DavKind::CalDav).map(|_| ()),
   )
}

/// # Errors
///
/// Returns [`MethodError`] if the request arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, the requested id set exceeds
/// the per-`get` object limit, or the response cannot be serialized.
pub fn notification_get(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   static_object::empty_get(
      args,
      "CalendarEventNotification",
      notification_state(&auth.id),
      |account_id| require_dav(state, auth, account_id, DavKind::CalDav).map(|_| ()),
   )
}

/// # Errors
///
/// Returns [`MethodError`] if the request arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, `sinceState` differs from the
/// current state ([`MethodError::CannotCalculateChanges`]), or the response
/// cannot be serialized.
pub fn notification_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   static_object::empty_changes(
      args,
      "CalendarEventNotification",
      notification_state(&auth.id),
      |account_id| require_dav(state, auth, account_id, DavKind::CalDav).map(|_| ()),
   )
}

/// # Errors
///
/// Returns [`MethodError`] if the request arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, the change set exceeds the
/// per-`set` object limit, or `ifInState` does not match the current state
/// ([`MethodError::StateMismatch`]). Individual changes are always rejected in
/// the response body, since notifications are server-created.
pub fn notification_set(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let forbidden = set_error("forbidden", "notifications are server-created");
   static_object::rejected_set(
      args,
      "CalendarEventNotification",
      notification_state(&auth.id),
      [
         forbidden.clone(),
         forbidden,
         set_error("notFound", "calendar event notification not found"),
      ],
      |account_id| require_dav(state, auth, account_id, DavKind::CalDav).map(|_| ()),
   )
}

/// # Errors
///
/// Returns [`MethodError`] if the filter references unsupported fields
/// ([`MethodError::UnsupportedFilter`]), the arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, the sort is unsupported
/// ([`MethodError::UnsupportedSort`]), an `anchor` is supplied
/// ([`MethodError::AnchorNotFound`]), or the response cannot be serialized.
pub fn notification_query(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_notification_filter(args.get("filter"))?;
   let req = serde_json::from_value::<NotificationQueryArgs>(args).map_err(|error| {
      bad_args(format!(
         "invalid CalendarEventNotification/query args: {error}"
      ))
   })?;
   let account_id = req.account_id.as_ref();
   require_dav(state, auth, account_id, DavKind::CalDav)?;
   validate_notification_sort(req.sort.as_deref())?;
   let _ = (&req.filter, req.anchor_offset);
   if req.anchor.is_some() {
      return Err(MethodError::AnchorNotFound);
   }
   let (limit, response_limit) = query_limit(req.limit, MAX_QUERY_RESULTS);
   let _ = limit;
   serde_json::to_value(QueryResponse {
      account_id:            AccountId(account_id.to_owned()),
      query_state:           notification_state(account_id),
      can_calculate_changes: true,
      position:              i64::try_from(query_position(req.position, 0)).unwrap_or(i64::MAX),
      ids:                   vec![],
      total:                 req.calculate_total.then_some(0),
      limit:                 response_limit,
   })
   .map_err(|error| server_fail(error.to_string()))
}

/// # Errors
///
/// Returns [`MethodError`] if the filter references unsupported fields
/// ([`MethodError::UnsupportedFilter`]), the arguments fail to deserialize, the
/// authenticated account has no `CalDAV` access, the sort is unsupported
/// ([`MethodError::UnsupportedSort`]), an `upToId` is supplied or
/// `sinceQueryState` is stale ([`MethodError::CannotCalculateChanges`]), or the
/// response cannot be serialized.
pub fn notification_query_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_notification_filter(args.get("filter"))?;
   let req =
      serde_json::from_value::<QueryChangesRequest<NotificationFilter>>(args).map_err(|error| {
         bad_args(format!(
            "invalid CalendarEventNotification/queryChanges args: {error}"
         ))
      })?;
   let account_id = req.account_id.as_ref();
   require_dav(state, auth, account_id, DavKind::CalDav)?;
   validate_notification_sort(req.sort.as_deref())?;
   if req.up_to_id.is_some() {
      return Err(MethodError::CannotCalculateChanges);
   }
   let current = notification_state(account_id);
   validate_static_since_state(&req.since_query_state, &current)?;
   serde_json::to_value(QueryChangesResponse {
      account_id:      AccountId(account_id.to_owned()),
      old_query_state: req.since_query_state,
      new_query_state: current,
      total:           req.calculate_total.then_some(0),
      removed:         vec![],
      added:           vec![],
   })
   .map_err(|error| server_fail(error.to_string()))
}

/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, the change set
/// exceeds the per-`set` object limit, `fromAccountId` equals `accountId`,
/// either account id does not match the authenticated account
/// ([`MethodError::AccountNotFound`] / [`MethodError::FromAccountNotFound`]),
/// or `CalDAV` access is denied. Copying across accounts is unsupported, so a
/// successful authorization still yields
/// [`MethodError::FromAccountNotSupportedByMethod`].
pub fn calendar_event_copy(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   recognize_unavailable_copy(state, auth, args, DavKind::CalDav, "CalendarEvent/copy")
}

/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, the change set
/// exceeds the per-`set` object limit, `fromAccountId` equals `accountId`,
/// either account id does not match the authenticated account
/// ([`MethodError::AccountNotFound`] / [`MethodError::FromAccountNotFound`]),
/// or `CardDAV` access is denied. Copying across accounts is unsupported, so a
/// successful authorization still yields
/// [`MethodError::FromAccountNotSupportedByMethod`].
pub fn contact_card_copy(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   recognize_unavailable_copy(state, auth, args, DavKind::CardDav, "ContactCard/copy")
}

fn recognize_unavailable_copy(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
   kind: DavKind,
   method: &str,
) -> MethodResult {
   let req = serde_json::from_value::<CopyAccountArgs>(args)
      .map_err(|error| bad_args(format!("invalid {method} args: {error}")))?;
   enforce_set_limit(req.create.len(), 0, 0)?;
   if req.from_account_id == req.account_id {
      return Err(bad_args("fromAccountId and accountId must differ"));
   }
   if req.account_id.as_ref() != auth.id {
      return Err(MethodError::AccountNotFound);
   }
   if req.from_account_id.as_ref() != auth.id {
      return Err(MethodError::FromAccountNotFound);
   }
   require_dav(state, auth, req.account_id.as_ref(), kind)?;
   Err(MethodError::FromAccountNotSupportedByMethod)
}

fn participant_identity(auth: &AccountInfo) -> ParticipantIdentity {
   ParticipantIdentity {
      id:               participant_id(auth),
      name:             auth.display_name.clone(),
      calendar_address: format!("mailto:{}", auth.email),
      is_default:       true,
   }
}

fn participant_id(auth: &AccountInfo) -> Id {
   Id(static_object::state("participant-id", [auth.id.as_str()]).0)
}

fn participant_state(auth: &AccountInfo) -> State {
   static_object::state("participant", [
      auth.id.as_str(),
      auth.email.as_str(),
      auth.display_name.as_str(),
   ])
}

fn notification_state(account_id: &str) -> State {
   static_object::state("calendar-event-notification", [account_id])
}

#[derive(Debug, Deserialize)]
struct CopyAccountArgs {
   #[serde(rename = "fromAccountId")]
   from_account_id: AccountId,
   #[serde(rename = "accountId")]
   account_id:      AccountId,
   create:          BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationFilter {
   #[serde(default)]
   after:              Option<chrono::DateTime<chrono::Utc>>,
   #[serde(default)]
   before:             Option<chrono::DateTime<chrono::Utc>>,
   #[serde(rename = "type", default)]
   type_:              Option<String>,
   #[serde(default)]
   calendar_event_ids: Option<Vec<Id>>,
}

#[derive(Debug, Deserialize)]
struct NotificationQueryArgs {
   #[serde(rename = "accountId")]
   account_id:      AccountId,
   #[serde(default)]
   filter:          Option<Filter<NotificationFilter>>,
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

fn validate_notification_filter(filter: Option<&serde_json::Value>) -> Result<(), MethodError> {
   if filter.is_some_and(|filter| {
      has_unsupported_fields(filter, &["after", "before", "type", "calendarEventIds"])
   }) {
      Err(MethodError::UnsupportedFilter)
   } else {
      Ok(())
   }
}

fn validate_notification_sort(sort: Option<&[Comparator]>) -> Result<(), MethodError> {
   if sort.unwrap_or(&[]).iter().any(|comparator| {
      comparator.property != "created"
         || comparator.collation.is_some()
         || !comparator.extra.is_empty()
   }) {
      Err(MethodError::UnsupportedSort)
   } else {
      Ok(())
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn participant_identity_state_tracks_visible_config() {
      let before = AccountInfo::from_bearer_token(
         "account",
         "before@example.test",
         "Example",
         "long-enough-test-token",
      );
      let after = AccountInfo::from_bearer_token(
         "account",
         "after@example.test",
         "Example",
         "long-enough-test-token",
      );

      assert_ne!(participant_state(&before), participant_state(&after));
      assert!(participant_identity(&before).is_default);
   }

   #[test]
   fn notification_filter_and_sort_validation_is_strict() {
      validate_notification_filter(Some(&serde_json::json!({
          "calendarEventIds": ["one"]
      })))
      .unwrap();
      assert_eq!(
         validate_notification_filter(Some(&serde_json::json!({"madeUp": true}))),
         Err(MethodError::UnsupportedFilter)
      );
      assert_eq!(
         validate_notification_sort(Some(&[Comparator {
            property:     "type".into(),
            is_ascending: true,
            collation:    None,
            extra:        serde_json::Map::new(),
         }])),
         Err(MethodError::UnsupportedSort)
      );
   }
}
