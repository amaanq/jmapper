//! Integration tests for the JMAP HTTP router.
//!
//! These exercise the axum layers end-to-end (bearer auth, account isolation,
//! CORS, operational endpoints) against per-test PostgreSQL databases. They
//! run the concrete handlers, not mocks — any routing or middleware regression
//! here should fail these tests.

use std::collections::{
   BTreeSet,
   HashMap,
};

use axum::{
   body::Body,
   http::{
      Request,
      StatusCode,
   },
   response::Response,
};
use base64::{
   Engine as _,
   engine::general_purpose::STANDARD,
};
use dav_sync::{
   engine,
   service,
};
use deadpool_postgres::Pool as PgPool;
use http_body_util::BodyExt as _;
use imap_sync::{
   account::AccountRequest,
   db,
   provider::ProviderKind,
   sync,
   testkit,
};
use jmap_server::{
   AccountInfo,
   AppState,
   build_router,
   state::DavAvailability,
};
use tokio::sync::mpsc;
use tokio_postgres::types::ToSql;
use tower::ServiceExt as _;

/// Per-test throwaway postgres database; `None` means "skip this test"
/// (`JMAPPER_TEST_DB_URL` unset outside the dev shell).
async fn setup_pool() -> Option<PgPool> {
   testkit::test_pool().await
}

/// Run a query against the pool — assertion/seed helper for integration tests.
async fn exec(pool: &PgPool, sql: &str, params: &[&(dyn ToSql + Sync)]) {
   pool
      .get()
      .await
      .unwrap()
      .execute(sql, params)
      .await
      .unwrap();
}

/// Build an `AppState` with two accounts, each with a distinct bearer token.
async fn two_accounts() -> Option<(AppState, &'static str, &'static str)> {
   let pool = setup_pool().await?;
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"hashA")
      .await
      .unwrap();
   db::upsert_account(&pool, "acctB", "b@x.com", ProviderKind::Imap, "B", b"hashB")
      .await
      .unwrap();

   let token_a = "bearer-A-abcdef0123456789";
   let token_b = "bearer-B-fedcba9876543210";
   let accounts = vec![
      AccountInfo::from_bearer_token("acctA", "a@x.com", "A", token_a),
      AccountInfo::from_bearer_token("acctB", "b@x.com", "B", token_b),
   ];
   let state = AppState::new(
      pool,
      accounts,
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   Some((state, token_a, token_b))
}

async fn dav_account() -> Option<(AppState, &'static str, service::DavHandle)> {
   use dav_sync::store::{
      self,
      DavKind,
   };

   let pool = setup_pool().await?;
   db::upsert_account(
      &pool,
      "acctDav",
      "dav@example.test",
      ProviderKind::Imap,
      "DAV User",
      b"hash",
   )
   .await
   .unwrap();
   let client = pool.get().await.unwrap();
   for kind in [DavKind::CalDav, DavKind::CardDav] {
      store::upsert_endpoint(
         &client,
         "acctDav",
         kind,
         "http://127.0.0.1:9/",
         "none",
         None,
         None,
      )
      .await
      .unwrap();
      store::set_sync_ok(&client, "acctDav", kind, 1)
         .await
         .unwrap();
   }

   let calendar_id = store::id_for_href(DavKind::CalDav, "/cal/default/");
   let calendar_modseq = store::bump_collection_modseq(&client, "acctDav", DavKind::CalDav)
      .await
      .unwrap();
   store::upsert_collection(
      &client,
      "acctDav",
      &calendar_id,
      DavKind::CalDav,
      "/cal/default/",
      "Personal",
      Some("#336699"),
      Some("Calendar"),
      Some("cal-token"),
      true,
      calendar_modseq,
   )
   .await
   .unwrap();
   let event_raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:event-1\r\nDTSTAMP:\
                    20260701T000000Z\r\nDTSTART:20260715T090000Z\r\nDURATION:PT1H\r\nSUMMARY:\
                    Planning\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
   let (event_uid, event_json) = engine::convert_resource(DavKind::CalDav, event_raw).unwrap();
   let event_id = store::id_for_uid(DavKind::CalDav, &event_uid);
   let event_modseq = store::bump_resource_modseq(&client, "acctDav", DavKind::CalDav)
      .await
      .unwrap();
   store::upsert_resource(
      &client,
      "acctDav",
      &event_id,
      &calendar_id,
      DavKind::CalDav,
      "/cal/default/event-1.ics",
      Some("\"1\""),
      &event_uid,
      event_raw,
      &event_json,
      event_modseq,
   )
   .await
   .unwrap();

   let addressbook_id = store::id_for_href(DavKind::CardDav, "/card/default/");
   let addressbook_modseq = store::bump_collection_modseq(&client, "acctDav", DavKind::CardDav)
      .await
      .unwrap();
   store::upsert_collection(
      &client,
      "acctDav",
      &addressbook_id,
      DavKind::CardDav,
      "/card/default/",
      "Contacts",
      None,
      Some("People"),
      Some("card-token"),
      true,
      addressbook_modseq,
   )
   .await
   .unwrap();
   let card_raw = "BEGIN:VCARD\r\nVERSION:4.0\r\nUID:card-1\r\nFN:Ada \
                   Lovelace\r\nEMAIL:ada@example.test\r\nEND:VCARD\r\n";
   let (card_uid, card_json) = engine::convert_resource(DavKind::CardDav, card_raw).unwrap();
   let card_id = store::id_for_uid(DavKind::CardDav, &card_uid);
   let card_modseq = store::bump_resource_modseq(&client, "acctDav", DavKind::CardDav)
      .await
      .unwrap();
   store::upsert_resource(
      &client,
      "acctDav",
      &card_id,
      &addressbook_id,
      DavKind::CardDav,
      "/card/default/card-1.vcf",
      Some("\"1\""),
      &card_uid,
      card_raw,
      &card_json,
      card_modseq,
   )
   .await
   .unwrap();
   drop(client);

   let token = "bearer-DAV-abcdef0123456789";
   let handle = service::spawn(pool.clone(), "acctDav".to_owned());
   let state = AppState::new_with_dav(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctDav",
         "dav@example.test",
         "DAV User",
         token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
      HashMap::from([("acctDav".to_owned(), handle.clone())]),
      HashMap::from([("acctDav".to_owned(), DavAvailability {
         calendars: true,
         contacts:  true,
      })]),
   );
   Some((state, token, handle))
}

async fn read_body(resp: Response) -> (StatusCode, serde_json::Value) {
   let status = resp.status();
   let bytes = resp.into_body().collect().await.unwrap().to_bytes();
   let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
   (status, value)
}

#[tokio::test]
async fn healthz_bypasses_auth() {
   let Some((state, _a, _b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);
   let resp = router
      .oneshot(
         Request::builder()
            .uri("/healthz")
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(resp.status(), StatusCode::OK);
   let bytes = resp.into_body().collect().await.unwrap().to_bytes();
   assert_eq!(bytes.as_ref(), b"ok\n");
}

#[tokio::test]
async fn api_without_credentials_is_401() {
   let Some((state, _a, _b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("content-type", "application/json")
            .body(Body::from("{}"))
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn basic_auth_requires_matching_identity_and_token() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   for username in ["acctA", "A@X.COM"] {
      let credentials = STANDARD.encode(format!("{username}:{token_a}"));
      let response = router
         .clone()
         .oneshot(
            Request::builder()
               .uri("/.well-known/jmap")
               .header("authorization", format!("Basic {credentials}"))
               .body(Body::empty())
               .unwrap(),
         )
         .await
         .unwrap();
      assert_eq!(response.status(), StatusCode::OK);
   }

   for credentials in [
      STANDARD.encode(format!("b@x.com:{token_a}")),
      STANDARD.encode("a@x.com:wrong-token"),
   ] {
      let response = router
         .clone()
         .oneshot(
            Request::builder()
               .uri("/.well-known/jmap")
               .header("authorization", format!("Basic {credentials}"))
               .body(Body::empty())
               .unwrap(),
         )
         .await
         .unwrap();
      assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
   }
}

#[tokio::test]
async fn api_request_errors_use_jmap_problem_types() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let response = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from("{"))
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(response.status(), StatusCode::BAD_REQUEST);
   assert_eq!(
      response
         .headers()
         .get("content-type")
         .and_then(|value| value.to_str().ok()),
      Some("application/problem+json")
   );
   let (_, value) = read_body(response).await;
   assert_eq!(
      value["type"], "urn:ietf:params:jmap:error:notJSON",
      "{value}"
   );

   let body = serde_json::json!({
       "using": ["urn:example:unsupported"],
       "methodCalls": [["Core/echo", {}, "c0"]]
   });
   let response = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_, value) = read_body(response).await;
   assert_eq!(
      value["type"], "urn:ietf:params:jmap:error:unknownCapability",
      "{value}"
   );

   let calls = (0..33)
      .map(|index| serde_json::json!(["Core/echo", {}, format!("c{index}")]))
      .collect::<Vec<_>>();
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core"],
       "methodCalls": calls
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_, value) = read_body(response).await;
   assert_eq!(
      value["type"], "urn:ietf:params:jmap:error:requestTooLarge",
      "{value}"
   );
}

