//! POST `/api` — JMAP method dispatch (RFC 8620 §3.5).
//!
//! Handlers authorize `accountId` against the bearer account. Result references
//! resolve inline so each call can consume earlier responses in the batch.

use std::{
   collections::HashMap,
   sync::atomic::Ordering,
};

use axum::{
   Extension,
   Json,
   body::{
      Body,
      Bytes,
      to_bytes,
   },
   extract::{
      Request as AxumRequest,
      State,
   },
   http::header,
   response::IntoResponse,
};
use jmap_protocol::{
   error::MethodError,
   ids::MethodCallId,
   method::{
      Invocation,
      Request as JmapRequest,
      Response,
   },
   session::CoreCapability,
};

use crate::{
   auth::AuthedAccount,
   error::ApiError,
   methods,
   observability::METRICS,
   resolve::resolve_args,
   session,
   state::{
      AccountInfo,
      AppState,
   },
};

/// # Errors
///
/// Returns [`ApiError::JmapRequest`] when the request is not usable as a whole:
/// the `Content-Type` is not `application/json`, the body exceeds
/// `maxSizeRequest`, the body is not valid JSON or a valid JMAP Request, the
/// batch is empty or exceeds `maxCallsInRequest`, or it lists an unsupported
/// capability. Per-call failures are reported inline as `error` invocations and
/// do not fail the handler.
pub async fn api_handler(
   State(state): State<AppState>,
   Extension(AuthedAccount(auth)): Extension<AuthedAccount>,
   request: AxumRequest,
) -> Result<impl IntoResponse, ApiError> {
   let content_type = request
      .headers()
      .get(header::CONTENT_TYPE)
      .and_then(|value| value.to_str().ok())
      .and_then(|value| value.split(';').next())
      .map(str::trim)
      .is_some_and(|value| value.eq_ignore_ascii_case("application/json"));
   if !content_type {
      return Err(jmap_request_error(
         "notRequest",
         "the JMAP endpoint requires Content-Type: application/json",
      ));
   }

   let limits = CoreCapability::default();
   let body = read_request_body(request.into_body(), limits.max_size_request as usize).await?;
   let req = decode_request(&body)?;
   validate_request(&req, limits.max_calls_in_request as usize)?;

   METRICS
      .jmap_method_calls_total
      .fetch_add(req.method_calls.len() as u64, Ordering::Relaxed);
   let mut responses = Vec::<Invocation>::with_capacity(req.method_calls.len());
   let mut created_ids = req.created_ids.unwrap_or_default();
   for call in req.method_calls {
      let call_id = call.call_id.clone();
      let resolved_args = match resolve_args(&call.arguments, &responses) {
         Ok(args) => args,
         Err(err) => {
            responses.push(method_error(err, call_id));
            continue;
         },
      };
      let resolved_call = Invocation::new(call.name, resolved_args, call_id);
      // A single call can yield more than one invocation: EmailSubmission/set
      // with onSuccess* appends its implicit Email/set response (RFC 8621
      // §7.5) under the same call id.
      let invs = dispatch(&state, &auth, resolved_call, &mut created_ids).await;
      responses.extend(invs);
   }

   METRICS.jmap_method_errors_total.fetch_add(
      responses
         .iter()
         .filter(|response| response.name == "error")
         .count() as u64,
      Ordering::Relaxed,
   );
   let session_state = session::session_state(&state, &auth);
   Ok(Json(Response {
      method_responses: responses,
      // Only echo createdIds when the client opted in or the batch produced
      // new server-minted ids. Empty response + None avoids an unnecessary
      // field on the wire.
      created_ids: if created_ids.is_empty() {
         None
      } else {
         Some(created_ids)
      },
      session_state,
   }))
}

async fn read_request_body(body: Body, max_size: usize) -> Result<Bytes, ApiError> {
   to_bytes(body, max_size).await.map_err(|_| {
      jmap_request_error(
         "requestTooLarge",
         format!("request body exceeds maxSizeRequest = {max_size} bytes"),
      )
   })
}

fn decode_request(body: &[u8]) -> Result<JmapRequest, ApiError> {
   let value = serde_json::from_slice::<serde_json::Value>(body)
      .map_err(|_| jmap_request_error("notJSON", "the request body is not valid JSON"))?;
   serde_json::from_value::<JmapRequest>(value).map_err(|error| {
      jmap_request_error(
         "notRequest",
         format!("the JSON value is not a valid JMAP Request: {error}"),
      )
   })
}

fn validate_request(req: &JmapRequest, max_calls: usize) -> Result<(), ApiError> {
   if req.method_calls.is_empty() {
      return Err(jmap_request_error(
         "notRequest",
         "methodCalls must contain at least one invocation",
      ));
   }
   if req.method_calls.len() > max_calls {
      return Err(jmap_request_error(
         "requestTooLarge",
         format!("request contains more than maxCallsInRequest = {max_calls} method calls"),
      ));
   }
   if let Some(capability) = req
      .using
      .iter()
      .find(|capability| !session::SUPPORTED_CAPABILITIES.contains(&capability.as_str()))
   {
      return Err(jmap_request_error(
         "unknownCapability",
         format!("capability {capability:?} is not supported"),
      ));
   }
   Ok(())
}

