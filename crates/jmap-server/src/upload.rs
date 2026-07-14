//! Account-scoped blob upload (RFC 8620 §6.1).

use axum::{
   Extension,
   body::Bytes,
   extract::{
      Path,
      State,
   },
   http::{
      HeaderMap,
      header::CONTENT_TYPE,
   },
   response::Json,
};
use jmapper_codegen::queries::blobs;
use sha2::{
   Digest as _,
   Sha256,
};

use crate::{
   auth::AuthedAccount,
   error::ApiError,
   state::AppState,
};

/// Uploaded blobs expire after one hour.
pub(crate) const DEFAULT_EXPIRY_SECS: i64 = 3600;

/// Cap uploads so a runaway client can't OOM the process. 50 MiB is enough
/// for a 20 MB attachment + headers; anything larger should land via the
/// MTA, not Email/import.
pub(crate) const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;

/// POST /upload/{accountId} — stores an uploaded blob and returns its metadata.
///
/// # Errors
///
/// Returns [`ApiError::Unauthorized`] if the path account does not match the
/// authenticated account, [`ApiError::BadRequest`] if the body is empty or
/// exceeds [`MAX_UPLOAD_BYTES`], and [`ApiError::Internal`] if a database
/// connection cannot be acquired or the blob insert fails.
pub async fn upload_handler(
   State(state): State<AppState>,
   Extension(AuthedAccount(auth)): Extension<AuthedAccount>,
   Path(account_id): Path<String>,
   headers: HeaderMap,
   body: Bytes,
) -> Result<Json<serde_json::Value>, ApiError> {
   if account_id != auth.id {
      return Err(ApiError::Unauthorized);
   }
   if body.len() > MAX_UPLOAD_BYTES {
      return Err(ApiError::BadRequest(format!(
         "upload exceeds {MAX_UPLOAD_BYTES} byte cap",
      )));
   }
   if body.is_empty() {
      return Err(ApiError::BadRequest("upload is empty".into()));
   }

   let content_type = headers
      .get(CONTENT_TYPE)
      .and_then(|value| value.to_str().ok())
      .unwrap_or("application/octet-stream")
      .to_owned();

   let mut hasher = Sha256::new();
   hasher.update(&body);
   let sha = hex::encode(hasher.finalize());
   let blob_id = format!("blob-upload-{sha}");

   let now = chrono::Utc::now().timestamp();
   let expires_at = now + DEFAULT_EXPIRY_SECS;

   let client = state
      .pool()
      .get()
      .await
      .map_err(|err| ApiError::Internal(format!("db pool: {err}")))?;
   blobs::upsert_uploaded_blob()
      .bind(
         &client,
         &account_id.as_str(),
         &blob_id.as_str(),
         &content_type.as_str(),
         &body.as_ref(),
         &now,
         &expires_at,
      )
      .await
      .map_err(|err| ApiError::Internal(format!("insert upload: {err}")))?;

   Ok(Json(serde_json::json!({
       "accountId": account_id,
       "blobId": blob_id,
       "type": content_type,
       "size": body.len(),
   })))
}