#[tokio::test]
async fn session_doc_only_lists_authed_account() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let resp = router
      .clone()
      .oneshot(
         Request::builder()
            .uri("/.well-known/jmap")
            .header("authorization", format!("Bearer {token_a}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, body) = read_body(resp).await;
   assert_eq!(status, StatusCode::OK);

   let accounts = body
      .get("accounts")
      .and_then(|value| value.as_object())
      .unwrap();
   assert_eq!(accounts.len(), 1, "session doc leaks other accounts");
   assert!(accounts.contains_key("acctA"), "expected only acctA in doc");
   assert!(!accounts.contains_key("acctB"));
   assert_eq!(body["username"].as_str(), Some("a@x.com"));
   assert_eq!(
      body["downloadUrl"].as_str(),
      Some("http://test.invalid/download/{accountId}/{blobId}/{name}?type={type}")
   );
   assert!(
      body["capabilities"]
         .as_object()
         .is_some_and(|capabilities| capabilities.contains_key("urn:ietf:params:jmap:quota"))
   );
   assert_eq!(
      body["primaryAccounts"]["urn:ietf:params:jmap:quota"],
      "acctA"
   );

   let session_state = body["state"].as_str().unwrap().to_owned();
   let request = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core"],
       "methodCalls": [["Core/echo", {}, "c0"]]
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(request.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_, response) = read_body(response).await;
   assert_eq!(response["sessionState"], session_state);
}

#[tokio::test]
async fn dav_capabilities_and_cached_methods_are_reachable_end_to_end() {
   use dav_sync::store::{
      self,
      DavKind,
   };

   let Some((state, token, handle)) = dav_account().await else {
      return;
   };
   let event_id = store::id_for_uid(DavKind::CalDav, "event-1");
   let router = build_router(state, vec!["http://example.test".into()]);
   let session = router
      .clone()
      .oneshot(
         Request::builder()
            .uri("/.well-known/jmap")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   let (_, session) = read_body(session).await;
   for capability in [
      "urn:ietf:params:jmap:calendars",
      "urn:ietf:params:jmap:contacts",
   ] {
      assert!(
         session["capabilities"].get(capability).is_some(),
         "{session}"
      );
      assert_eq!(session["primaryAccounts"][capability], "acctDav");
   }
   assert_eq!(
      session["accounts"]["acctDav"]["accountCapabilities"]["urn:ietf:params:jmap:contacts"]
         ["maxAddressBooksPerCard"],
      1
   );

   let request = serde_json::json!({
       "using": [
           "urn:ietf:params:jmap:core",
           "urn:ietf:params:jmap:calendars",
           "urn:ietf:params:jmap:contacts"
       ],
       "methodCalls": [
           ["AddressBook/get", {"accountId": "acctDav"}, "a"],
           ["ContactCard/query", {
               "accountId": "acctDav",
               "filter": {"name": "Lovelace Ada"},
               "calculateTotal": true
           }, "b"],
           ["CalendarEvent/get", {
               "accountId": "acctDav",
               "properties": ["id", "title", "calendarIds", "isOrigin"]
           }, "c"],
           ["CalendarEvent/changes", {
               "accountId": "acctDav",
               "sinceState": "0"
           }, "d"],
           ["ParticipantIdentity/get", {
               "accountId": "acctDav"
           }, "e"],
           ["CalendarEventNotification/query", {
               "accountId": "acctDav",
               "calculateTotal": true
           }, "f"],
           ["CalendarEvent/query", {
               "accountId": "acctDav",
               "filter": {
                   "after": "2026-07-15T08:30:00",
                   "before": "2026-07-15T09:30:00"
               },
               "timeZone": "Etc/UTC",
               "position": -1,
               "calculateTotal": true,
               "limit": 10
           }, "g"],
           ["ContactCard/copy", {
               "fromAccountId": "acctDav",
               "accountId": "acctDav",
               "create": {}
           }, "h"],
           ["CalendarEvent/set", {
               "accountId": "acctDav",
               "sendSchedulingMessages": true,
               "update": {event_id.clone(): {"title": "Must not be stored"}}
           }, "i"]
       ]
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(request.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, response) = read_body(response).await;
   assert_eq!(status, StatusCode::OK);
   let methods = response["methodResponses"].as_array().unwrap();
   assert_eq!(methods[0][0], "AddressBook/get");
   assert_eq!(methods[0][1]["list"][0]["name"], "Contacts");
   assert_eq!(methods[1][0], "ContactCard/query");
   assert_eq!(methods[1][1]["total"], 1);
   assert_eq!(methods[1][1]["ids"].as_array().unwrap().len(), 1);
   assert_eq!(methods[2][0], "CalendarEvent/get");
   assert_eq!(methods[2][1]["list"][0]["title"], "Planning");
   assert_eq!(methods[2][1]["list"][0]["isOrigin"], true);
   assert!(methods[2][1]["list"][0].get("uid").is_none());
   assert_eq!(methods[3][0], "CalendarEvent/changes");
   assert_eq!(methods[3][1]["created"].as_array().unwrap().len(), 1);
   assert_eq!(methods[4][0], "ParticipantIdentity/get");
   assert_eq!(
      methods[4][1]["list"][0]["calendarAddress"],
      "mailto:dav@example.test"
   );
   assert_eq!(methods[4][1]["list"][0]["isDefault"], true);
   assert_eq!(methods[5][0], "CalendarEventNotification/query");
   assert_eq!(methods[5][1]["total"], 0);
   assert_eq!(methods[6][0], "CalendarEvent/query");
   assert_eq!(methods[6][1]["total"], 1);
   assert_eq!(methods[6][1]["position"], 0);
   assert_eq!(methods[6][1]["ids"].as_array().unwrap().len(), 1);
   assert_eq!(methods[7][0], "error");
   assert_eq!(methods[7][1]["type"], "invalidArguments");
   assert_eq!(methods[8][0], "CalendarEvent/set");
   assert_eq!(
      methods[8][1]["notUpdated"][&event_id]["type"],
      "noSupportedScheduleMethods"
   );
   assert_eq!(methods[8][1]["oldState"], methods[8][1]["newState"]);
   handle.shutdown().await;
}

#[tokio::test]
async fn dav_updates_avoid_partial_move_and_edit_side_effects() {
   use dav_sync::store::{
      self,
      DavKind,
   };

   let Some((state, token, handle)) = dav_account().await else {
      return;
   };
   let pool = state.pool().clone();
   let client = pool.get().await.unwrap();
   let calendar_id = store::id_for_href(DavKind::CalDav, "/cal/target/");
   let calendar_modseq = store::bump_collection_modseq(&client, "acctDav", DavKind::CalDav)
      .await
      .unwrap();
   store::upsert_collection(
      &client,
      "acctDav",
      &calendar_id,
      DavKind::CalDav,
      "/cal/target/",
      "Target",
      None,
      None,
      None,
      false,
      calendar_modseq,
   )
   .await
   .unwrap();
   let addressbook_id = store::id_for_href(DavKind::CardDav, "/card/target/");
   let addressbook_modseq = store::bump_collection_modseq(&client, "acctDav", DavKind::CardDav)
      .await
      .unwrap();
   store::upsert_collection(
      &client,
      "acctDav",
      &addressbook_id,
      DavKind::CardDav,
      "/card/target/",
      "Target",
      None,
      None,
      None,
      false,
      addressbook_modseq,
   )
   .await
   .unwrap();
   drop(client);

   let event_id = store::id_for_uid(DavKind::CalDav, "event-1");
   let card_id = store::id_for_uid(DavKind::CardDav, "card-1");
   let mut event_noop = serde_json::Map::new();
   event_noop.insert(event_id.clone(), serde_json::json!({}));
   let mut event_combined = serde_json::Map::new();
   event_combined.insert(
      event_id.clone(),
      serde_json::json!({"calendarIds": {calendar_id.clone(): true}, "title": "Changed"}),
   );
   let mut card_noop = serde_json::Map::new();
   card_noop.insert(card_id.clone(), serde_json::json!({}));
   let mut card_combined = serde_json::Map::new();
   card_combined.insert(
      card_id.clone(),
      serde_json::json!({
          "addressBookIds": {addressbook_id.clone(): true},
          "name/full": "Changed"
      }),
   );
   let request = serde_json::json!({
       "using": [
           "urn:ietf:params:jmap:core",
           "urn:ietf:params:jmap:calendars",
           "urn:ietf:params:jmap:contacts"
       ],
       "methodCalls": [
           ["CalendarEvent/set", {"accountId": "acctDav", "update": event_noop}, "en"],
           ["CalendarEvent/set", {"accountId": "acctDav", "update": event_combined}, "ec"],
           ["ContactCard/set", {"accountId": "acctDav", "update": card_noop}, "cn"],
           ["ContactCard/set", {"accountId": "acctDav", "update": card_combined}, "cc"]
       ]
   });
   let router = build_router(state, vec!["http://example.test".into()]);
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(request.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, response) = read_body(response).await;
   assert_eq!(status, StatusCode::OK);
   let methods = response["methodResponses"].as_array().unwrap();
   assert_eq!(methods[0][1]["updated"][0], event_id);
   assert_eq!(methods[0][1]["oldState"], methods[0][1]["newState"]);
   assert_eq!(
      methods[1][1]["notUpdated"][&event_id]["type"],
      "invalidPatch"
   );
   assert_eq!(methods[2][1]["updated"][0], card_id);
   assert_eq!(methods[2][1]["oldState"], methods[2][1]["newState"]);
   assert_eq!(
      methods[3][1]["notUpdated"][&card_id]["type"],
      "invalidPatch"
   );

   let client = pool.get().await.unwrap();
   let event = store::get_resource(&client, "acctDav", &event_id)
      .await
      .unwrap()
      .unwrap();
   let card = store::get_resource(&client, "acctDav", &card_id)
      .await
      .unwrap()
      .unwrap();
   assert_ne!(event.collection_id, calendar_id);
   assert_ne!(card.collection_id, addressbook_id);
   handle.shutdown().await;
}

#[tokio::test]
async fn api_rejects_cross_account_request() {
   // acctA's bearer token requesting acctB must not reveal that acctB exists.
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Mailbox/get",
           {"accountId": "acctB"},
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(resp).await;
   assert_eq!(
      status,
      StatusCode::OK,
      "method errors still return HTTP 200"
   );

   let inv = &value["methodResponses"][0];
   assert_eq!(inv[0].as_str(), Some("error"));
   assert_eq!(inv[1]["type"].as_str(), Some("accountNotFound"));
   assert_eq!(inv[2].as_str(), Some("c0"));
}

#[tokio::test]
async fn api_same_account_succeeds() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Mailbox/get",
           {"accountId": "acctA"},
           "c1"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(resp).await;
   assert_eq!(status, StatusCode::OK);
   let inv = &value["methodResponses"][0];
   assert_eq!(inv[0].as_str(), Some("Mailbox/get"));
   assert!(inv[1]["list"].is_array());
}

#[tokio::test]
async fn email_query_anchor_pagination() {
   use chrono::{
      TimeZone as _,
      Utc,
   };
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   // Received order (DESC default sort): m4, m3, m2, m1.
   for (mid, ts) in [("m1", 100), ("m2", 200), ("m3", 300), ("m4", 400)] {
      let env = MessageEnvelope {
         msgid:              mid.into(),
         thrid:              format!("t-{mid}"),
         flags:              vec![],
         received_at:        Utc.timestamp_opt(ts, 0).unwrap(),
         sent_at:            None,
         size:               1,
         from:               None,
         to:                 None,
         cc:                 None,
         bcc:                None,
         reply_to:           None,
         subject:            None,
         preview:            None,
         has_attachment:     false,
         message_id_header:  None,
         in_reply_to_header: None,
         references_header:  None,
      };
      db::upsert_message(&pool, "acctA", &env).await.unwrap();
   }
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);
   let query = |args| {
      let router = router.clone();
      async move {
         let body = serde_json::json!({
             "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
             "methodCalls": [["Email/query", args, "c0"]]
         });
         let resp = router
            .oneshot(
               Request::builder()
                  .method("POST")
                  .uri("/api")
                  .header("authorization", format!("Bearer {token}"))
                  .header("content-type", "application/json")
                  .body(Body::from(body.to_string()))
                  .unwrap(),
            )
            .await
            .unwrap();
         read_body(resp).await.1
      }
   };

   // Anchor on m3 (index 1 in DESC order) with offset 1 → window starts at m2.
   let query_result = query(serde_json::json!({
       "accountId": "acctA", "anchor": "m3", "anchorOffset": 1, "limit": 2
   }))
   .await;
   let response = &query_result["methodResponses"][0][1];
   assert_eq!(response["position"], 2, "{query_result}");
   assert_eq!(
      response["ids"],
      serde_json::json!(["m2", "m1"]),
      "{query_result}"
   );

   // A negative position counts backwards from the full result set.
   let query_result = query(serde_json::json!({
       "accountId": "acctA", "position": -2, "limit": 2
   }))
   .await;
   let response = &query_result["methodResponses"][0][1];
   assert_eq!(response["position"], 2, "{query_result}");
   assert_eq!(
      response["ids"],
      serde_json::json!(["m2", "m1"]),
      "{query_result}"
   );
   assert!(
      response.get("limit").is_none(),
      "client limit was not clamped: {query_result}"
   );

   // Negative offset clamps to the top of the list.
   let value = query(serde_json::json!({
       "accountId": "acctA", "anchor": "m4", "anchorOffset": -5, "limit": 1
   }))
   .await;
   assert_eq!(
      value["methodResponses"][0][1]["ids"],
      serde_json::json!(["m4"]),
      "{value}"
   );

   // Unknown anchor is the spec's anchorNotFound error.
   let value = query(serde_json::json!({"accountId": "acctA", "anchor": "nope"})).await;
   assert_eq!(value["methodResponses"][0][0], "error", "{value}");
   assert_eq!(
      value["methodResponses"][0][1]["type"], "anchorNotFound",
      "{value}"
   );
}

#[tokio::test]
async fn email_changes_rejects_malformed_state() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/changes",
           {"accountId": "acctA", "sinceState": "not-a-number"},
           "c3"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   assert_eq!(
      value["methodResponses"][0][1]["type"].as_str(),
      Some("invalidArguments"),
   );
}

/// A literal `%` in a text filter must not act as a SQL wildcard.
#[tokio::test]
async fn email_query_escapes_like_wildcards() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();

   let mk = |msgid: &str, subject: &str| {
      MessageEnvelope {
         msgid:              msgid.into(),
         thrid:              msgid.into(),
         flags:              vec![],
         received_at:        Utc::now(),
         sent_at:            None,
         size:               1,
         from:               None,
         to:                 None,
         cc:                 None,
         bcc:                None,
         reply_to:           None,
         subject:            Some(subject.to_owned()),
         preview:            None,
         has_attachment:     false,
         message_id_header:  None,
         in_reply_to_header: None,
         references_header:  None,
      }
   };
   for (mid, subj) in [
      ("m1", "plain foo"),
      ("m2", "50% off"),
      ("m3", "hello_world"),
   ] {
      db::upsert_message(&pool, "acctA", &mk(mid, subj))
         .await
         .unwrap();
   }

   let token_a = "bearer-A-abcdef0123456789";
   let accounts = vec![AccountInfo::from_bearer_token(
      "acctA", "a@x.com", "A", token_a,
   )];
   let state = AppState::new(
      pool,
      accounts,
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let query = |subject| {
      serde_json::json!({
          "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
          "methodCalls": [[
              "Email/query",
              {"accountId": "acctA", "filter": {"subject": subject}},
              "c0"
          ]]
      })
      .to_string()
   };
   let run = |body| {
      let router = router.clone();
      let token = token_a.to_owned();
      async move {
         let resp = router
            .oneshot(
               Request::builder()
                  .method("POST")
                  .uri("/api")
                  .header("authorization", format!("Bearer {token}"))
                  .header("content-type", "application/json")
                  .body(Body::from(body))
                  .unwrap(),
            )
            .await
            .unwrap();
         let (_s, value) = read_body(resp).await;
         value["methodResponses"][0][1]["ids"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|value| value.as_str().unwrap_or("").to_owned())
            .collect::<Vec<_>>()
      }
   };

   // Literal `%` should only match "50% off" — not everything (which is what
   // an un-escaped LIKE wildcard would do).
   let hits = run(query("%")).await;
   assert_eq!(
      hits,
      vec!["m2"],
      "literal % must not wildcard; got {hits:?}"
   );

   // Literal `_` should only match "hello_world".
   let hits = run(query("_")).await;
   assert_eq!(
      hits,
      vec!["m3"],
      "literal _ must not wildcard; got {hits:?}"
   );

   // Plain substring still matches normally.
   let hits = run(query("foo")).await;
   assert_eq!(hits, vec!["m1"]);
}

/// R3/L1 regression: the stubbed query/change methods
/// still need to run the same account-match check the real methods do.
/// Without it, a cross-account caller gets `cannotCalculateChanges` instead
/// of `accountNotFound`, leaking the existence of another account.
#[tokio::test]
async fn stubs_reject_cross_account_requests() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [
           ["Email/queryChanges", {"accountId": "acctB", "sinceQueryState": "0"}, "q0"],
           ["Thread/changes", {"accountId": "acctB", "sinceState": "0"}, "q1"],
           ["Mailbox/queryChanges", {"accountId": "acctB", "sinceQueryState": "0"}, "q2"],
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   for idx in 0..=2 {
      let inv = &value["methodResponses"][idx];
      assert_eq!(inv[0].as_str(), Some("error"), "idx={idx}");
      assert_eq!(
         inv[1]["type"].as_str(),
         Some("accountNotFound"),
         "stub must leak nothing: idx={idx}, body={value}",
      );
   }
}

/// Advertised standard methods must be recognized even when this single-account
/// server cannot perform the requested cross-account copy.
#[tokio::test]
async fn copy_methods_return_account_errors_instead_of_unknown_method() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [
           ["Email/copy", {
               "fromAccountId": "acctB",
               "accountId": "acctA",
               "create": {}
           }, "c0"],
           ["Blob/copy", {
               "fromAccountId": "acctA",
               "accountId": "acctB",
               "blobIds": []
           }, "c1"]
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(resp).await;
   assert_eq!(status, StatusCode::OK);
   assert_eq!(value["methodResponses"][0][0], "error");
   assert_eq!(
      value["methodResponses"][0][1]["type"],
      "fromAccountNotFound"
   );
   assert_eq!(value["methodResponses"][1][0], "error");
   assert_eq!(value["methodResponses"][1][1]["type"], "accountNotFound");
}

/// Header forms we can't decode must be rejected with `invalidArguments`
/// instead of silently dropping them from the response. (bodyStructure and
/// the standard header forms are supported and exercised elsewhere.)
#[tokio::test]
async fn email_get_rejects_unsupported_properties() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   for prop in [
      "header:Foo:asBogus",
      "header:Foo:asText:all:extra",
      "header:",
   ] {
      let body = serde_json::json!({
          "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
          "methodCalls": [[
              "Email/get",
              {"accountId": "acctA", "ids": [], "properties": [prop]},
              "c0"
          ]]
      });
      let resp = router
         .clone()
         .oneshot(
            Request::builder()
               .method("POST")
               .uri("/api")
               .header("authorization", format!("Bearer {token_a}"))
               .header("content-type", "application/json")
               .body(Body::from(body.to_string()))
               .unwrap(),
         )
         .await
         .unwrap();
      let (_status, value) = read_body(resp).await;
      let inv = &value["methodResponses"][0];
      assert_eq!(inv[0].as_str(), Some("error"), "property {prop}: {value}");
      assert_eq!(
         inv[1]["type"].as_str(),
         Some("invalidArguments"),
         "property {prop}: {value}",
      );
   }
}

/// R3/L2 regression: Mailbox/get must honor the `properties` filter so
/// property-based client caches stay correct. Previously any caller asking
/// for a narrow subset got back every Mailbox field regardless.
#[tokio::test]
async fn mailbox_get_applies_properties_filter() {
   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb1", "acctA", "Inbox", None, Some("inbox"), 0)
      .await
      .unwrap();
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Mailbox/get",
           {"accountId": "acctA", "properties": ["name", "role"]},
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   let entry = &value["methodResponses"][0][1]["list"][0];
   let keys = entry
      .as_object()
      .unwrap()
      .keys()
      .cloned()
      .collect::<BTreeSet<String>>();
   // `id` is always retained; other requested props should be present; the
   // rest (totalEmails, unreadEmails, myRights, etc.) must be stripped.
   assert!(keys.contains("id"));
   assert!(keys.contains("name"));
   assert!(keys.contains("role"));
   assert!(
      !keys.contains("totalEmails"),
      "properties filter leaked extras: {keys:?}"
   );
   assert!(!keys.contains("myRights"), "leaked myRights: {keys:?}");
}

/// Thread/get with null ids enumerates distinct threads.
#[tokio::test]
async fn thread_get_null_ids_returns_all() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let mk = |msgid: &str, thrid: &str| {
      MessageEnvelope {
         msgid:              msgid.into(),
         thrid:              thrid.into(),
         flags:              vec![],
         received_at:        Utc::now(),
         sent_at:            None,
         size:               1,
         from:               None,
         to:                 None,
         cc:                 None,
         bcc:                None,
         reply_to:           None,
         subject:            None,
         preview:            None,
         has_attachment:     false,
         message_id_header:  None,
         in_reply_to_header: None,
         references_header:  None,
      }
   };
   for (mid, tid) in [("m1", "t1"), ("m2", "t2"), ("m3", "t2")] {
      db::upsert_message(&pool, "acctA", &mk(mid, tid))
         .await
         .unwrap();
   }
   let email_state = db::get_state(&pool, "acctA").await.unwrap().email_modseq;
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Thread/get",
           {"accountId": "acctA", "ids": null},
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   assert_eq!(
      value["methodResponses"][0][1]["state"],
      email_state.to_string()
   );
   let list = value["methodResponses"][0][1]["list"].as_array().unwrap();
   let thread_ids = list
      .iter()
      .map(|thread| thread["id"].as_str().unwrap().to_owned())
      .collect::<BTreeSet<String>>();
   assert_eq!(
      thread_ids,
      ["t1", "t2"].iter().map(ToString::to_string).collect()
   );
}

