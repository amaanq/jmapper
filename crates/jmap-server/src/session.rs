//! GET `/.well-known/jmap` session resource (RFC 8620 §2).

use std::collections::HashMap;

use axum::{
   Extension,
   Json,
   extract::State,
   response::IntoResponse,
};
use jmap_protocol::{
   ids::AccountId,
   session::{
      Account,
      CalendarsCapability,
      ContactsCapability,
      CoreCapability,
      MailCapability,
      Session,
      SubmissionCapability,
      URN_CALENDARS,
      URN_CONTACTS,
      URN_CORE,
      URN_MAIL,
      URN_QUOTA,
      URN_SUBMISSION,
      URN_VACATION_RESPONSE,
   },
};

use crate::{
   auth::AuthedAccount,
   state::{
      AccountInfo,
      AppState,
   },
};

pub(crate) const SUPPORTED_CAPABILITIES: &[&str] = &[
   URN_CORE,
   URN_MAIL,
   URN_SUBMISSION,
   URN_QUOTA,
   URN_VACATION_RESPONSE,
   URN_CALENDARS,
   URN_CONTACTS,
];

/// GET /.well-known/jmap — returns the session resource for the authenticated
/// account.
///
/// # Panics
///
/// Panics if a built-in capability object fails to serialize to JSON, which
/// cannot happen for the fixed capability types constructed here.
pub async fn session_handler(
   State(state): State<AppState>,
   Extension(AuthedAccount(current)): Extension<AuthedAccount>,
) -> impl IntoResponse {
   let base = state.session_url().trim_end_matches('/').to_owned();
   let dav = state.dav_availability(&current.id);

   let id = AccountId(current.id.clone());
   let mut account_capabilities = HashMap::from([
      (
         URN_MAIL.to_owned(),
         serde_json::to_value(MailCapability::default()).unwrap(),
      ),
      (
         URN_SUBMISSION.to_owned(),
         serde_json::to_value(SubmissionCapability::default()).unwrap(),
      ),
      (URN_QUOTA.to_owned(), serde_json::json!({})),
      (URN_VACATION_RESPONSE.to_owned(), serde_json::json!({})),
   ]);
   let mut capabilities = account_capabilities.clone();
   capabilities.insert(
      URN_CORE.to_owned(),
      serde_json::to_value(CoreCapability::default()).unwrap(),
   );
   if dav.calendars {
      account_capabilities.insert(
         URN_CALENDARS.to_owned(),
         serde_json::to_value(CalendarsCapability::dav_proxy()).unwrap(),
      );
      capabilities.insert(URN_CALENDARS.to_owned(), serde_json::json!({}));
   }
   if dav.contacts {
      account_capabilities.insert(
         URN_CONTACTS.to_owned(),
         serde_json::to_value(ContactsCapability::dav_proxy()).unwrap(),
      );
      capabilities.insert(URN_CONTACTS.to_owned(), serde_json::json!({}));
   }
   let accounts = HashMap::from([(id.clone(), Account {
      name: current.display_name.clone(),
      is_personal: true,
      is_read_only: false,
      account_capabilities,
   })]);
   let mut primary_accounts = HashMap::new();
   for urn in [URN_MAIL, URN_SUBMISSION, URN_QUOTA, URN_VACATION_RESPONSE] {
      primary_accounts.insert(urn.to_owned(), id.clone());
   }
   if dav.calendars {
      primary_accounts.insert(URN_CALENDARS.to_owned(), id.clone());
   }
   if dav.contacts {
      primary_accounts.insert(URN_CONTACTS.to_owned(), id);
   }

   let state_hash = session_state(&state, &current);

   Json(Session {
      capabilities,
      accounts,
      primary_accounts,
      username: current.email,
      api_url: format!("{base}/api"),
      download_url: format!("{base}/download/{{accountId}}/{{blobId}}/{{name}}?type={{type}}"),
      upload_url: format!("{base}/upload/{{accountId}}"),
      event_source_url: format!(
         "{base}/eventsource?types={{types}}&closeafter={{closeafter}}&ping={{ping}}"
      ),
      state: state_hash,
   })
}

/// Hash every config-backed part of the authenticated user's Session object.
/// Unrelated accounts stay out of the hash, while reloads that change URLs,
/// identity metadata, limits, or advertised capabilities invalidate clients.
pub(crate) fn session_state(state: &AppState, account: &AccountInfo) -> String {
   use sha1::{
      Digest as _,
      Sha1,
   };

   let mut hash = Sha1::new();
   for value in [
      state.session_url(),
      account.id.as_str(),
      account.email.as_str(),
      account.display_name.as_str(),
   ] {
      hash.update((value.len() as u64).to_be_bytes());
      hash.update(value.as_bytes());
   }
   let dav = state.dav_availability(&account.id);
   hash.update([u8::from(dav.calendars), u8::from(dav.contacts)]);
   hash.update(
      serde_json::to_vec(&(
         CoreCapability::default(),
         MailCapability::default(),
         SubmissionCapability::default(),
         SUPPORTED_CAPABILITIES,
      ))
      .expect("Session capability values are serializable"),
   );
   hex::encode(&hash.finalize()[..6])
}
