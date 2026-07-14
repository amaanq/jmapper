//! Wire up the axum router with CORS + auth middleware.

use std::time::Duration;

use axum::{
   Router,
   extract::DefaultBodyLimit,
   http::{
      HeaderValue,
      Method,
      header::{
         AUTHORIZATION,
         CONTENT_TYPE,
      },
   },
   middleware,
   routing::{
      get,
      post,
   },
};
use tower_http::{
   cors::{
      AllowOrigin,
      CorsLayer,
   },
   trace::TraceLayer,
};

use crate::{
   api::api_handler,
   auth::auth_middleware,
   blob::download_handler,
   events::eventsource_handler,
   observability::{
      count_requests,
      healthz,
      metrics,
      readyz,
   },
   session::session_handler,
   state::AppState,
   upload::{
      MAX_UPLOAD_BYTES,
      upload_handler,
   },
};

pub fn build_router(state: AppState, cors_origins: Vec<String>) -> Router {
   let cors = build_cors(cors_origins);

   let authed = Router::new()
      .route("/.well-known/jmap", get(session_handler))
      .route("/api", post(api_handler))
      .route(
         "/download/{accountId}/{blobId}/{name}",
         get(download_handler),
      )
      .route(
         "/upload/{accountId}",
         post(upload_handler).layer(DefaultBodyLimit::max(MAX_UPLOAD_BYTES)),
      )
      .route("/eventsource", get(eventsource_handler))
      .layer(middleware::from_fn_with_state(
         state.clone(),
         auth_middleware,
      ));

   let public = Router::new()
      .route("/healthz", get(healthz))
      .route("/readyz", get(readyz))
      .route("/metrics", get(metrics));

   authed
      .merge(public)
      .layer(middleware::from_fn(count_requests))
      .layer(cors)
      .layer(TraceLayer::new_for_http())
      .with_state(state)
}

fn build_cors(origins: Vec<String>) -> CorsLayer {
   let allow_origin = if origins.is_empty() || origins.iter().any(|origin| origin == "*") {
      AllowOrigin::any()
   } else {
      let parsed = origins
         .into_iter()
         .filter_map(|origin| HeaderValue::from_str(&origin).ok())
         .collect::<Vec<HeaderValue>>();
      AllowOrigin::list(parsed)
   };

   CorsLayer::new()
      .allow_origin(allow_origin)
      .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
      .allow_headers([AUTHORIZATION, CONTENT_TYPE])
      .allow_credentials(false)
      .max_age(Duration::from_hours(24))
}