#[tokio::test]
async fn email_submission_resolves_created_email_id() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;
   use jmap_protocol::email::EmailAddress;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   db::upsert_message(&pool, "acctA", &MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "t1".into(),
      flags:              vec!["$draft".into()],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               Some(vec![EmailAddress {
         name:  None,
         email: "a@x.com".into(),
      }]),
      to:                 Some(vec![EmailAddress {
         name:  None,
         email: "b@x.com".into(),
      }]),
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   })
   .await
   .unwrap();

   let (tx, mut rx) = mpsc::channel::<AccountRequest>(1);
   tokio::spawn(async move {
      let Some(AccountRequest::SubmitEmail {
         msgid,
         mail_from,
         rcpt_to,
         respond,
      }) = rx.recv().await
      else {
         return;
      };
      assert_eq!(msgid, "m1");
      assert_eq!(mail_from, "a@x.com");
      assert_eq!(rcpt_to, ["b@x.com"]);
      let _ = respond.send(Ok("250 2.0.0 queued".into()));
   });

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::from([("acctA".to_owned(), tx)]),
   );
   let router = build_router(state, vec!["http://example.test".into()]);
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "createdIds": {"draft": "m1"},
       "methodCalls": [[
           "EmailSubmission/set",
           {"accountId": "acctA", "create": {"send": {
               "emailId": "#draft",
               "identityId": "ident-acctA"
           }}},
           "c0"
       ]]
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(response).await;

   assert_eq!(status, StatusCode::OK);
   assert_eq!(
      value["methodResponses"][0][1]["created"]["send"]["deliveryStatus"]["b@x.com"]["smtpReply"],
      "250 2.0.0 queued",
      "{value}"
   );
}

