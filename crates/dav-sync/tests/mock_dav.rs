//! Integration tests against an in-process mock CalDAV/CardDAV server.
//!
//! Every test asserts the exact persisted DB state (rows, etags, modseqs,
//! tombstones, sync tokens), not just that calls returned Ok. DB-backed
//! tests skip without `JMAPPER_TEST_DB_URL` and fail loudly under
//! `JMAPPER_REQUIRE_DB_TESTS=1` (the predeploy gate) — same contract as the
//! mail suite.

use std::{
   collections::{
      BTreeMap,
      BTreeSet,
   },
   sync::{
      Arc,
      Mutex,
   },
};

use axum::{
   body::Bytes,
   extract::State,
   http::{
      HeaderMap,
      Method,
      StatusCode,
      Uri,
   },
   response::{
      IntoResponse as _,
      Response,
   },
};
use dav_sync::{
   convert::contact,
   engine::{
      self,
      SyncStats,
   },
   error::DavError,
   service,
   store::{
      self,
      DavKind,
   },
};
use deadpool_postgres::Pool;
use imap_sync::testkit;
use pretty_assertions::assert_eq;
use tokio::net::TcpListener;

// ============================ mock server ============================

#[derive(Debug, Clone)]
struct MockResource {
   data: String,
   etag: u64,
   /// Server revision at which this resource last changed.
   rev:  u64,
}

#[derive(Debug, Default)]
struct MockDav {
   /// 'caldav' or 'carddav' behavior.
   carddav:            bool,
   /// collection href → display name.
   collections:        BTreeMap<String, String>,
   /// resource href → body.
   resources:          BTreeMap<String, MockResource>,
   /// resource href → revision at which it was deleted.
   deleted:            BTreeMap<String, u64>,
   /// Monotonic revision; the sync token is its decimal form.
   rev:                u64,
   /// When set, any non-empty sync token is rejected with the RFC 6578
   /// valid-sync-token precondition error.
   invalidate_tokens:  bool,
   /// When set, sync-collection REPORTs fail with a plain 403 (server
   /// that advertises the report but refuses it).
   refuse_sync:        bool,
   /// When set, the next multiget/propfind answers with garbage XML.
   serve_garbage:      bool,
   /// Simulate a broken server returning a partial multiget response.
   omit_multiget_href: Option<String>,
   /// Simulate server-side canonicalization despite returning an `ETag`.
   rewrite_next_put:   Option<String>,
   /// Every request seen: (method, path, `if_match`, `if_none_match`).
   requests:           Vec<(String, String, Option<String>, Option<String>)>,
}

impl MockDav {
   fn put(&mut self, href: &str, data: &str) {
      self.rev += 1;
      let etag = self.rev;
      self.resources.insert(href.to_owned(), MockResource {
         data: data.to_owned(),
         etag,
         rev: self.rev,
      });
      self.deleted.remove(href);
   }

   fn delete(&mut self, href: &str) {
      self.rev += 1;
      self.resources.remove(href);
      self.deleted.insert(href.to_owned(), self.rev);
   }

   fn etag_of(&self, href: &str) -> String {
      format!("\"{}\"", self.resources[href].etag)
   }
}

type Shared = Arc<Mutex<MockDav>>;

/// CR must go out as a character reference: bare CRLF in element content
/// is normalized to LF by any conforming XML parser (XML 1.0 §2.11), which
/// would corrupt the iCalendar/vCard payload's line endings.
fn xml_escape(text: &str) -> String {
   text
      .replace('&', "&amp;")
      .replace('<', "&lt;")
      .replace('>', "&gt;")
      .replace('\r', "&#13;")
}

fn multistatus(inner: &str, sync_token: Option<&str>) -> Response {
   let token = sync_token
      .map(|tok| format!("<D:sync-token>{tok}</D:sync-token>"))
      .unwrap_or_default();
   let body = format!(
      r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:A="urn:ietf:params:xml:ns:carddav">
{inner}{token}</D:multistatus>"#
   );
   (
      StatusCode::MULTI_STATUS,
      [("Content-Type", "application/xml; charset=utf-8")],
      body,
   )
      .into_response()
}