fn jmap_request_error(kind: &'static str, detail: impl Into<String>) -> ApiError {
   ApiError::JmapRequest {
      kind,
      detail: detail.into(),
   }
}

async fn dispatch(
   state: &AppState,
   auth: &AccountInfo,
   call: Invocation,
   created_ids: &mut HashMap<String, String>,
) -> Vec<Invocation> {
   let call_id = call.call_id.clone();

   if call.name == "EmailSubmission/set" {
      return match methods::email_submission::set_with_implicit(
         state,
         auth,
         call.arguments,
         created_ids,
      )
      .await
      {
         Ok((sub_resp, implicit)) => {
            register_created_ids(&sub_resp, created_ids);
            let mut out = vec![Invocation::new(
               "EmailSubmission/set".to_owned(),
               sub_resp,
               call_id.clone(),
            )];
            if let Some(email_set) = implicit {
               out.push(Invocation::new("Email/set".to_owned(), email_set, call_id));
            }
            out
         },
         Err(err) => vec![method_error(err, call_id)],
      };
   }

   let response_name = call.name.as_str();
   macro_rules! method {
      ($handler:path) => {
         ($handler)(state, auth, call.arguments)
            .await
            .map(|value| (response_name, value))
      };
   }
   macro_rules! local_method {
      ($handler:path) => {
         ($handler)(call.arguments).map(|value| (response_name, value))
      };
   }
   macro_rules! static_method {
      ($handler:path) => {
         ($handler)(state, auth, call.arguments).map(|value| (response_name, value))
      };
   }

   let result = match response_name {
      // Core/echo is the only method with no accountId contract; let it
      // through unchanged as a dumb round-trip.
      "Core/echo" => Ok((response_name, call.arguments)),
      "PushSubscription/get" => local_method!(methods::push_subscription::get),
      "PushSubscription/set" => local_method!(methods::push_subscription::set),
      "Mailbox/get" => method!(methods::mailbox::get),
      "Mailbox/query" => method!(methods::mailbox::query),
      "Mailbox/queryChanges" => static_method!(methods::mailbox::query_changes),
      "Mailbox/changes" => method!(methods::mailbox::changes),
      "Mailbox/set" => method!(methods::mailbox_set::set),
      "Email/get" => method!(methods::email::get),
      "Email/query" => method!(methods::email::query),
      "Email/queryChanges" => method!(methods::email::query_changes),
      "Email/changes" => method!(methods::email::changes),
      "Email/set" => method!(methods::email_set::set),
      "Email/import" => method!(methods::email_import::import),
      "Email/parse" => method!(methods::email_parse::parse),
      "Email/copy" | "Blob/copy" => {
         methods::copy::unavailable(auth, call.arguments, response_name)
            .map(|value| (response_name, value))
      },
      "Identity/get" => static_method!(methods::identity::get),
      "Identity/changes" => static_method!(methods::identity::changes),
      "Identity/set" => static_method!(methods::identity::set),
      "Quota/get" => static_method!(methods::quota::get),
      "Quota/changes" => static_method!(methods::quota::changes),
      "Quota/query" => static_method!(methods::quota::query),
      "Quota/queryChanges" => static_method!(methods::quota::query_changes),
      "VacationResponse/get" => static_method!(methods::vacation::get),
      "VacationResponse/changes" => static_method!(methods::vacation::changes),
      "VacationResponse/set" => static_method!(methods::vacation::set),
      "AddressBook/get" => method!(methods::contacts::addressbook_get),
      "AddressBook/changes" => method!(methods::contacts::addressbook_changes),
      "AddressBook/set" => method!(methods::contacts::addressbook_set),
      "ContactCard/get" => method!(methods::contacts::contact_get),
      "ContactCard/changes" => method!(methods::contacts::contact_changes),
      "ContactCard/query" => method!(methods::contacts::contact_query),
      "ContactCard/queryChanges" => method!(methods::contacts::contact_query_changes),
      "ContactCard/set" => method!(methods::contacts::contact_set),
      "ContactCard/copy" => static_method!(methods::calendar_aux::contact_card_copy),
      "ParticipantIdentity/get" => static_method!(methods::calendar_aux::participant_get),
      "ParticipantIdentity/changes" => {
         static_method!(methods::calendar_aux::participant_changes)
      },
      "ParticipantIdentity/set" => static_method!(methods::calendar_aux::participant_set),
      "Calendar/get" => method!(methods::calendar::calendar_get),
      "Calendar/changes" => method!(methods::calendar::calendar_changes),
      "Calendar/set" => method!(methods::calendar::calendar_set),
      "CalendarEvent/get" => method!(methods::calendar::event_get),
      "CalendarEvent/changes" => method!(methods::calendar::event_changes),
      "CalendarEvent/query" => method!(methods::calendar::event_query),
      "CalendarEvent/queryChanges" => method!(methods::calendar::event_query_changes),
      "CalendarEvent/set" => method!(methods::calendar::event_set),
      "CalendarEvent/copy" => static_method!(methods::calendar_aux::calendar_event_copy),
      "CalendarEventNotification/get" => {
         static_method!(methods::calendar_aux::notification_get)
      },
      "CalendarEventNotification/changes" => {
         static_method!(methods::calendar_aux::notification_changes)
      },
      "CalendarEventNotification/set" => {
         static_method!(methods::calendar_aux::notification_set)
      },
      "CalendarEventNotification/query" => {
         static_method!(methods::calendar_aux::notification_query)
      },
      "CalendarEventNotification/queryChanges" => {
         static_method!(methods::calendar_aux::notification_query_changes)
      },
      "Thread/get" => method!(methods::thread::get),
      "Thread/changes" => method!(methods::thread::changes),
      "SearchSnippet/get" => method!(methods::search_snippet::get),
      "EmailSubmission/get" => method!(methods::email_submission::get),
      "EmailSubmission/changes" => method!(methods::email_submission::changes),
      "EmailSubmission/query" => method!(methods::email_submission::query),
      "EmailSubmission/queryChanges" => {
         static_method!(methods::email_submission::query_changes)
      },
      _ => Err(MethodError::UnknownMethod),
   };
   match result {
      Ok((name, args)) => {
         register_created_ids(&args, created_ids);
         vec![Invocation::new(name.to_owned(), args, call_id)]
      },
      Err(err) => vec![method_error(err, call_id)],
   }
}