/// Delayed send lifecycle without SMTP: create with a future sendAt queues
/// a pending submission (raw bytes staged from the cache), cancel flips it
/// to canceled, and destroy tombstones it. Stale /changes fails safely until
/// creation history is stored; the current state has an exact empty delta.
#[tokio::test]
async fn email_submission_delayed_pending_cancel_destroy() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;
   use jmap_protocol::email::EmailAddress;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "t1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 Some(vec![EmailAddress {
         name:  None,
         email: "b@x.com".into(),
      }]),
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();
   let raw = b"From: a@x.com\r\nTo: b@x.com\r\nSubject: hi\r\n\r\nHello\r\n";
   exec(
      &pool,
      "INSERT INTO raw_messages (account_id, msgid, headers_json, body_values_json, \
       attachments_json, raw_rfc822, fetched_at) VALUES ($1, $2, '{}', '{}', '[]', $3, \
       EXTRACT(EPOCH FROM now())::bigint)",
      &[&"acctA", &"m1", &&raw[..]],
   )
   .await;

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);
   let call = |body: serde_json::Value| {
      let router = router.clone();
      let token = token.to_owned();
      async move {
         let resp = router
            .oneshot(
               Request::builder()
                  .method("POST")
                  .uri("/api")
                  .header("authorization", format!("Bearer {token}"))
                  .header("content-type", "application/json")
                  .body(Body::from(body.to_string()))
                  .unwrap(),
            )
            .await
            .unwrap();
         read_body(resp).await.1
      }
   };

   let send_at =
      (Utc::now() + chrono::Duration::hours(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
   let value = call(serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [[
           "EmailSubmission/set",
           {"accountId": "acctA", "create": {"s1": {
               "emailId": "m1",
               "identityId": "ident-acctA",
               "sendAt": send_at
           }}},
           "c0"
       ]]
   }))
   .await;
   let created = &value["methodResponses"][0][1]["created"]["s1"];
   assert_eq!(created["undoStatus"], "pending", "{value}");
   let sub_id = created["id"].as_str().unwrap().to_owned();

   // Cancel while pending succeeds.
   let value = call(serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [[
           "EmailSubmission/set",
           {"accountId": "acctA", "update": {sub_id.clone(): {"undoStatus": "canceled"}}},
           "c0"
       ]]
   }))
   .await;
   assert!(
      value["methodResponses"][0][1]["updated"]
         .as_object()
         .unwrap()
         .contains_key(&sub_id),
      "{value}"
   );

   let value = call(serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [[
           "EmailSubmission/get",
           {"accountId": "acctA", "ids": [sub_id.clone()]},
           "c0"
       ]]
   }))
   .await;
   assert_eq!(
      value["methodResponses"][0][1]["list"][0]["undoStatus"], "canceled",
      "{value}"
   );

   // Destroy the canceled row; a second cancel now reports notFound.
   let value = call(serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [[
           "EmailSubmission/set",
           {"accountId": "acctA", "destroy": [sub_id.clone()]},
           "c0"
       ]]
   }))
   .await;
   assert_eq!(
      value["methodResponses"][0][1]["destroyed"][0], sub_id,
      "{value}"
   );
   let current_state = value["methodResponses"][0][1]["newState"]
      .as_str()
      .unwrap()
      .to_owned();

   let value = call(serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [[
           "EmailSubmission/changes",
           {"accountId": "acctA", "sinceState": "0"},
           "c0"
       ]]
   }))
   .await;
   assert_eq!(value["methodResponses"][0][0], "error", "{value}");
   assert_eq!(
      value["methodResponses"][0][1]["type"], "cannotCalculateChanges",
      "{value}"
   );

   let value = call(serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [[
           "EmailSubmission/changes",
           {"accountId": "acctA", "sinceState": current_state},
           "c1"
       ]]
   }))
   .await;
   assert_eq!(
      value["methodResponses"][0][0], "EmailSubmission/changes",
      "{value}"
   );
   assert_eq!(
      value["methodResponses"][0][1]["created"],
      serde_json::json!([])
   );
   assert_eq!(
      value["methodResponses"][0][1]["updated"],
      serde_json::json!([])
   );
   assert_eq!(
      value["methodResponses"][0][1]["destroyed"],
      serde_json::json!([])
   );
}

/// Every dynamic filter/sort combination must be executable Postgres, not
/// just a well-formed string — this exercises the whole vocabulary against
/// the real database so dialect drift (instr vs strpos, `json_extract` vs
/// jsonb operators) fails loudly.
#[tokio::test]
async fn email_query_filter_vocabulary_executes() {
   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let filter = serde_json::json!({
       "operator": "AND",
       "conditions": [
           {"inMailbox": "mb-1"},
           {"inMailboxOtherThan": ["mb-2"]},
           {"before": "2030-01-01T00:00:00Z", "after": "2000-01-01T00:00:00Z"},
           {"minSize": 1, "maxSize": 10_000_000},
           {"subject": "s", "from": "f", "to": "t", "cc": ["c"], "bcc": ["b"]},
           {"text": "x", "body": "y"},
           {"hasKeyword": "$seen", "notKeyword": "$draft", "hasAttachment": false},
           {"operator": "NOT", "conditions": [{"someInThreadHaveKeyword": "$flagged"}]},
           {"allInThreadHaveKeyword": "$seen"},
           {"noneInThreadHaveKeyword": "$junk"}
       ]
   });
   for (sort, collapse) in [
      ("receivedAt", false),
      ("sentAt", false),
      ("size", true),
      ("subject", true),
      ("from", true),
      ("to", false),
   ] {
      let body = serde_json::json!({
          "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
          "methodCalls": [[
              "Email/query",
              {
                  "accountId": "acctA",
                  "filter": filter,
                  "sort": [{"property": sort}],
                  "collapseThreads": collapse,
                  "calculateTotal": true
              },
              "c0"
          ]]
      });
      let resp = router
         .clone()
         .oneshot(
            Request::builder()
               .method("POST")
               .uri("/api")
               .header("authorization", format!("Bearer {token}"))
               .header("content-type", "application/json")
               .body(Body::from(body.to_string()))
               .unwrap(),
         )
         .await
         .unwrap();
      let (_s, value) = read_body(resp).await;
      assert_eq!(
         value["methodResponses"][0][0], "Email/query",
         "sort={sort} collapse={collapse}: {value}"
      );
      assert_eq!(value["methodResponses"][0][1]["total"], 0, "{value}");
   }
}

/// collapseThreads: one representative per thread, picked by the requested
/// sort; total counts distinct threads.
#[tokio::test]
async fn email_query_collapse_threads() {
   use chrono::{
      TimeZone as _,
      Utc,
   };
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let mk = |msgid: &str, thrid: &str, ts: i64| {
      MessageEnvelope {
         msgid:              msgid.into(),
         thrid:              thrid.into(),
         flags:              vec![],
         received_at:        Utc.timestamp_opt(ts, 0).unwrap(),
         sent_at:            None,
         size:               1,
         from:               None,
         to:                 None,
         cc:                 None,
         bcc:                None,
         reply_to:           None,
         subject:            None,
         preview:            None,
         has_attachment:     false,
         message_id_header:  None,
         in_reply_to_header: None,
         references_header:  None,
      }
   };
   // t1 has two messages (m2 newest), t2 has one.
   for (mid, tid, ts) in [("m1", "t1", 100), ("m2", "t1", 300), ("m3", "t2", 200)] {
      db::upsert_message(&pool, "acctA", &mk(mid, tid, ts))
         .await
         .unwrap();
   }
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/query",
           {"accountId": "acctA", "collapseThreads": true, "calculateTotal": true},
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   let ids = value["methodResponses"][0][1]["ids"]
      .as_array()
      .unwrap()
      .iter()
      .map(|i| i.as_str().unwrap())
      .collect::<Vec<&str>>();
   // Default sort is receivedAt DESC: t1 is represented by its newest
   // member m2 (ts 300), then t2's m3 (ts 200). m1 is collapsed away.
   assert_eq!(ids, vec!["m2", "m3"]);
   assert_eq!(value["methodResponses"][0][1]["total"], 2);
}