async fn handle(
   State(state): State<Shared>,
   method: Method,
   uri: Uri,
   headers: HeaderMap,
   body: Bytes,
) -> Response {
   let path = uri.path().to_owned();
   let body = String::from_utf8_lossy(&body).into_owned();
   let hdr = |name| {
      headers
         .get(name)
         .and_then(|value| value.to_str().ok())
         .map(str::to_owned)
   };
   let mut dav = state.lock().expect("mock lock");
   dav.requests.push((
      method.to_string(),
      path.clone(),
      hdr("If-Match"),
      hdr("If-None-Match"),
   ));

   let response = match method.as_str() {
      "PROPFIND" => propfind(&mut dav, &path, &body),
      "REPORT" => report(&mut dav, &path, &body),
      "GET" => {
         dav.resources.get(&path).map_or_else(
            || StatusCode::NOT_FOUND.into_response(),
            |res| {
               (
                  StatusCode::OK,
                  [("ETag", format!("\"{}\"", res.etag))],
                  res.data.clone(),
               )
                  .into_response()
            },
         )
      },
      "PUT" => {
         let exists = dav.resources.contains_key(&path);
         if let Some(im) = hdr("If-Match")
            && (!exists || dav.etag_of(&path) != im)
         {
            return StatusCode::PRECONDITION_FAILED.into_response();
         }
         if hdr("If-None-Match").as_deref() == Some("*") && exists {
            return StatusCode::PRECONDITION_FAILED.into_response();
         }
         let stored = dav.rewrite_next_put.take().unwrap_or(body);
         dav.put(&path, &stored);
         let etag = dav.etag_of(&path);
         (StatusCode::CREATED, [("ETag", etag)], "").into_response()
      },
      "DELETE" => {
         if let Some(im) = hdr("If-Match")
            && (!dav.resources.contains_key(&path) || dav.etag_of(&path) != im)
         {
            return StatusCode::PRECONDITION_FAILED.into_response();
         }
         if dav.resources.contains_key(&path) {
            dav.delete(&path);
            StatusCode::NO_CONTENT.into_response()
         } else {
            StatusCode::NOT_FOUND.into_response()
         }
      },
      "MOVE" => {
         let Some(dest) = hdr("Destination") else {
            return StatusCode::BAD_REQUEST.into_response();
         };
         let dest_path = url::Url::parse(&dest)
            .map(|url| url.path().to_owned())
            .unwrap_or(dest);
         let Some(res) = dav.resources.get(&path).cloned() else {
            return StatusCode::NOT_FOUND.into_response();
         };
         dav.delete(&path);
         dav.put(&dest_path, &res.data);
         StatusCode::CREATED.into_response()
      },
      _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
   };
   drop(dav);
   response
}

fn propfind(dav: &mut MockDav, path: &str, body: &str) -> Response {
   use std::fmt::Write as _;

   if dav.serve_garbage {
      dav.serve_garbage = false;
      return (
         StatusCode::MULTI_STATUS,
         [("Content-Type", "application/xml")],
         "<not-multistatus>this is garbage</not-multistatus>",
      )
         .into_response();
   }
   let home = if dav.carddav { "/card/" } else { "/cal/" };
   if body.contains("current-user-principal") {
      return multistatus(
         &format!(
            "<D:response><D:href>{path}</D:href><D:propstat><D:prop>
<D:current-user-principal><D:href>/principals/user/</D:href></D:current-user-principal>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
"
         ),
         None,
      );
   }
   if body.contains("calendar-home-set") || body.contains("addressbook-home-set") {
      let (ns, name) = if dav.carddav {
         ("A", "addressbook-home-set")
      } else {
         ("C", "calendar-home-set")
      };
      return multistatus(
         &format!(
            "<D:response><D:href>{path}</D:href><D:propstat><D:prop>
<{ns}:{name}><D:href>{home}</D:href></{ns}:{name}>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
"
         ),
         None,
      );
   }
   // Depth:1 listing of the home set (collections) or of one collection
   // (the ETag fallback).
   if path == home {
      let rtype = if dav.carddav {
         "<A:addressbook/>"
      } else {
         "<C:calendar/>"
      };
      let mut inner = format!(
         "<D:response><D:href>{home}</D:href><D:propstat><D:prop>
<D:resourcetype><D:collection/></D:resourcetype>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
"
      );
      // Advertised even when refused: the engine must survive the 403 below.
      let sync_report =
         "<D:supported-report><D:report><D:sync-collection/></D:report></D:supported-report>";
      for (href, name) in &dav.collections {
         write!(
            inner,
            "<D:response><D:href>{href}</D:href><D:propstat><D:prop>
<D:resourcetype><D:collection/>{rtype}</D:resourcetype>
<D:displayname>{}</D:displayname>
<D:supported-report-set>{sync_report}</D:supported-report-set>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
",
            xml_escape(name)
         )
         .unwrap();
      }
      return multistatus(&inner, None);
   }
   // ETag listing inside a collection.
   let mut inner = format!(
      "<D:response><D:href>{path}</D:href><D:propstat><D:prop>
<D:resourcetype><D:collection/></D:resourcetype>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
"
   );
   for (href, res) in dav.resources.range(path.to_owned()..) {
      if !href.starts_with(path) {
         break;
      }
      write!(
         inner,
         r#"<D:response><D:href>{href}</D:href><D:propstat><D:prop>
<D:resourcetype/><D:getetag>"{}"</D:getetag>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
"#,
         res.etag
      )
      .unwrap();
   }
   multistatus(&inner, None)
}

