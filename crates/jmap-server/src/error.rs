//! HTTP error responses in RFC 7807 (problem+json) shape.

use axum::{
   Json,
   http::{
      HeaderValue,
      StatusCode,
      header::CONTENT_TYPE,
   },
   response::{
      IntoResponse,
      Response,
   },
};
use jmap_protocol::error::ProblemDetails;

/// Convert a status code + urn + detail into a full JSON problem response.
pub fn problem(status: StatusCode, urn_kind: &str, detail: impl Into<String>) -> Response {
   let body = ProblemDetails::urn(urn_kind)
      .with_status(status.as_u16())
      .with_detail(detail);
   let mut resp = Json(body).into_response();
   *resp.status_mut() = status;
   resp.headers_mut().insert(
      CONTENT_TYPE,
      HeaderValue::from_static("application/problem+json"),
   );
   resp
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
   #[error("unauthorized")]
   Unauthorized,
   #[error("bad request: {0}")]
   BadRequest(String),
   #[error("JMAP request error {kind}: {detail}")]
   JmapRequest {
      kind:   &'static str,
      detail: String,
   },
   #[error("not found")]
   NotFound,
   #[error("internal: {0}")]
   Internal(String),
}

impl IntoResponse for ApiError {
   fn into_response(self) -> Response {
      match self {
         Self::Unauthorized => {
            problem(
               StatusCode::UNAUTHORIZED,
               "unauthorized",
               "authentication required",
            )
         },
         Self::BadRequest(detail) => problem(StatusCode::BAD_REQUEST, "invalidRequest", detail),
         Self::JmapRequest { kind, detail } => problem(StatusCode::BAD_REQUEST, kind, detail),
         Self::NotFound => problem(StatusCode::NOT_FOUND, "notFound", "resource not found"),
         Self::Internal(detail) => {
            tracing::error!(error = %detail, "internal server error");
            problem(StatusCode::INTERNAL_SERVER_ERROR, "serverFail", detail)
         },
      }
   }
}