/// R3/M1 regression: `/readyz` must be scoped to the live account set. If an
/// account that previously completed initial sync is removed on reload and
/// a new unsynced account takes its place, readyz must 503 — not green-flag
/// the service based on the stale synced row.
#[tokio::test]
async fn readyz_ignores_stale_synced_rows_after_churn() {
   use std::str;

   let Some(pool) = setup_pool().await else {
      return;
   };
   // Both accounts start seeded in the DB. `old-acct` has completed initial
   // sync; `new-acct` has not.
   db::upsert_account(&pool, "old-acct", "o@x.com", ProviderKind::Imap, "O", b"oh")
      .await
      .unwrap();
   db::upsert_account(&pool, "new-acct", "n@x.com", ProviderKind::Imap, "N", b"nh")
      .await
      .unwrap();
   db::mark_initial_sync_done(&pool, "old-acct").await.unwrap();

   // Live AppState only lists `new-acct` — mirrors a config reload that
   // swapped accounts.
   let token = "bearer-N-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "new-acct", "n@x.com", "N", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let resp = router
      .oneshot(
         Request::builder()
            .uri("/readyz")
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(
      resp.status(),
      StatusCode::SERVICE_UNAVAILABLE,
      "stale synced rows must not unblock readyz for an unsynced live account",
   );
   let bytes = resp.into_body().collect().await.unwrap().to_bytes();
   let txt = str::from_utf8(&bytes).unwrap_or_default();
   assert!(
      txt.contains("0/1"),
      "readyz body should report 0/1 live accounts synced; got {txt:?}",
   );
}

/// R3/M2 regression: when body properties are requested but the account has
/// no live sync task (channel not wired up), the error must surface as
/// `serverFail` instead of silently returning an email with empty body
/// fields — that looked to clients like "message has no body".
#[tokio::test]
async fn email_get_body_fetch_failure_surfaces_server_fail() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   // Insert one envelope row so Email/get finds something to return; we
   // don't seed `raw_messages`, so ensure_bodies_cached will route through
   // the (absent) account sender and fail.
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "m1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(), // no account_senders → body fetch channel is absent
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/get",
           {
               "accountId": "acctA",
               "ids": ["m1"],
               "fetchTextBodyValues": true
           },
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   let inv = &value["methodResponses"][0];
   assert_eq!(inv[0].as_str(), Some("error"), "body: {value}");
   assert_eq!(
      inv[1]["type"].as_str(),
      Some("serverFail"),
      "body fetch failure must surface as serverFail; got {value}",
   );
}

#[tokio::test]
async fn email_get_honors_body_value_and_body_property_arguments() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;
   use jmapper_codegen::queries::raw_messages;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let envelope = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "m1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("body args".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &envelope).await.unwrap();
   let raw = b"From: a@x.com\r\n\
Content-Type: multipart/alternative; boundary=a\r\n\
\r\n\
--a\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
plain body\r\n\
--a\r\n\
Content-Type: text/html; charset=utf-8\r\n\
\r\n\
<p>html body</p>\r\n\
--a--\r\n";
   let message = mail_parser::MessageParser::default().parse(raw).unwrap();
   let projected = sync::project_to_jmap_with_msgid(&message, Some("m1"));
   let headers_json = serde_json::json!({
       "headers": projected.headers,
       "textBody": projected.text_body,
       "htmlBody": projected.html_body,
   })
   .to_string();
   let body_values_json = serde_json::to_string(&projected.body_values).unwrap();
   let attachments_json = serde_json::to_string(&projected.attachments).unwrap();
   raw_messages::upsert_raw_message()
      .bind(
         &pool.get().await.unwrap(),
         &"acctA",
         &"m1",
         &headers_json.as_str(),
         &body_values_json.as_str(),
         &attachments_json.as_str(),
         &raw.as_slice(),
      )
      .await
      .unwrap();

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [
           ["Email/get", {
               "accountId": "acctA",
               "ids": ["m1"],
               "properties": ["bodyValues", "textBody", "bodyStructure"],
               "bodyProperties": ["partId", "header:Content-Type"],
               "fetchTextBodyValues": true,
               "maxBodyValueBytes": 0
           }, "selected"],
           ["Email/get", {
               "accountId": "acctA",
               "ids": ["m1"],
               "properties": ["bodyValues"]
           }, "empty"]
       ]
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(response).await;
   assert_eq!(status, StatusCode::OK);
   let selected = &value["methodResponses"][0][1]["list"][0];
   let part_id = selected["textBody"][0]["partId"].as_str().unwrap();
   assert_eq!(selected["bodyValues"][part_id]["value"], "plain body");
   assert_eq!(selected["bodyValues"][part_id]["isTruncated"], false);
   assert!(
      selected["textBody"][0]["header:Content-Type"]
         .as_str()
         .unwrap()
         .contains("text/plain")
   );
   assert!(selected["bodyStructure"]["subParts"].is_array());
   assert_eq!(
      value["methodResponses"][1][1]["list"][0]["bodyValues"],
      serde_json::json!({})
   );
}

/// R3 regression: Thread/get(properties=[...]) must strip unrequested keys,
/// mirroring `mailbox_get_applies_properties_filter`.
#[tokio::test]
async fn thread_get_applies_properties_filter() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "t1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            None,
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   // Ask for *only* `id`. The `emailIds` field must not appear.
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Thread/get",
           {"accountId": "acctA", "ids": ["t1"], "properties": ["id"]},
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   let entry = &value["methodResponses"][0][1]["list"][0];
   let keys = entry
      .as_object()
      .unwrap()
      .keys()
      .cloned()
      .collect::<BTreeSet<String>>();
   assert!(keys.contains("id"));
   assert!(
      !keys.contains("emailIds"),
      "Thread/get properties filter must strip emailIds when not requested: {keys:?}",
   );
}