fn report(dav: &mut MockDav, path: &str, body: &str) -> Response {
   use std::fmt::Write as _;

   if body.contains("sync-collection") {
      if dav.refuse_sync {
         return StatusCode::FORBIDDEN.into_response();
      }
      let since = body
         .split("<D:sync-token>")
         .nth(1)
         .and_then(|rest| rest.split("</D:sync-token>").next())
         .and_then(|tok| tok.trim().parse::<u64>().ok());
      if since.is_some() && dav.invalidate_tokens {
         let body = r#"<?xml version="1.0" encoding="utf-8"?>
<D:error xmlns:D="DAV:"><D:valid-sync-token/></D:error>"#;
         return (
            StatusCode::FORBIDDEN,
            [("Content-Type", "application/xml")],
            body,
         )
            .into_response();
      }
      let since = since.unwrap_or(0);
      let mut inner = String::new();
      for (href, res) in &dav.resources {
         if href.starts_with(path) && res.rev > since {
            write!(
               inner,
               r#"<D:response><D:href>{href}</D:href><D:propstat><D:prop>
<D:getetag>"{}"</D:getetag>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
"#,
               res.etag
            )
            .unwrap();
         }
      }
      // Removals only make sense for incremental rounds.
      if since > 0 {
         for (href, rev) in &dav.deleted {
            if href.starts_with(path) && *rev > since {
               writeln!(
                  inner,
                  "<D:response><D:href>{href}</D:href><D:status>HTTP/1.1 404 Not \
                   Found</D:status></D:response>"
               )
               .unwrap();
            }
         }
      }
      return multistatus(&inner, Some(&dav.rev.to_string()));
   }

   // multiget
   if dav.serve_garbage {
      dav.serve_garbage = false;
      return (
         StatusCode::MULTI_STATUS,
         [("Content-Type", "application/xml")],
         "<<<< definitely not xml",
      )
         .into_response();
   }
   let data_tag = if dav.carddav {
      ("A", "address-data")
   } else {
      ("C", "calendar-data")
   };
   let mut inner = String::new();
   for line in body.lines() {
      let Some(href) = line
         .trim()
         .strip_prefix("<D:href>")
         .and_then(|rest| rest.strip_suffix("</D:href>"))
      else {
         continue;
      };
      if dav.omit_multiget_href.as_deref() == Some(href) {
         continue;
      }
      match dav.resources.get(href) {
         Some(res) => {
            write!(
               inner,
               r#"<D:response><D:href>{href}</D:href><D:propstat><D:prop>
<D:getetag>"{}"</D:getetag>
<{}:{}>{}</{}:{}>
</D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>
"#,
               res.etag,
               data_tag.0,
               data_tag.1,
               xml_escape(&res.data),
               data_tag.0,
               data_tag.1,
            )
            .unwrap();
         },
         None => {
            writeln!(
               inner,
               "<D:response><D:href>{href}</D:href><D:status>HTTP/1.1 404 Not \
                Found</D:status></D:response>"
            )
            .unwrap();
         },
      }
   }
   multistatus(&inner, None)
}

async fn start_server(state: Shared) -> String {
   let app = axum::Router::new().fallback(handle).with_state(state);
   let listener = TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind mock server");
   let addr = listener.local_addr().expect("local addr");
   tokio::spawn(async move {
      axum::serve(listener, app).await.expect("mock server");
   });
   format!("http://{addr}/")
}

// ============================ fixtures ============================

const ACCOUNT: &str = "acct1";

fn ical(uid: &str, summary: &str) -> String {
   format!(
      "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Mock//EN\r\nBEGIN:VEVENT\r\nUID:{uid}\r\\
       nDTSTAMP:20260401T120000Z\r\nDTSTART;TZID=Europe/Berlin:20260406T100000\r\nDURATION:PT1H\r\\
       nSUMMARY:{summary}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
   )
}

fn vcard(uid: &str, name: &str) -> String {
   format!("BEGIN:VCARD\r\nVERSION:4.0\r\nUID:{uid}\r\nFN:{name}\r\nEND:VCARD\r\n")
}

async fn setup(carddav: bool) -> Option<(Pool, Shared, store::DavEndpoint)> {
   let pool = testkit::test_pool().await?;
   let client = pool.get().await.expect("pool client");
   client
      .execute(
         "INSERT INTO accounts (id, email, provider, display_name, bearer_token_hash, created_at)
         VALUES ($1, 'u@example.com', 'imap', 'U', '\\x00'::bytea, 0)",
         &[&ACCOUNT],
      )
      .await
      .expect("insert account");

   let state = Arc::new(Mutex::new(MockDav {
      carddav,
      ..MockDav::default()
   }));
   let base_url = start_server(Arc::clone(&state)).await;

   let kind = if carddav {
      DavKind::CardDav
   } else {
      DavKind::CalDav
   };
   store::upsert_endpoint(
      &client,
      ACCOUNT,
      kind,
      &base_url,
      "basic",
      Some("u"),
      Some("pw"),
   )
   .await
   .expect("upsert endpoint");
   let endpoint = store::get_endpoint(&client, ACCOUNT, kind)
      .await
      .expect("get endpoint")
      .expect("endpoint exists");
   Some((pool, state, endpoint))
}

