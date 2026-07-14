//! `/healthz` and `/metrics` — small surface, no auth, no CORS concerns.
//!
//! Metrics are lightweight atomic counters scraped into a Prometheus exposition
//! text body on demand. We intentionally don't pull in the `metrics` crate
//! family: the counters we care about fit in a handful of `AtomicU64`s and the
//! resulting surface has zero runtime overhead on the hot path.

use std::{
   fmt::Write as _,
   sync::atomic::{
      AtomicU64,
      Ordering,
   },
   time::Instant,
};

use axum::{
   body::Body,
   extract::{
      MatchedPath,
      State,
   },
   http::{
      Request,
      StatusCode,
      header::CONTENT_TYPE,
   },
   middleware::Next,
   response::{
      IntoResponse as _,
      Response,
   },
};
use imap_sync::account::SYNC_FAILURES_TOTAL;
use jmapper_codegen::queries::state;

use crate::state::AppState;

/// Process-wide metric counters. Kept in a `static` so middleware and handlers
/// can bump them without threading yet-another extension through every axum
/// layer.
pub struct Metrics {
   pub http_requests_total:          AtomicU64,
   pub http_requests_2xx:            AtomicU64,
   pub http_requests_4xx:            AtomicU64,
   pub http_requests_5xx:            AtomicU64,
   pub http_request_duration_ms_sum: AtomicU64,
   pub jmap_method_calls_total:      AtomicU64,
   pub jmap_method_errors_total:     AtomicU64,
   pub body_fetches_total:           AtomicU64,
   pub body_fetch_failures_total:    AtomicU64,
}

pub static METRICS: Metrics = Metrics {
   http_requests_total:          AtomicU64::new(0),
   http_requests_2xx:            AtomicU64::new(0),
   http_requests_4xx:            AtomicU64::new(0),
   http_requests_5xx:            AtomicU64::new(0),
   http_request_duration_ms_sum: AtomicU64::new(0),
   jmap_method_calls_total:      AtomicU64::new(0),
   jmap_method_errors_total:     AtomicU64::new(0),
   body_fetches_total:           AtomicU64::new(0),
   body_fetch_failures_total:    AtomicU64::new(0),
};

const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

/// `GET /healthz` — unconditional 200 once the server is accepting requests.
/// Liveness only; does not probe downstream state.
pub async fn healthz() -> &'static str {
   "ok\n"
}

/// `GET /readyz` — 200 once every *currently configured* account has finished
/// at least one `initial_sync` pass, regardless of how many messages it had.
///
/// The old "count messages" heuristic incorrectly reported 503 forever for a
/// legitimately empty inbox; the `state.initial_sync_done` flag is the
/// authoritative signal.
///
/// The count is scoped to the live account set (the same set HTTP auth uses),
/// not every `state` row in the DB, so churn — removing a synced account and
/// adding an unsynced one on reload — can't flip readiness green before the
/// new account has actually finished its first pass.
pub async fn readyz(State(state): State<AppState>) -> Response {
   let pool = state.pool();
   let ids = state
      .accounts()
      .iter()
      .map(|acct| acct.id.clone())
      .collect::<Vec<String>>();
   let n_accounts = ids.len() as i64;
   if n_accounts == 0 {
      return (StatusCode::OK, "ok\n").into_response();
   }
   let done = async {
      let client = pool.get().await.map_err(|err| err.to_string())?;
      let n = state::count_ready_accounts()
         .bind(&client, &ids)
         .one()
         .await
         .map_err(|err| err.to_string())?;
      Ok::<i64, String>(n)
   }
   .await;
   match done {
      Ok(n) if n >= n_accounts => (StatusCode::OK, "ready\n").into_response(),
      Ok(n) => {
         (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("syncing: {n}/{n_accounts} accounts have completed initial sync\n"),
         )
            .into_response()
      },
      Err(err) => {
         (
            StatusCode::SERVICE_UNAVAILABLE,
            format!("db error: {err}\n"),
         )
            .into_response()
      },
   }
}