/// Upload returns a content-addressed blob id and stores the bytes.
#[tokio::test]
async fn upload_returns_blob_id_and_persists_bytes() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let pool = state.pool().clone();
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = b"raw rfc822 draft bytes";
   let resp = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/upload/acctA")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "message/rfc822")
            .body(Body::from(body.to_vec()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(resp).await;
   assert_eq!(status, StatusCode::OK);
   let blob_id = value["blobId"].as_str().unwrap();
   assert!(
      blob_id.starts_with("blob-upload-"),
      "unexpected blobId {blob_id}"
   );
   assert_eq!(value["accountId"].as_str(), Some("acctA"));
   assert_eq!(value["type"].as_str(), Some("message/rfc822"));
   assert_eq!(value["size"].as_u64(), Some(body.len() as u64));

   let stored = pool
      .get()
      .await
      .unwrap()
      .query_one(
         "SELECT bytes FROM uploaded_blobs WHERE account_id = 'acctA' AND blob_id = $1",
         &[&blob_id],
      )
      .await
      .unwrap()
      .get::<_, Vec<u8>>(0);
   assert_eq!(stored.as_slice(), body);

   let resp = router
      .clone()
      .oneshot(
         Request::builder()
            .uri(format!(
               "/download/acctA/{blob_id}/draft.eml?type=text/plain"
            ))
            .header("authorization", format!("Bearer {token_a}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(resp.status(), StatusCode::OK);
   assert_eq!(
      resp
         .headers()
         .get("content-type")
         .and_then(|value| value.to_str().ok()),
      Some("text/plain")
   );
   assert_eq!(
      resp
         .headers()
         .get("cache-control")
         .and_then(|value| value.to_str().ok()),
      Some("private, immutable, max-age=31536000")
   );
   assert!(
      resp
         .headers()
         .get("content-disposition")
         .and_then(|value| value.to_str().ok())
         .is_some_and(|value| value.contains("draft.eml"))
   );
   let downloaded = resp.into_body().collect().await.unwrap().to_bytes();
   assert_eq!(downloaded.as_ref(), body);

   pool
      .get()
      .await
      .unwrap()
      .execute(
         "UPDATE uploaded_blobs SET expires_at = 0 WHERE account_id = 'acctA' AND blob_id = $1",
         &[&blob_id],
      )
      .await
      .unwrap();
   let resp = router
      .oneshot(
         Request::builder()
            .uri(format!("/download/acctA/{blob_id}/draft.eml"))
            .header("authorization", format!("Bearer {token_a}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn email_parse_decodes_mime_and_returns_downloadable_parts() {
   let Some((state, token_a, token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);
   let raw = b"From: Alice <alice@example.test>\r\n\
To: Bob <bob@example.test>\r\n\
Message-ID: <parse-1@example.test>\r\n\
Date: Wed, 15 Jul 2026 10:00:00 +0000\r\n\
Subject: =?UTF-8?Q?Parsed_=E2=9C=93?=\r\n\
X-Test: decoded\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=m\r\n\
\r\n\
--m\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
Hello =F0=9F=8E=89\r\n\
--m\r\n\
Content-Type: application/octet-stream; name=data.bin\r\n\
Content-Disposition: attachment; filename=data.bin\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
AAEC\r\n\
--m--\r\n";
   let upload = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/upload/acctA")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "message/rfc822")
            .body(Body::from(raw.as_slice()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, uploaded) = read_body(upload).await;
   assert_eq!(status, StatusCode::OK);
   let source_blob_id = uploaded["blobId"].as_str().unwrap();

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/parse",
           {
               "accountId": "acctA",
               "blobIds": [source_blob_id, "blob-upload-missing"],
               "properties": [
                   "blobId", "messageId", "from", "subject", "headers",
                   "bodyValues", "textBody", "attachments", "bodyStructure",
                   "header:X-Test:asText"
               ],
               "bodyProperties": [
                   "partId", "blobId", "size", "type", "name", "headers", "subParts"
               ],
               "fetchAllBodyValues": true,
               "maxBodyValueBytes": 8
           },
           "parse"
       ]]
   });
   let response = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(response).await;
   assert_eq!(status, StatusCode::OK);
   let result = &value["methodResponses"][0];
   assert_eq!(result[0], "Email/parse", "{value}");
   assert_eq!(result[1]["notFound"][0], "blob-upload-missing");
   let email = &result[1]["parsed"][source_blob_id];
   assert_eq!(email["blobId"], source_blob_id);
   assert_eq!(email["subject"], "Parsed ✓");
   assert_eq!(email["messageId"][0], "parse-1@example.test");
   assert_eq!(email["from"][0]["email"], "alice@example.test");
   assert_eq!(email["header:X-Test:asText"], "decoded");
   assert!(email["headers"].as_array().unwrap().len() >= 8);
   assert_eq!(email["attachments"][0]["name"], "data.bin");
   assert!(email["bodyStructure"]["subParts"].is_array());

   let text_part = &email["textBody"][0];
   let part_id = text_part["partId"].as_str().unwrap();
   let part_blob_id = text_part["blobId"].as_str().unwrap();
   assert!(part_id.starts_with('p'));
   assert_eq!(email["bodyValues"][part_id]["value"], "Hello ");
   assert_eq!(email["bodyValues"][part_id]["isTruncated"], true);
   let download = router
      .clone()
      .oneshot(
         Request::builder()
            .uri(format!("/download/acctA/{part_blob_id}/body.txt"))
            .header("authorization", format!("Bearer {token_a}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(download.status(), StatusCode::OK);
   let bytes = download.into_body().collect().await.unwrap().to_bytes();
   assert!(bytes.starts_with("Hello 🎉".as_bytes()), "{bytes:?}");

   let isolated = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/parse",
           {"accountId": "acctB", "blobIds": [source_blob_id]},
           "parse"
       ]]
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_b}"))
            .header("content-type", "application/json")
            .body(Body::from(isolated.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_, value) = read_body(response).await;
   assert_eq!(
      value["methodResponses"][0][1]["notFound"][0],
      source_blob_id
   );
}

#[tokio::test]
async fn push_subscription_methods_dispatch_without_enabling_webhooks() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core"],
       "methodCalls": [
           ["PushSubscription/get", {"ids": null}, "get"],
           ["PushSubscription/get", {"properties": ["url"]}, "private"],
           ["PushSubscription/set", {
               "create": {"new": {"url": "https://example.test/push"}}
           }, "set"]
       ]
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(response).await;
   assert_eq!(status, StatusCode::OK);
   assert_eq!(value["methodResponses"][0][0], "PushSubscription/get");
   assert_eq!(
      value["methodResponses"][0][1]["list"],
      serde_json::json!([])
   );
   assert_eq!(value["methodResponses"][1][0], "error");
   assert_eq!(value["methodResponses"][1][1]["type"], "forbidden");
   assert_eq!(value["methodResponses"][2][0], "PushSubscription/set");
   assert_eq!(
      value["methodResponses"][2][1]["notCreated"]["new"]["type"],
      "forbidden"
   );
}

/// Upload to another account's URL is 401, even with a valid
/// bearer token.
#[tokio::test]
async fn upload_cross_account_is_401() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/upload/acctB")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "text/plain")
            .body(Body::from("hello"))
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// Email/import with no live sync task surfaces `serverUnavailable`
/// per-entry in notCreated rather than failing the whole method.
#[tokio::test]
async fn email_import_without_sync_task_is_server_unavailable() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/import",
           {
               "accountId": "acctA",
               "emails": {
                   "c1": {
                       "blobId": "blob-upload-deadbeef",
                       "mailboxIds": {"mb1": true}
                   }
               }
           },
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   let inv = &value["methodResponses"][0];
   assert_eq!(inv[0].as_str(), Some("Email/import"), "{value}");
   assert_eq!(inv[1]["notCreated"]["c1"]["type"], "serverUnavailable");
}

/// Spawn a fake account task that acknowledges every `StoreFlags` request,
/// applying the delta directly to `messages.flags_json` so Email/get can see
/// the new state. Simulates the production `sync::store_flags_batch` path
/// without needing a live IMAP connection.
fn spawn_fake_store_task(pool: PgPool, account_id: String) -> mpsc::Sender<AccountRequest> {
   let (tx, mut rx) = mpsc::channel::<AccountRequest>(16);
   tokio::spawn(async move {
      while let Some(req) = rx.recv().await {
         if let AccountRequest::StoreFlags {
            msgid,
            add,
            remove,
            respond,
         } = req
         {
            let cur = pool
               .get()
               .await
               .unwrap()
               .query_opt(
                  "SELECT flags_json FROM messages WHERE account_id = $1 AND msgid = $2",
                  &[&account_id, &msgid],
               )
               .await
               .unwrap()
               .map(|row| row.get::<_, String>(0));
            let mut flags = cur
               .as_deref()
               .map(|json| serde_json::from_str::<Vec<String>>(json).unwrap_or_default())
               .unwrap_or_default();
            for flag in &remove {
               let keyword = sync::imap_flag_to_keyword(flag);
               flags.retain(|existing| existing != &keyword);
            }
            for flag in &add {
               let keyword = sync::imap_flag_to_keyword(flag);
               if !flags.contains(&keyword) {
                  flags.push(keyword);
               }
            }
            let new_modseq = db::bump_modseq(&pool, &account_id, db::StateKind::Email)
               .await
               .unwrap();
            exec(
               &pool,
               "UPDATE messages SET flags_json = $1, modseq = $2 WHERE account_id = $3 AND msgid \
                = $4",
               &[
                  &serde_json::to_string(&flags).unwrap(),
                  &(new_modseq as i64),
                  &account_id,
                  &msgid,
               ],
            )
            .await;
            let _ = respond.send(Ok(()));
         }
      }
   });
   tx
}

/// Identity/get returns exactly one identity per account, derived
/// from the authenticated account's email and `display_name`.
#[tokio::test]
async fn identity_get_returns_one_identity() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [
           ["Identity/get", {"accountId": "acctA"}, "c0"],
           ["Identity/set", {
               "accountId": "acctA",
               "update": {"ident-acctA": {"name": "Other"}}
           }, "c1"]
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(resp).await;
   assert_eq!(status, StatusCode::OK);
   let list = value["methodResponses"][0][1]["list"].as_array().unwrap();
   assert_eq!(list.len(), 1);
   assert_eq!(list[0]["email"].as_str(), Some("a@x.com"));
   assert_eq!(list[0]["name"].as_str(), Some("A"));
   assert_eq!(list[0]["mayDelete"].as_bool(), Some(false));
   assert_eq!(value["methodResponses"][1][0], "Identity/set");
   assert_eq!(
      value["methodResponses"][1][1]["notUpdated"]["ident-acctA"]["type"],
      "forbidden"
   );
}

#[tokio::test]
async fn advertised_object_limits_return_request_too_large() {
   use jmap_protocol::session::MAX_OBJECTS_IN_GET;

   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);
   let ids = (0..=MAX_OBJECTS_IN_GET)
      .map(|index| format!("id-{index}"))
      .collect::<Vec<_>>();
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
       "methodCalls": [
           ["Identity/get", {"accountId": "acctA", "ids": ids}, "get"],
           ["Identity/set", {"accountId": "acctA", "destroy": ids}, "set"]
       ]
   });
   let response = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(response).await;
   assert_eq!(status, StatusCode::OK);
   for invocation in value["methodResponses"].as_array().unwrap() {
      assert_eq!(invocation[0], "error");
      assert_eq!(invocation[1]["type"], "requestTooLarge");
   }
}

/// The RFC 9425 surface exposes one static "Mail" quota with an
/// unknown hard limit, including the required query methods.
#[tokio::test]
async fn quota_get_reports_unknown_limit() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:quota"],
       "methodCalls": [
           ["Quota/get", {"accountId": "acctA"}, "c0"],
           ["Quota/changes", {
               "accountId": "acctA",
               "#sinceState": {"resultOf": "c0", "name": "Quota/get", "path": "/state"}
           }, "c1"],
           ["Quota/query", {
               "accountId": "acctA",
               "filter": {"resourceType": "octets", "type": "Email"},
               "calculateTotal": true
           }, "c2"],
           ["Quota/queryChanges", {
               "accountId": "acctA",
               "sinceQueryState": "stale"
           }, "c3"]
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   let entry = &value["methodResponses"][0][1]["list"][0];
   assert_eq!(entry["name"].as_str(), Some("Mail"));
   assert!(entry["hardLimit"].is_null());
   assert_eq!(entry["used"].as_u64(), Some(0));
   assert!(value["methodResponses"][1][1]["updatedProperties"].is_null());
   assert_eq!(value["methodResponses"][2][0], "Quota/query");
   assert_eq!(value["methodResponses"][2][1]["ids"][0], "quota-acctA");
   assert_eq!(value["methodResponses"][2][1]["total"], 1);
   assert_eq!(value["methodResponses"][3][0], "error");
   assert_eq!(
      value["methodResponses"][3][1]["type"],
      "cannotCalculateChanges"
   );
}

/// VacationResponse/get returns the `singleton` object with
/// `isEnabled: false`.
#[tokio::test]
async fn vacation_response_get_is_disabled_singleton() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:vacationresponse"],
       "methodCalls": [
           ["VacationResponse/get", {"accountId": "acctA"}, "c0"],
           ["VacationResponse/set", {
               "accountId": "acctA",
               "update": {"singleton": {"isEnabled": true}}
           }, "c1"]
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   let list = value["methodResponses"][0][1]["list"].as_array().unwrap();
   assert_eq!(list.len(), 1);
   assert_eq!(list[0]["id"].as_str(), Some("singleton"));
   assert_eq!(list[0]["isEnabled"].as_bool(), Some(false));
   assert_eq!(value["methodResponses"][1][0], "VacationResponse/set");
   assert_eq!(
      value["methodResponses"][1][1]["notUpdated"]["singleton"]["type"],
      "forbidden"
   );
}

/// Email invalidation also invalidates its derived Thread objects.
#[tokio::test]
async fn state_change_broadcast_reaches_subscribers() {
   use std::time::Duration;

   use jmap_server::state::{
      StateChange,
      StateKind,
   };
   use tokio::time;

   let Some((state, _token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let mut rx = state.state_changes();

   state.publish_state_change(StateChange {
      account_id: "acctA".into(),
      kind:       StateKind::Email,
      new_state:  "42".into(),
   });
   let change = time::timeout(Duration::from_millis(500), rx.recv())
      .await
      .expect("broadcast should deliver within 500ms")
      .expect("recv should not error");
   assert_eq!(change.account_id, "acctA");
   assert_eq!(change.kind, StateKind::Email);
   assert_eq!(change.new_state, "42");
   let thread = rx.recv().await.unwrap();
   assert_eq!(thread.kind, StateKind::Thread);
   assert_eq!(thread.new_state, "42");
}

/// Spawn a fake account task that handles `MutateMailboxes` + `DestroyMessage`,
/// updating the DB as if the IMAP exchange succeeded. Tests use this
/// to verify dispatch without needing a live IMAP session.
fn spawn_fake_move_destroy_task(pool: PgPool, account_id: String) -> mpsc::Sender<AccountRequest> {
   let (tx, mut rx) = mpsc::channel::<AccountRequest>(16);
   tokio::spawn(async move {
      while let Some(req) = rx.recv().await {
         match req {
            AccountRequest::MutateMailboxes {
               msgid,
               add,
               remove,
               respond,
            } => {
               for mb in &add {
                  exec(
                     &pool,
                     "INSERT INTO message_mailboxes (account_id, msgid, mailbox_id) VALUES ($1, \
                      $2, $3) ON CONFLICT DO NOTHING",
                     &[&account_id, &msgid, mb],
                  )
                  .await;
               }
               for mb in &remove {
                  exec(
                     &pool,
                     "DELETE FROM message_mailboxes WHERE account_id = $1 AND msgid = $2 AND \
                      mailbox_id = $3",
                     &[&account_id, &msgid, mb],
                  )
                  .await;
               }
               let _ = db::bump_modseq(&pool, &account_id, db::StateKind::Email).await;
               let _ = respond.send(Ok(()));
            },
            AccountRequest::DestroyMessage { msgid, respond } => {
               exec(
                  &pool,
                  "DELETE FROM messages WHERE account_id = $1 AND msgid = $2",
                  &[&account_id, &msgid],
               )
               .await;
               let _ = db::bump_modseq(&pool, &account_id, db::StateKind::Email).await;
               let _ = respond.send(Ok(()));
            },
            _ => {},
         }
      }
   });
   tx
}

/// Email/set update with mailboxIds/newMb routes through the
/// account task; the DB membership reflects the move after the call.
#[tokio::test]
async fn email_set_moves_mailbox() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb-inbox", "acctA", "Inbox", None, Some("inbox"), 0)
      .await
      .unwrap();
   db::upsert_mailbox(
      &pool,
      "mb-archive",
      "acctA",
      "Archive",
      None,
      Some("archive"),
      1,
   )
   .await
   .unwrap();
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "m1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();
   exec(
      &pool,
      "INSERT INTO message_mailboxes (account_id, msgid, mailbox_id) VALUES ($1, $2, $3)",
      &[&"acctA", &"m1", &"mb-inbox"],
   )
   .await;

   let tx = spawn_fake_move_destroy_task(pool.clone(), "acctA".into());
   let mut senders = HashMap::new();
   senders.insert("acctA".into(), tx);
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool.clone(),
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      senders,
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/set",
           {
               "accountId": "acctA",
               "update": {
                   "m1": {
                       "mailboxIds/mb-archive": true,
                       "mailboxIds/mb-inbox": false
                   }
               }
           },
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   assert_eq!(
      value["methodResponses"][0][1]["updated"]["m1"],
      serde_json::Value::Null
   );

   let memberships = pool
      .get()
      .await
      .unwrap()
      .query(
         "SELECT mailbox_id FROM message_mailboxes WHERE account_id = 'acctA' AND msgid = 'm1' \
          ORDER BY mailbox_id",
         &[],
      )
      .await
      .unwrap()
      .into_iter()
      .map(|row| row.get::<_, String>(0))
      .collect::<Vec<String>>();
   assert_eq!(memberships, vec!["mb-archive"]);
}

/// Email/set destroy routes through `DestroyMessage` and drops the
/// row. `destroyed` in the response lists the deleted id.
#[tokio::test]
async fn email_set_destroy_removes_row() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "m1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();

   let tx = spawn_fake_move_destroy_task(pool.clone(), "acctA".into());
   let mut senders = HashMap::new();
   senders.insert("acctA".into(), tx);
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool.clone(),
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      senders,
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/set",
           {"accountId": "acctA", "destroy": ["m1"]},
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   let destroyed = value["methodResponses"][0][1]["destroyed"]
      .as_array()
      .unwrap();
   assert_eq!(
      destroyed
         .iter()
         .map(|x| x.as_str().unwrap())
         .collect::<Vec<_>>(),
      vec!["m1"]
   );

   let n = pool
      .get()
      .await
      .unwrap()
      .query_one("SELECT COUNT(*) FROM messages WHERE msgid = 'm1'", &[])
      .await
      .unwrap()
      .get::<_, i64>(0);
   assert_eq!(n, 0);
}

/// Email/set update clears `$flagged` via the pointer patch form;
/// Email/get afterwards reports the new keyword map and a bumped modseq.
#[tokio::test]
async fn email_set_clears_flagged_keyword() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "m1".into(),
      flags:              vec!["$flagged".into()],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();

   let tx = spawn_fake_store_task(pool.clone(), "acctA".into());
   let token = "bearer-A-abcdef0123456789";
   let mut senders = HashMap::new();
   senders.insert("acctA".to_owned(), tx);
   let state = AppState::new(
      pool.clone(),
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      senders,
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/set",
           {
               "accountId": "acctA",
               "update": {
                   "m1": {"keywords/$flagged": false, "keywords/$seen": true}
               }
           },
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(resp).await;
   assert_eq!(status, StatusCode::OK);

   let inv = &value["methodResponses"][0];
   assert_eq!(inv[0].as_str(), Some("Email/set"), "body: {value}");
   assert_eq!(inv[1]["updated"]["m1"], serde_json::Value::Null);
   assert!(inv[1]["oldState"].is_string());
   assert!(inv[1]["newState"].is_string());
   assert_ne!(inv[1]["oldState"], inv[1]["newState"], "state must bump");

   // Verify the DB reflects the delta.
   let flags_json = pool
      .get()
      .await
      .unwrap()
      .query_one(
         "SELECT flags_json FROM messages WHERE account_id = 'acctA' AND msgid = 'm1'",
         &[],
      )
      .await
      .unwrap()
      .get::<_, String>(0);
   let flags = serde_json::from_str::<BTreeSet<String>>(&flags_json).unwrap();
   assert!(flags.contains("$seen"), "expected $seen, got {flags:?}");
   assert!(
      !flags.contains("$flagged"),
      "expected no $flagged, got {flags:?}"
   );
}

/// Unsupported create fails with `forbidden`. Destroy without a sync task
/// surfaces `serverUnavailable` — we can't talk to IMAP, so we can't delete.
#[tokio::test]
async fn email_set_create_validates_mailbox_destroy_reports_unavailable() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/set",
           {
               "accountId": "acctA",
               "create": {"new1": {"mailboxIds": {}, "subject": "hi"}},
               "destroy": ["old1"]
           },
           "c0"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   // Empty mailboxIds: create is implemented but requires exactly one
   // target mailbox.
   assert_eq!(
      value["methodResponses"][0][1]["notCreated"]["new1"]["type"],
      "invalidProperties"
   );
   assert_eq!(
      value["methodResponses"][0][1]["notCreated"]["new1"]["properties"][0],
      "mailboxIds"
   );
   assert_eq!(
      value["methodResponses"][0][1]["notDestroyed"]["old1"]["type"],
      "serverUnavailable",
   );
}