async fn run_sync(pool: &Pool, endpoint: &store::DavEndpoint) -> SyncStats {
   // Rows change between runs, so refresh the endpoint (discovery cache).
   let client = pool.get().await.expect("pool client");
   let endpoint = store::get_endpoint(&client, &endpoint.account_id, endpoint.kind)
      .await
      .expect("get endpoint")
      .expect("endpoint exists");
   drop(client);
   engine::sync_endpoint(pool, &endpoint, false, 1_000)
      .await
      .expect("sync succeeds")
}

// ============================ tests ============================

#[tokio::test(flavor = "multi_thread")]
async fn initial_sync_persists_exact_state() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      let e1 = ical("e1@x", "One");
      let e2 = ical("e2@x", "Two");
      dav.put("/cal/work/e1.ics", &e1);
      dav.put("/cal/work/e2.ics", &e2);
   }

   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.collections_created, 1);
   assert_eq!(stats.resources_created, 2);
   assert_eq!(stats.resources_updated, 0);
   assert_eq!(stats.resources_removed, 0);
   assert!(stats.parse_failures.is_empty());

   let client = pool.get().await.unwrap();
   let cols = store::list_collections(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap();
   assert_eq!(cols.len(), 1);
   let col = &cols[0];
   assert_eq!(col.href, "/cal/work/");
   assert_eq!(col.name, "Work");
   assert_eq!(col.id, store::id_for_href(DavKind::CalDav, "/cal/work/"));
   assert!(col.supports_sync);
   assert!(col.sync_token.is_some(), "token stored after full round");
   assert!(!col.destroyed);

   let resources = store::list_resources(&client, ACCOUNT, &col.id)
      .await
      .unwrap();
   assert_eq!(resources.len(), 2);
   let e1 = resources
      .iter()
      .find(|res| res.href == "/cal/work/e1.ics")
      .expect("e1 row");
   assert_eq!(e1.id, store::id_for_uid(DavKind::CalDav, "e1@x"));
   assert_eq!(e1.uid, "e1@x");
   assert_eq!(e1.raw, ical("e1@x", "One"));
   assert_eq!(e1.etag.as_deref(), Some("\"1\""));
   assert!(!e1.destroyed);
   // The normalized projection is real JSCalendar, not a placeholder.
   let json = serde_json::from_str::<serde_json::Value>(&e1.json).unwrap();
   assert_eq!(json["@type"], "Event");
   assert_eq!(json["title"], "One");
   assert_eq!(json["start"], "2026-04-06T10:00:00");
   assert_eq!(json["timeZone"], "Europe/Berlin");

   let st = store::get_state(&client, ACCOUNT).await.unwrap();
   assert_eq!(st.calendar_modseq, 1);
   assert_eq!(st.calendar_event_modseq, 1);
   assert_eq!(st.addressbook_modseq, 0);
   assert_eq!(st.contact_card_modseq, 0);

   // Endpoint health recorded.
   let ep = store::get_endpoint(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap()
      .unwrap();
   assert_eq!(ep.last_sync_at, Some(1_000));
   assert_eq!(ep.last_sync_error, None);
   assert_eq!(ep.principal_href.as_deref(), Some("/principals/user/"));
   assert_eq!(ep.home_href.as_deref(), Some("/cal/"));
}

#[tokio::test(flavor = "multi_thread")]
async fn incremental_sync_applies_changes_and_tombstones() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One"));
      dav.put("/cal/work/e2.ics", &ical("e2@x", "Two"));
   }
   run_sync(&pool, &endpoint).await;

   // Idempotence: an immediate re-run changes nothing.
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.resources_created + stats.resources_updated, 0);
   let client = pool.get().await.unwrap();
   let st_before = store::get_state(&client, ACCOUNT).await.unwrap();
   drop(client);

   {
      let mut dav = state.lock().unwrap();
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One updated"));
      dav.delete("/cal/work/e2.ics");
      dav.put("/cal/work/e3.ics", &ical("e3@x", "Three"));
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.resources_created, 1);
   assert_eq!(stats.resources_updated, 1);
   assert_eq!(stats.resources_removed, 1);

   let client = pool.get().await.unwrap();
   let col_id = store::id_for_href(DavKind::CalDav, "/cal/work/");
   let live = store::list_resources(&client, ACCOUNT, &col_id)
      .await
      .unwrap();
   assert_eq!(
      live
         .iter()
         .map(|res| res.href.as_str())
         .collect::<BTreeSet<_>>(),
      ["/cal/work/e1.ics", "/cal/work/e3.ics"]
         .into_iter()
         .collect()
   );
   let e1 = live
      .iter()
      .find(|res| res.href.ends_with("e1.ics"))
      .unwrap();
   let json = serde_json::from_str::<serde_json::Value>(&e1.json).unwrap();
   assert_eq!(json["title"], "One updated");

   // Tombstone visible via changed-since, gone from live listing.
   let st = store::get_state(&client, ACCOUNT).await.unwrap();
   assert_eq!(
      st.calendar_event_modseq,
      st_before.calendar_event_modseq + 1
   );
   let changed = store::resources_changed_since(
      &client,
      ACCOUNT,
      DavKind::CalDav,
      st_before.calendar_event_modseq,
   )
   .await
   .unwrap();
   let e2 = changed
      .iter()
      .find(|res| res.href.ends_with("e2.ics"))
      .expect("tombstone row");
   assert!(e2.destroyed);
   assert_eq!(e2.modseq, st.calendar_event_modseq);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalidated_sync_token_falls_back_to_full_round() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One"));
   }
   run_sync(&pool, &endpoint).await;

   {
      let mut dav = state.lock().unwrap();
      dav.invalidate_tokens = true;
      dav.put("/cal/work/e2.ics", &ical("e2@x", "Two"));
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.resources_created, 1, "full round found the new one");
   assert_eq!(stats.resources_updated, 0, "unchanged etag not re-applied");

   let client = pool.get().await.unwrap();
   let col_id = store::id_for_href(DavKind::CalDav, "/cal/work/");
   let live = store::list_resources(&client, ACCOUNT, &col_id)
      .await
      .unwrap();
   assert_eq!(live.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn refused_sync_collection_degrades_to_etag_listing() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.refuse_sync = true;
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One"));
      dav.put("/cal/work/e2.ics", &ical("e2@x", "Two"));
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.etag_fallbacks, 1);
   assert_eq!(stats.resources_created, 2);

   // Fallback also detects updates and deletions.
   {
      let mut dav = state.lock().unwrap();
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One v2"));
      dav.delete("/cal/work/e2.ics");
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.resources_updated, 1);
   assert_eq!(stats.resources_removed, 1);

   let client = pool.get().await.unwrap();
   let col = store::get_collection(
      &client,
      ACCOUNT,
      &store::id_for_href(DavKind::CalDav, "/cal/work/"),
   )
   .await
   .unwrap()
   .unwrap();
   assert_eq!(col.sync_token, None, "no token stored on the etag path");
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_xml_fails_sync_without_state_advance() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One"));
   }
   run_sync(&pool, &endpoint).await;
   let client = pool.get().await.unwrap();
   let st_before = store::get_state(&client, ACCOUNT).await.unwrap();
   let col_before = store::get_collection(
      &client,
      ACCOUNT,
      &store::id_for_href(DavKind::CalDav, "/cal/work/"),
   )
   .await
   .unwrap()
   .unwrap();
   let ep = store::get_endpoint(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap()
      .unwrap();
   drop(client);

   {
      let mut dav = state.lock().unwrap();
      dav.put("/cal/work/e2.ics", &ical("e2@x", "Two"));
      dav.serve_garbage = true;
   }
   let err = engine::sync_endpoint(&pool, &ep, false, 2_000)
      .await
      .expect_err("garbage XML must fail the round");
   assert!(matches!(err, DavError::Xml(_)), "got {err:?}");

   let client = pool.get().await.unwrap();
   let st_after = store::get_state(&client, ACCOUNT).await.unwrap();
   assert_eq!(st_before, st_after, "no modseq advance on failure");
   let col_after = store::get_collection(
      &client,
      ACCOUNT,
      &store::id_for_href(DavKind::CalDav, "/cal/work/"),
   )
   .await
   .unwrap()
   .unwrap();
   assert_eq!(
      col_before.sync_token, col_after.sync_token,
      "token untouched"
   );
   let ep = store::get_endpoint(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap()
      .unwrap();
   assert!(ep.last_sync_error.is_some(), "error recorded for operators");
   drop(client);

   // The retry (server healthy again) applies exactly the missed change.
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.resources_created, 1);
   let client = pool.get().await.unwrap();
   let ep = store::get_endpoint(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap()
      .unwrap();
   assert_eq!(ep.last_sync_error, None, "error cleared on success");
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_resource_is_reported_not_stored() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/good.ics", &ical("good@x", "Fine"));
      // No UID: conversion must reject it.
      dav.put(
         "/cal/work/bad.ics",
         "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nDTSTAMP:20260101T000000Z\r\nEND:VEVENT\r\nEND:\
          VCALENDAR\r\n",
      );
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.resources_created, 1);
   assert_eq!(stats.parse_failures.len(), 1);
   assert_eq!(stats.parse_failures[0].0, "/cal/work/bad.ics");

   let client = pool.get().await.unwrap();
   let live = store::list_resources(
      &client,
      ACCOUNT,
      &store::id_for_href(DavKind::CalDav, "/cal/work/"),
   )
   .await
   .unwrap();
   assert_eq!(live.len(), 1);
   assert_eq!(live[0].href, "/cal/work/good.ics");
}