/// RFC 8620 §3.5: server-minted ids from Set-style creates are published in
/// the response's createdIds map.
fn register_created_ids(args: &serde_json::Value, created_ids: &mut HashMap<String, String>) {
   if let Some(created) = args.get("created").and_then(|value| value.as_object()) {
      for (creation_id, obj) in created {
         if let Some(id) = obj.get("id").and_then(|value| value.as_str()) {
            created_ids.insert(creation_id.clone(), id.to_owned());
         }
      }
   }
}

fn method_error(err: MethodError, call_id: MethodCallId) -> Invocation {
   Invocation::new("error", serde_json::to_value(err).unwrap(), call_id)
}

#[cfg(test)]
mod tests {
   use jmap_protocol::session::URN_CORE;

   use super::*;

   #[expect(
      clippy::panic,
      reason = "test helper asserts the error is the expected variant"
   )]
   fn request_error_kind(error: ApiError) -> &'static str {
      match error {
         ApiError::JmapRequest { kind, .. } => kind,
         other => panic!("expected JMAP request error, got {other:?}"),
      }
   }

   #[test]
   fn method_error_shape() {
      let err = method_error(MethodError::UnknownMethod, MethodCallId::from("c1"));
      let value = serde_json::to_value(&err).unwrap();
      assert_eq!(value[0], "error");
      assert_eq!(value[1]["type"], "unknownMethod");
      assert_eq!(value[2], "c1");
   }

   #[test]
   fn request_decode_distinguishes_json_from_request_shape() {
      assert_eq!(
         request_error_kind(decode_request(b"{").unwrap_err()),
         "notJSON"
      );
      assert_eq!(
         request_error_kind(decode_request(b"[]").unwrap_err()),
         "notRequest"
      );
   }

   #[test]
   fn request_validation_enforces_calls_and_capabilities() {
      let invocation =
         Invocation::new("Core/echo", serde_json::json!({}), MethodCallId::from("c0"));
      let empty = JmapRequest {
         using:        vec![],
         method_calls: vec![],
         created_ids:  None,
      };
      assert_eq!(
         request_error_kind(validate_request(&empty, 1).unwrap_err()),
         "notRequest"
      );

      let too_many = JmapRequest {
         using:        vec![URN_CORE.into()],
         method_calls: vec![invocation.clone(), invocation.clone()],
         created_ids:  None,
      };
      assert_eq!(
         request_error_kind(validate_request(&too_many, 1).unwrap_err()),
         "requestTooLarge"
      );

      let unknown = JmapRequest {
         using:        vec!["urn:example:unknown".into()],
         method_calls: vec![invocation],
         created_ids:  None,
      };
      assert_eq!(
         request_error_kind(validate_request(&unknown, 1).unwrap_err()),
         "unknownCapability"
      );
   }

   #[tokio::test]
   async fn request_body_limit_is_enforced_before_json_parsing() {
      let error = read_request_body(Body::from("1234"), 3).await.unwrap_err();
      assert_eq!(request_error_kind(error), "requestTooLarge");
   }
}