/// Mailbox/query with `hasAnyRole: true` returns only role-bearing
/// rows; sort by name works; position/limit paginate cleanly.
#[tokio::test]
async fn mailbox_query_filters_and_sorts() {
   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb1", "acctA", "Inbox", None, Some("inbox"), 0)
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb2", "acctA", "Archive", None, Some("archive"), 1)
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb3", "acctA", "Personal", None, None, 2)
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb4", "acctA", "Projects", None, None, 3)
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb5", "acctA", "Alpha", Some("mb4"), None, 0)
      .await
      .unwrap();
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   // hasAnyRole: true → only Inbox + Archive (not Personal).
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Mailbox/query",
           {
               "accountId": "acctA",
               "filter": {"hasAnyRole": true},
               "sort": [{"property": "name", "isAscending": true}]
           },
           "c0"
       ]]
   });
   let resp = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   let ids = value["methodResponses"][0][1]["ids"]
      .as_array()
      .unwrap()
      .iter()
      .map(|x| x.as_str().unwrap().to_owned())
      .collect::<Vec<String>>();
   // Archive < Inbox alphabetically.
   assert_eq!(ids, vec!["mb2", "mb1"]);

   // Filter on exact role.
   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Mailbox/query",
           {"accountId": "acctA", "filter": {"role": "inbox"}},
           "c0"
       ]]
   });
   let resp = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   let ids = value["methodResponses"][0][1]["ids"]
      .as_array()
      .unwrap()
      .iter()
      .map(|x| x.as_str().unwrap().to_owned())
      .collect::<Vec<String>>();
   assert_eq!(ids, vec!["mb1"]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [
           ["Mailbox/query", {
               "accountId": "acctA",
               "sort": [{"property": "name"}],
               "sortAsTree": true
           }, "tree"],
           ["Mailbox/query", {
               "accountId": "acctA",
               "filter": {"name": "Alpha"}
           }, "flat-filter"],
           ["Mailbox/query", {
               "accountId": "acctA",
               "filter": {"name": "Alpha"},
               "filterAsTree": true
           }, "tree-filter"],
           ["Mailbox/query", {
               "accountId": "acctA",
               "filter": {
                   "operator": "OR",
                   "conditions": [{"role": "inbox"}, {"name": "Alpha"}]
               },
               "sort": [{"property": "name"}]
           }, "operator-filter"]
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   let responses = value["methodResponses"].as_array().unwrap();
   assert_eq!(
      responses[0][1]["ids"],
      serde_json::json!(["mb2", "mb1", "mb3", "mb4", "mb5"]),
      "a child must remain immediately after its parent"
   );
   assert_eq!(responses[1][1]["ids"], serde_json::json!(["mb5"]));
   assert_eq!(responses[2][1]["ids"], serde_json::json!([]));
   assert_eq!(responses[3][1]["ids"], serde_json::json!(["mb5", "mb1"]));
}

/// Query deltas require old result snapshots; returning a fabricated delta
/// from current rows would lose expungements and assign incorrect indices. An
/// unchanged query state can still return the exact empty delta.
#[tokio::test]
async fn email_query_changes_requires_snapshot_history() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let mk = |msgid: &str| {
      MessageEnvelope {
         msgid:              msgid.into(),
         thrid:              msgid.into(),
         flags:              vec![],
         received_at:        Utc::now(),
         sent_at:            None,
         size:               1,
         from:               None,
         to:                 None,
         cc:                 None,
         bcc:                None,
         reply_to:           None,
         subject:            Some("s".into()),
         preview:            None,
         has_attachment:     false,
         message_id_header:  None,
         in_reply_to_header: None,
         references_header:  None,
      }
   };
   // Seed two messages; capture the email_modseq after each seed.
   db::upsert_message(&pool, "acctA", &mk("m1")).await.unwrap();
   let cur1 = db::get_state(&pool, "acctA").await.unwrap().email_modseq;
   db::upsert_message(&pool, "acctA", &mk("m2")).await.unwrap();
   let cur2 = db::get_state(&pool, "acctA").await.unwrap().email_modseq;

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/queryChanges",
           {"accountId": "acctA", "sinceQueryState": cur1.to_string()},
           "c0"
       ]]
   });
   let resp = router
      .clone()
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   assert_eq!(
      value["methodResponses"][0][0].as_str(),
      Some("error"),
      "body: {value}"
   );
   assert_eq!(
      value["methodResponses"][0][1]["type"].as_str(),
      Some("cannotCalculateChanges")
   );

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [[
           "Email/queryChanges",
           {"accountId": "acctA", "sinceQueryState": cur2.to_string()},
           "c1"
       ]]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_s, value) = read_body(resp).await;
   assert_eq!(
      value["methodResponses"][0][0].as_str(),
      Some("Email/queryChanges"),
      "body: {value}"
   );
   assert_eq!(
      value["methodResponses"][0][1]["oldQueryState"],
      cur2.to_string()
   );
   assert_eq!(
      value["methodResponses"][0][1]["newQueryState"],
      cur2.to_string()
   );
   assert_eq!(
      value["methodResponses"][0][1]["removed"],
      serde_json::json!([])
   );
   assert_eq!(
      value["methodResponses"][0][1]["added"],
      serde_json::json!([])
   );
}