#[tokio::test(flavor = "multi_thread")]
async fn malformed_update_tombstones_the_previously_valid_resource() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "Initially valid"));
   }
   run_sync(&pool, &endpoint).await;

   {
      let mut dav = state.lock().unwrap();
      dav.put(
         "/cal/work/e1.ics",
         "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n",
      );
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.parse_failures.len(), 1);
   assert_eq!(stats.resources_removed, 1);

   let client = pool.get().await.unwrap();
   let id = store::id_for_uid(DavKind::CalDav, "e1@x");
   let row = store::get_resource(&client, ACCOUNT, &id)
      .await
      .unwrap()
      .unwrap();
   assert!(row.destroyed, "stale valid data must no longer be exposed");
}

#[tokio::test(flavor = "multi_thread")]
async fn partial_multiget_does_not_advance_token_or_resource_state() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One"));
   }
   run_sync(&pool, &endpoint).await;

   let client = pool.get().await.unwrap();
   let col_id = store::id_for_href(DavKind::CalDav, "/cal/work/");
   let before_state = store::get_state(&client, ACCOUNT).await.unwrap();
   let before_token = store::get_collection(&client, ACCOUNT, &col_id)
      .await
      .unwrap()
      .unwrap()
      .sync_token;
   let endpoint = store::get_endpoint(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap()
      .unwrap();
   drop(client);

   {
      let mut dav = state.lock().unwrap();
      dav.put("/cal/work/e2.ics", &ical("e2@x", "Two"));
      dav.put("/cal/work/e3.ics", &ical("e3@x", "Three"));
      dav.omit_multiget_href = Some("/cal/work/e3.ics".to_owned());
   }
   let err = engine::sync_endpoint(&pool, &endpoint, false, 2_000)
      .await
      .expect_err("partial multiget must fail the round");
   assert!(err.to_string().contains("multiget omitted"), "got {err:?}");

   let client = pool.get().await.unwrap();
   assert_eq!(
      store::get_state(&client, ACCOUNT).await.unwrap(),
      before_state
   );
   assert_eq!(
      store::get_collection(&client, ACCOUNT, &col_id)
         .await
         .unwrap()
         .unwrap()
         .sync_token,
      before_token
   );
   assert_eq!(
      store::list_resources(&client, ACCOUNT, &col_id)
         .await
         .unwrap()
         .len(),
      1
   );
   drop(client);

   state.lock().unwrap().omit_multiget_href = None;
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.resources_created, 2, "retry applies the whole batch");
}