/// `GET /metrics` — Prometheus text exposition.
pub async fn metrics(State(state): State<AppState>) -> Response {
   let mut out = String::with_capacity(2048);

   // Build info.
   let _ = writeln!(
      out,
      "# HELP jmapper_build_info build information\n# TYPE jmapper_build_info \
       gauge\njmapper_build_info{{version=\"{BUILD_VERSION}\"}} 1"
   );

   // Configured accounts by provider.
   let mut gmail = 0_u64;
   let mut imap = 0_u64;
   let _ = writeln!(
      out,
      "# HELP jmapper_accounts number of configured accounts by provider\n# TYPE jmapper_accounts \
       gauge"
   );

   // Pull provider split from the DB (authoritative).
   if let Ok(rows) = metrics_rows(
      &state,
      "SELECT provider, COUNT(*) FROM accounts GROUP BY provider",
   )
   .await
   {
      for (provider, n) in rows {
         match provider.as_str() {
            "gmail" => gmail = n as u64,
            _ => imap = n as u64,
         }
      }
   }
   let _ = writeln!(out, "jmapper_accounts{{provider=\"gmail\"}} {gmail}");
   let _ = writeln!(out, "jmapper_accounts{{provider=\"imap\"}} {imap}");

   // Cached message counts per account.
   let _ = writeln!(
      out,
      "# HELP jmapper_messages_cached number of envelopes cached per account\n# TYPE \
       jmapper_messages_cached gauge"
   );
   if let Ok(rows) = metrics_rows(
      &state,
      "SELECT account_id, COUNT(*) FROM messages GROUP BY account_id",
   )
   .await
   {
      for (acct, n) in rows {
         let _ = writeln!(
            out,
            "jmapper_messages_cached{{account=\"{}\"}} {n}",
            escape_label(&acct)
         );
      }
   }

   // Cached raw bodies.
   let _ = writeln!(
      out,
      "# HELP jmapper_bodies_cached number of RFC 5322 bodies cached across all accounts\n# TYPE \
       jmapper_bodies_cached gauge"
   );
   let body_count = match state.pool().get().await {
      Ok(client) => {
         client
            .query_one("SELECT COUNT(*) FROM raw_messages", &[])
            .await
            .map_or(0, |row| row.get::<_, i64>(0))
      },
      Err(_) => 0,
   };
   let _ = writeln!(out, "jmapper_bodies_cached {body_count}");

   // HTTP counters.
   let _ = writeln!(
      out,
      "# HELP jmapper_http_requests_total HTTP requests by status class\n# TYPE \
       jmapper_http_requests_total counter"
   );
   let _ = writeln!(
      out,
      "jmapper_http_requests_total{{class=\"2xx\"}} {}",
      METRICS.http_requests_2xx.load(Ordering::Relaxed)
   );
   let _ = writeln!(
      out,
      "jmapper_http_requests_total{{class=\"4xx\"}} {}",
      METRICS.http_requests_4xx.load(Ordering::Relaxed)
   );
   let _ = writeln!(
      out,
      "jmapper_http_requests_total{{class=\"5xx\"}} {}",
      METRICS.http_requests_5xx.load(Ordering::Relaxed)
   );

   let _ = writeln!(
      out,
      "# HELP jmapper_http_request_duration_ms_sum sum of HTTP request handling latencies\n# TYPE \
       jmapper_http_request_duration_ms_sum counter"
   );
   let _ = writeln!(
      out,
      "jmapper_http_request_duration_ms_sum {}",
      METRICS.http_request_duration_ms_sum.load(Ordering::Relaxed)
   );

   let _ = writeln!(
      out,
      "# HELP jmapper_method_calls_total JMAP method calls dispatched\n# TYPE \
       jmapper_method_calls_total counter\njmapper_method_calls_total {}",
      METRICS.jmap_method_calls_total.load(Ordering::Relaxed)
   );
   let _ = writeln!(
      out,
      "# HELP jmapper_method_errors_total JMAP method calls that returned an error\n# TYPE \
       jmapper_method_errors_total counter\njmapper_method_errors_total {}",
      METRICS.jmap_method_errors_total.load(Ordering::Relaxed)
   );

   let _ = writeln!(
      out,
      "# HELP jmapper_body_fetches_total RFC 5322 body fetches triggered by Email/get\n# TYPE \
       jmapper_body_fetches_total counter"
   );
   let _ = writeln!(
      out,
      "jmapper_body_fetches_total{{outcome=\"ok\"}} {}",
      METRICS.body_fetches_total.load(Ordering::Relaxed)
   );
   let _ = writeln!(
      out,
      "jmapper_body_fetches_total{{outcome=\"error\"}} {}",
      METRICS.body_fetch_failures_total.load(Ordering::Relaxed)
   );

   let _ = writeln!(
      out,
      "# HELP jmapper_imap_sync_failures_total account-task failures that triggered a \
       reconnect\n# TYPE jmapper_imap_sync_failures_total counter"
   );
   let _ = writeln!(
      out,
      "jmapper_imap_sync_failures_total {}",
      SYNC_FAILURES_TOTAL.load(Ordering::Relaxed)
   );

   (
      [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
      out,
   )
      .into_response()
}

/// Axum middleware that increments request + duration counters. Mounted on the
/// `/.well-known/jmap` + `/api` routes only; `/healthz` and `/metrics` are
/// excluded so a scraper can't inflate its own counts.
pub async fn count_requests(req: Request<Body>, next: Next) -> Response {
   let start = Instant::now();
   // Exclude /metrics and /healthz self-scrapes.
   let path = req
      .extensions()
      .get::<MatchedPath>()
      .map_or_else(|| req.uri().path(), MatchedPath::as_str);
   if matches!(path, "/metrics" | "/healthz" | "/readyz") {
      return next.run(req).await;
   }

   let resp = next.run(req).await;
   let elapsed_ms = start.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
   METRICS
      .http_request_duration_ms_sum
      .fetch_add(elapsed_ms, Ordering::Relaxed);
   METRICS.http_requests_total.fetch_add(1, Ordering::Relaxed);
   match resp.status().as_u16() {
      200..=299 => METRICS.http_requests_2xx.fetch_add(1, Ordering::Relaxed),
      400..=499 => METRICS.http_requests_4xx.fetch_add(1, Ordering::Relaxed),
      500..=599 => METRICS.http_requests_5xx.fetch_add(1, Ordering::Relaxed),
      _ => 0,
   };
   resp
}

async fn metrics_rows(
   state: &AppState,
   sql: &str,
) -> Result<Vec<(String, i64)>, deadpool_postgres::PoolError> {
   let client = state.pool().get().await?;
   let rows = client.query(sql, &[]).await.unwrap_or_default();
   Ok(rows
      .into_iter()
      .map(|row| (row.get(0), row.get(1)))
      .collect())
}

fn escape_label(label: &str) -> String {
   label
      .replace('\\', r"\\")
      .replace('"', "\\\"")
      .replace('\n', "\\n")
}