/// Whole-message blob download — GET
/// /download/{acct}/blob-{msgid}/{name} streams the RFC 5322 bytes.
#[tokio::test]
async fn download_serves_whole_message() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "m1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     false,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();

   // Seed raw_messages with a canned RFC 5322 body so the handler finds
   // bytes without needing a live IMAP session.
   let raw = b"From: a@x.com\r\nTo: b@x.com\r\nSubject: hi\r\n\r\nHello world\r\n";
   exec(
      &pool,
      "INSERT INTO raw_messages (account_id, msgid, headers_json, body_values_json, \
       attachments_json, raw_rfc822, fetched_at) VALUES ($1, $2, '{}', '{}', '[]', $3, \
       EXTRACT(EPOCH FROM now())::bigint)",
      &[&"acctA", &"m1", &&raw[..]],
   )
   .await;

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let resp = router
      .oneshot(
         Request::builder()
            .uri("/download/acctA/blob-m1/message.eml")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   let status = resp.status();
   let content_type = resp
      .headers()
      .get("content-type")
      .and_then(|header| header.to_str().ok())
      .map(ToOwned::to_owned);
   let bytes = resp.into_body().collect().await.unwrap().to_bytes();
   assert_eq!(status, StatusCode::OK);
   assert_eq!(content_type.as_deref(), Some("message/rfc822"));
   assert_eq!(bytes.as_ref(), raw);
}

/// Part download returns an attachment with its own content-type
/// and the original filename echoed in Content-Disposition.
#[tokio::test]
async fn download_serves_mime_part() {
   use chrono::Utc;
   use imap_sync::db::MessageEnvelope;

   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   let env = MessageEnvelope {
      msgid:              "m1".into(),
      thrid:              "m1".into(),
      flags:              vec![],
      received_at:        Utc::now(),
      sent_at:            None,
      size:               1,
      from:               None,
      to:                 None,
      cc:                 None,
      bcc:                None,
      reply_to:           None,
      subject:            Some("hi".into()),
      preview:            None,
      has_attachment:     true,
      message_id_header:  None,
      in_reply_to_header: None,
      references_header:  None,
   };
   db::upsert_message(&pool, "acctA", &env).await.unwrap();

   // Minimal multipart/mixed with one text part + one PDF-ish attachment.
   let raw = b"From: a@x.com\r\n\
To: b@x.com\r\n\
Subject: hi\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/mixed; boundary=\"BOUND\"\r\n\
\r\n\
--BOUND\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
hello world\r\n\
--BOUND\r\n\
Content-Type: application/pdf\r\n\
Content-Disposition: attachment; filename=\"report.pdf\"\r\n\
\r\n\
%PDF-1.4 fake\r\n\
--BOUND--\r\n";

   exec(
      &pool,
      "INSERT INTO raw_messages (account_id, msgid, headers_json, body_values_json, \
       attachments_json, raw_rfc822, fetched_at) VALUES ($1, $2, '{}', '{}', '[]', $3, \
       EXTRACT(EPOCH FROM now())::bigint)",
      &[&"acctA", &"m1", &&raw[..]],
   )
   .await;

   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let resp = router
      .oneshot(
         Request::builder()
            .uri("/download/acctA/blob-m1~att-0/report.pdf")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   let status = resp.status();
   let content_type = resp
      .headers()
      .get("content-type")
      .and_then(|header| header.to_str().ok())
      .map(ToOwned::to_owned);
   let content_disposition = resp
      .headers()
      .get("content-disposition")
      .and_then(|header| header.to_str().ok())
      .map(ToOwned::to_owned);
   let bytes = resp.into_body().collect().await.unwrap().to_bytes();
   assert_eq!(status, StatusCode::OK);
   assert_eq!(content_type.as_deref(), Some("application/pdf"));
   assert!(
      content_disposition
         .as_deref()
         .is_some_and(|disposition| disposition.contains("report.pdf")),
      "expected filename in Content-Disposition, got {content_disposition:?}"
   );
   assert_eq!(bytes.as_ref(), b"%PDF-1.4 fake");
}

/// acctA's bearer token cannot download
/// an acctB blob, even if the blob id exists. Mirrors the same contract the
/// /api handler enforces.
#[tokio::test]
async fn download_cross_account_is_unauthorized() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let resp = router
      .oneshot(
         Request::builder()
            .uri("/download/acctB/blob-m1/msg.eml")
            .header("authorization", format!("Bearer {token_a}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

/// A malformed blob id is a clean 404, not an internal error.
#[tokio::test]
async fn download_bad_blob_id_is_404() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let resp = router
      .oneshot(
         Request::builder()
            .uri("/download/acctA/m-nope/name.eml")
            .header("authorization", format!("Bearer {token_a}"))
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

/// Result references (RFC 8620 §3.7). A single batch with
/// `Mailbox/query → Mailbox/get(#ids: /ids)` should resolve the ids from the
/// earlier response, so real clients can do one-round-trip listings.
#[tokio::test]
async fn api_resolves_result_references() {
   let Some(pool) = setup_pool().await else {
      return;
   };
   db::upsert_account(&pool, "acctA", "a@x.com", ProviderKind::Imap, "A", b"h")
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb1", "acctA", "Inbox", None, Some("inbox"), 0)
      .await
      .unwrap();
   db::upsert_mailbox(&pool, "mb2", "acctA", "Sent", None, Some("sent"), 1)
      .await
      .unwrap();
   let token = "bearer-A-abcdef0123456789";
   let state = AppState::new(
      pool,
      vec![AccountInfo::from_bearer_token(
         "acctA", "a@x.com", "A", token,
      )],
      "http://test.invalid".to_owned(),
      HashMap::new(),
   );
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [
           ["Mailbox/query", {"accountId": "acctA"}, "c0"],
           ["Mailbox/get", {
               "accountId": "acctA",
               "#ids": {"resultOf": "c0", "name": "Mailbox/query", "path": "/ids"}
           }, "c1"],
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (status, value) = read_body(resp).await;
   assert_eq!(status, StatusCode::OK);

   // Both calls returned successfully.
   let r0 = &value["methodResponses"][0];
   assert_eq!(r0[0].as_str(), Some("Mailbox/query"), "call0: {value}");
   assert_eq!(r0[2].as_str(), Some("c0"));

   let r1 = &value["methodResponses"][1];
   assert_eq!(r1[0].as_str(), Some("Mailbox/get"), "call1: {value}");
   assert_eq!(r1[2].as_str(), Some("c1"));

   // The Mailbox/get response should contain both mailboxes — its `ids` arg
   // was substituted from the query result.
   let list = r1[1]["list"].as_array().expect("list present");
   let ids = list
      .iter()
      .map(|mailbox| mailbox["id"].as_str().unwrap().to_owned())
      .collect::<BTreeSet<String>>();
   assert_eq!(
      ids,
      ["mb1", "mb2"]
         .iter()
         .map(ToString::to_string)
         .collect::<BTreeSet<_>>(),
      "result reference should have substituted both mailbox ids"
   );
}

/// Unknown `resultOf` surfaces as
/// `invalidResultReference` on the *referencing* call, not a silent empty
/// result. The first call must still succeed.
#[tokio::test]
async fn api_bad_result_reference_is_method_error() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
       "methodCalls": [
           ["Mailbox/query", {"accountId": "acctA"}, "c0"],
           ["Mailbox/get", {
               "accountId": "acctA",
               "#ids": {"resultOf": "does-not-exist", "name": "Mailbox/query", "path": "/ids"}
           }, "c1"],
       ]
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;

   assert_eq!(
      value["methodResponses"][0][0].as_str(),
      Some("Mailbox/query"),
      "first call should still succeed: {value}"
   );
   let err = &value["methodResponses"][1];
   assert_eq!(err[0].as_str(), Some("error"));
   assert_eq!(err[1]["type"].as_str(), Some("invalidResultReference"));
   assert_eq!(err[2].as_str(), Some("c1"));
}

/// Client-supplied `createdIds` should round-trip unchanged on the
/// response when no Set method has added new entries. This preserves the
/// contract clients rely on — the map persists across batches they send.
#[tokio::test]
async fn api_echoes_created_ids() {
   let Some((state, token_a, _token_b)) = two_accounts().await else {
      return;
   };
   let router = build_router(state, vec!["http://example.test".into()]);

   let body = serde_json::json!({
       "using": ["urn:ietf:params:jmap:core"],
       "methodCalls": [["Core/echo", {"hi": "there"}, "c0"]],
       "createdIds": {"k1": "acctA"}
   });
   let resp = router
      .oneshot(
         Request::builder()
            .method("POST")
            .uri("/api")
            .header("authorization", format!("Bearer {token_a}"))
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
      )
      .await
      .unwrap();
   let (_status, value) = read_body(resp).await;
   assert_eq!(value["createdIds"]["k1"].as_str(), Some("acctA"));
}

#[tokio::test]
async fn cors_preflight_allowed_origin() {
   let Some((state, _a, _b)) = two_accounts().await else {
      return;
   };
   let origin = "http://webmail.example";
   let router = build_router(state, vec![origin.to_owned()]);

   let resp = router
      .oneshot(
         Request::builder()
            .method("OPTIONS")
            .uri("/api")
            .header("origin", origin)
            .header("access-control-request-method", "POST")
            .header(
               "access-control-request-headers",
               "authorization,content-type",
            )
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();
   assert!(
      resp.status().is_success() || resp.status() == StatusCode::NO_CONTENT,
      "preflight should succeed, got {}",
      resp.status()
   );
   let allow_origin = resp
      .headers()
      .get("access-control-allow-origin")
      .and_then(|header| header.to_str().ok());
   assert_eq!(allow_origin, Some(origin));
}

#[tokio::test]
async fn cors_wildcard_allows_the_request_origin() {
   let Some((state, _a, _b)) = two_accounts().await else {
      return;
   };
   let origin = "https://arbitrary.example";
   let router = build_router(state, vec!["*".to_owned()]);
   let response = router
      .oneshot(
         Request::builder()
            .method("OPTIONS")
            .uri("/api")
            .header("origin", origin)
            .header("access-control-request-method", "POST")
            .body(Body::empty())
            .unwrap(),
      )
      .await
      .unwrap();

   assert!(response.status().is_success());
   assert_eq!(
      response
         .headers()
         .get("access-control-allow-origin")
         .and_then(|value| value.to_str().ok()),
      Some("*")
   );
}