#[tokio::test(flavor = "multi_thread")]
async fn forced_resync_refetches_equal_etags_and_handles_uid_replacement() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.put("/cal/work/e1.ics", &ical("old@x", "Old backend"));
   }
   run_sync(&pool, &endpoint).await;

   // Simulate switching to a backend that happens to use the same href and
   // ETag but stores a different object. A forced resync must not trust the
   // coincidental ETag match.
   state
      .lock()
      .unwrap()
      .resources
      .get_mut("/cal/work/e1.ics")
      .unwrap()
      .data = ical("new@x", "New backend");

   let client = pool.get().await.unwrap();
   let endpoint = store::get_endpoint(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap()
      .unwrap();
   drop(client);
   let stats = engine::sync_endpoint(&pool, &endpoint, true, 2_000)
      .await
      .expect("forced sync succeeds");
   assert_eq!(stats.resources_removed, 1);
   assert_eq!(stats.resources_created, 1);

   let client = pool.get().await.unwrap();
   let old = store::get_resource(
      &client,
      ACCOUNT,
      &store::id_for_uid(DavKind::CalDav, "old@x"),
   )
   .await
   .unwrap()
   .unwrap();
   assert!(old.destroyed);
   let new = store::get_resource(
      &client,
      ACCOUNT,
      &store::id_for_uid(DavKind::CalDav, "new@x"),
   )
   .await
   .unwrap()
   .unwrap();
   assert_eq!(new.href, "/cal/work/e1.ics");
   assert!(!new.destroyed);
}

#[tokio::test(flavor = "multi_thread")]
async fn collection_removal_tombstones_collection_and_members() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.collections
         .insert("/cal/home/".to_owned(), "Home".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One"));
      dav.put("/cal/home/h1.ics", &ical("h1@x", "Haus"));
   }
   run_sync(&pool, &endpoint).await;

   {
      let mut dav = state.lock().unwrap();
      dav.collections.remove("/cal/home/");
      dav.delete("/cal/home/h1.ics");
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.collections_removed, 1);

   let client = pool.get().await.unwrap();
   let cols = store::list_collections(&client, ACCOUNT, DavKind::CalDav)
      .await
      .unwrap();
   assert_eq!(cols.len(), 1);
   assert_eq!(cols[0].href, "/cal/work/");
   let home_id = store::id_for_href(DavKind::CalDav, "/cal/home/");
   let home = store::get_collection(&client, ACCOUNT, &home_id)
      .await
      .unwrap()
      .unwrap();
   assert!(home.destroyed);
   let h1 = store::get_resource(
      &client,
      ACCOUNT,
      &store::id_for_uid(DavKind::CalDav, "h1@x"),
   )
   .await
   .unwrap()
   .unwrap();
   assert!(h1.destroyed, "members tombstoned with their collection");
}

#[tokio::test(flavor = "multi_thread")]
async fn put_create_update_and_etag_conflict() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
   }
   run_sync(&pool, &endpoint).await;
   let handle = service::spawn(pool.clone(), ACCOUNT.to_owned());
   let col_id = store::id_for_href(DavKind::CalDav, "/cal/work/");

   let row = handle
      .put_resource(
         DavKind::CalDav,
         col_id.clone(),
         None,
         ical("new@x", "Created"),
      )
      .await
      .expect("create succeeds");
   assert_eq!(row.uid, "new@x");
   assert!(row.href.starts_with("/cal/work/"));
   assert!(row.etag.is_some());
   {
      let dav = state.lock().unwrap();
      assert!(
         dav.requests.iter().any(|(method, path, _, inm)| {
            method == "PUT" && path == &row.href && inm.as_deref() == Some("*")
         }),
         "create guarded by If-None-Match: *"
      );
      assert_eq!(dav.resources[&row.href].data, ical("new@x", "Created"));
   }

   let updated = handle
      .put_resource(
         DavKind::CalDav,
         col_id.clone(),
         Some(row.id.clone()),
         ical("new@x", "Edited"),
      )
      .await
      .expect("update succeeds");
   assert_eq!(updated.id, row.id);
   assert_ne!(updated.etag, row.etag);
   {
      let dav = state.lock().unwrap();
      assert!(
         dav.requests
            .iter()
            .any(|(method, path, im, _)| method == "PUT" && path == &row.href && im == &row.etag),
         "update guarded by If-Match with the stored etag"
      );
      drop(dav);
   }

   // Conflict: server-side edit invalidates our ETag; the PUT must fail
   // typed and leave both sides consistent.
   {
      let mut dav = state.lock().unwrap();
      let remote_edit = ical("new@x", "Remote edit");
      dav.put(&row.href, &remote_edit);
   }
   let err = handle
      .put_resource(
         DavKind::CalDav,
         col_id.clone(),
         Some(row.id.clone()),
         ical("new@x", "Doomed"),
      )
      .await
      .expect_err("stale etag must 412");
   assert!(
      matches!(err, DavError::PreconditionFailed { .. }),
      "got {err:?}"
   );
   let client = pool.get().await.unwrap();
   let db_row = store::get_resource(&client, ACCOUNT, &row.id)
      .await
      .unwrap()
      .unwrap();
   assert_eq!(
      db_row.json, updated.json,
      "conflict left the cache at the last acknowledged write"
   );
   {
      let dav = state.lock().unwrap();
      assert_eq!(
         dav.resources[&row.href].data,
         ical("new@x", "Remote edit"),
         "server kept the concurrent edit"
      );
   }
   handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn put_caches_server_rewritten_bytes_even_when_an_etag_is_returned() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
   }
   run_sync(&pool, &endpoint).await;
   state.lock().unwrap().rewrite_next_put = Some(ical("new@x", "Server canonical"));

   let handle = service::spawn(pool, ACCOUNT.to_owned());
   let row = handle
      .put_resource(
         DavKind::CalDav,
         store::id_for_href(DavKind::CalDav, "/cal/work/"),
         None,
         ical("new@x", "Client bytes"),
      )
      .await
      .expect("create succeeds");
   let json = serde_json::from_str::<serde_json::Value>(&row.json).unwrap();
   assert_eq!(json["title"], "Server canonical");
   assert_eq!(row.raw, ical("new@x", "Server canonical"));
   handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn delete_and_move_update_cache_exactly() {
   let Some((pool, state, endpoint)) = setup(false).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/cal/work/".to_owned(), "Work".to_owned());
      dav.collections
         .insert("/cal/home/".to_owned(), "Home".to_owned());
      dav.put("/cal/work/e1.ics", &ical("e1@x", "One"));
      dav.put("/cal/work/e2.ics", &ical("e2@x", "Two"));
   }
   run_sync(&pool, &endpoint).await;
   let handle = service::spawn(pool.clone(), ACCOUNT.to_owned());

   let e1_id = store::id_for_uid(DavKind::CalDav, "e1@x");
   handle
      .delete_resource(DavKind::CalDav, e1_id.clone())
      .await
      .expect("delete succeeds");
   {
      let dav = state.lock().unwrap();
      assert!(!dav.resources.contains_key("/cal/work/e1.ics"));
      assert!(
         dav.requests.iter().any(|(method, path, im, _)| {
            method == "DELETE" && path == "/cal/work/e1.ics" && im.is_some()
         }),
         "delete guarded by If-Match"
      );
      drop(dav);
   }
   let client = pool.get().await.unwrap();
   let row = store::get_resource(&client, ACCOUNT, &e1_id)
      .await
      .unwrap()
      .unwrap();
   assert!(row.destroyed);
   drop(client);

   // Idempotence: deleting an already-deleted resource is an error at the
   // service level (unknown resource), and the remote 404 path is covered
   // by the engine's DELETE handling.

   let e2_id = store::id_for_uid(DavKind::CalDav, "e2@x");
   let home_id = store::id_for_href(DavKind::CalDav, "/cal/home/");
   let moved = handle
      .move_resource(DavKind::CalDav, e2_id.clone(), home_id.clone())
      .await
      .expect("move succeeds");
   assert_eq!(moved.href, "/cal/home/e2.ics");
   assert_eq!(moved.collection_id, home_id);
   assert_eq!(moved.uid, "e2@x");
   {
      let dav = state.lock().unwrap();
      assert!(!dav.resources.contains_key("/cal/work/e2.ics"));
      assert!(dav.resources.contains_key("/cal/home/e2.ics"));
      drop(dav);
   }
   let client = pool.get().await.unwrap();
   let moved_row = store::get_resource(&client, ACCOUNT, &e2_id)
      .await
      .unwrap()
      .unwrap();
   assert!(!moved_row.destroyed);
   assert_eq!(moved.id, e2_id, "MOVE preserves the JMAP resource id");
   assert_eq!(moved_row.href, "/cal/home/e2.ics");

   // A follow-up sync sees server state already matching the cache.
   drop(client);
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(
      stats.resources_created + stats.resources_updated + stats.resources_removed,
      0,
      "cache already consistent after remote-first writes"
   );
   handle.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn carddav_sync_and_roundtrip() {
   let Some((pool, state, endpoint)) = setup(true).await else {
      return;
   };
   {
      let mut dav = state.lock().unwrap();
      dav.collections
         .insert("/card/default/".to_owned(), "Contacts".to_owned());
      dav.put("/card/default/c1.vcf", &vcard("c1@x", "Ada Lovelace"));
   }
   let stats = run_sync(&pool, &endpoint).await;
   assert_eq!(stats.collections_created, 1);
   assert_eq!(stats.resources_created, 1);

   let client = pool.get().await.unwrap();
   let st = store::get_state(&client, ACCOUNT).await.unwrap();
   assert_eq!(st.addressbook_modseq, 1);
   assert_eq!(st.contact_card_modseq, 1);
   assert_eq!(st.calendar_modseq, 0, "streams are independent");
   assert_eq!(st.calendar_event_modseq, 0);

   let col_id = store::id_for_href(DavKind::CardDav, "/card/default/");
   let rows = store::list_resources(&client, ACCOUNT, &col_id)
      .await
      .unwrap();
   assert_eq!(rows.len(), 1);
   assert_eq!(rows[0].kind, "card");
   assert_eq!(rows[0].uid, "c1@x");
   let json = serde_json::from_str::<serde_json::Value>(&rows[0].json).unwrap();
   assert_eq!(json["@type"], "Card");
   assert_eq!(json["version"], "2.0");
   assert_eq!(json["name"]["full"], "Ada Lovelace");

   // Round trip: raw → Card → vCard → Card is lossless.
   let (uid, card) = contact::vcard_to_card(&rows[0].raw).unwrap();
   let regenerated = contact::card_to_vcard(&card).unwrap();
   let (uid2, card2) = contact::vcard_to_card(&regenerated).unwrap();
   assert_eq!(uid, uid2);
   assert_eq!(card, card2);
}

#[tokio::test(flavor = "multi_thread")]
async fn query_snapshots_store_and_expire() {
   let Some(pool) = testkit::test_pool().await else {
      return;
   };
   let client = pool.get().await.unwrap();
   client
      .execute(
         "INSERT INTO accounts (id, email, provider, display_name, bearer_token_hash, created_at)
         VALUES ($1, 'u@example.com', 'imap', 'U', '\\x00'::bytea, 0)",
         &[&ACCOUNT],
      )
      .await
      .unwrap();

   let ids = vec!["a".to_owned(), "b".to_owned()];
   store::save_query_snapshot(&client, ACCOUNT, "CalendarEvent", "h1", 7, &ids, 100, 200)
      .await
      .unwrap();
   let snap = store::get_query_snapshot(&client, ACCOUNT, "CalendarEvent", "h1", 7, 150)
      .await
      .unwrap()
      .expect("snapshot live before expiry");
   assert_eq!(snap.modseq, 7);
   assert_eq!(snap.ids, ids);

   assert_eq!(
      store::get_query_snapshot(&client, ACCOUNT, "CalendarEvent", "h1", 7, 200)
         .await
         .unwrap(),
      None,
      "expired snapshot is not served"
   );
   let purged = store::delete_expired_query_snapshots(&client, 200)
      .await
      .unwrap();
   assert_eq!(purged, 1);
}
