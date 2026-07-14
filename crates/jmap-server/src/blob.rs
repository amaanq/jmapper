//! Blob download (RFC 8620 §6) — `GET /download/{accountId}/{blobId}/{name}`.
//!
//! IDs are `blob-{msgid}`, `blob-{msgid}~{partId}`, or
//! `blob-upload-{sha256}`. `~` keeps part ids unambiguous when message ids
//! contain hyphens.

use std::time::Duration;

use axum::{
   Extension,
   extract::{
      Path,
      Query,
      State,
   },
   http::{
      HeaderMap,
      HeaderValue,
      StatusCode,
      header,
   },
   response::{
      IntoResponse as _,
      Response,
   },
};
use imap_sync::account::AccountRequest;
use mail_parser::MimeHeaders as _;
use serde::Deserialize;
use tokio::{
   sync::oneshot,
   time,
};

use crate::{
   auth::AuthedAccount,
   error::ApiError,
   state::AppState,
};

#[derive(Debug, Default, Deserialize)]
pub struct DownloadQuery {
   #[serde(rename = "type")]
   mime_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlobRef {
   Message { msgid: String },
   Part { msgid: String, part_id: String },
   Upload { sha: String },
}

impl BlobRef {
   #[must_use]
   pub fn msgid(&self) -> Option<&str> {
      match self {
         Self::Message { msgid } | Self::Part { msgid, .. } => Some(msgid),
         Self::Upload { .. } => None,
      }
   }
}

/// Parse a wire blob id. Returns `None` if the shape doesn't match any of the
/// known prefixes — the handler maps that to a 404, consistent with how JMAP
/// treats unknown blob ids (RFC 8620 §6.2).
#[must_use]
pub fn parse_blob_id(id: &str) -> Option<BlobRef> {
   // `blob-upload-{sha}` comes first because its msgid part contains
   // a hyphen (`upload-{sha}`) that the Message/Part parse rules would
   // otherwise split incorrectly.
   if let Some(sha) = id.strip_prefix("blob-upload-") {
      if sha.is_empty() {
         return None;
      }
      return Some(BlobRef::Upload {
         sha: sha.to_owned(),
      });
   }
   let rest = id.strip_prefix("blob-")?;
   if rest.is_empty() {
      return None;
   }
   // `~` is the unambiguous msgid/part-id separator. Split at the LAST
   // `~` so even a (rare) RFC 5322 Message-ID containing one is parsed
   // correctly — our synthesized msgids never include `~`, and the
   // mint side always picks the last `~` as the boundary. Validate the
   // part-id shape so we don't misinterpret tildes in arbitrary
   // contexts.
   if let Some((stem, part_id)) = rest.rsplit_once('~')
      && !stem.is_empty()
      && is_known_part_id(part_id)
   {
      return Some(BlobRef::Part {
         msgid:   stem.to_owned(),
         part_id: part_id.to_owned(),
      });
   }
   Some(BlobRef::Message {
      msgid: rest.to_owned(),
   })
}

/// Resolve any account-scoped blob id to decoded bytes for JMAP methods such
/// as Email/parse. Missing ids return `Ok(None)`; storage and sync failures
/// remain errors so callers do not misreport an outage as a missing blob.
pub(crate) async fn load_blob_bytes(
   state: &AppState,
   account_id: &str,
   blob_id: &str,
) -> Result<Option<Vec<u8>>, ApiError> {
   let Some(blob) = parse_blob_id(blob_id) else {
      return Ok(None);
   };
   match blob {
      BlobRef::Upload { .. } => {
         let row = state
            .pool()
            .get()
            .await
            .map_err(|error| ApiError::Internal(format!("db pool: {error}")))?
            .query_opt(
               "SELECT bytes FROM uploaded_blobs WHERE account_id = $1 AND blob_id = $2 AND \
                expires_at > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT",
               &[&account_id, &blob_id],
            )
            .await
            .map_err(|error| ApiError::Internal(format!("load uploaded blob: {error}")))?;
         Ok(row.map(|row| row.get::<_, Vec<u8>>(0)))
      },
      BlobRef::Message { msgid } => {
         if !message_exists(state, account_id, &msgid).await? {
            return Ok(None);
         }
         load_raw_rfc822(state, account_id, &msgid).await.map(Some)
      },
      BlobRef::Part { msgid, part_id } => {
         if !message_exists(state, account_id, &msgid).await? {
            return Ok(None);
         }
         let raw = load_raw_rfc822(state, account_id, &msgid).await?;
         let Some(parsed) = mail_parser::MessageParser::default().parse(&raw) else {
            return Ok(None);
         };
         Ok(locate_part(&parsed, &part_id).map(|(bytes, ..)| bytes))
      },
   }
}

async fn message_exists(state: &AppState, account_id: &str, msgid: &str) -> Result<bool, ApiError> {
   state
      .pool()
      .get()
      .await
      .map_err(|error| ApiError::Internal(format!("db pool: {error}")))?
      .query_opt(
         "SELECT 1 FROM messages WHERE account_id = $1 AND msgid = $2",
         &[&account_id, &msgid],
      )
      .await
      .map(|row| row.is_some())
      .map_err(|error| ApiError::Internal(format!("message lookup: {error}")))
}

/// Whitelist the part-id shapes the projection emits. New shapes added to
/// `project_to_jmap_with_msgid` must also extend this check.
fn is_known_part_id(part_id: &str) -> bool {
   if part_id == "text-1" || part_id == "html-1" {
      return true;
   }
   if let Some(rest) = part_id.strip_prefix("att-") {
      return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit());
   }
   // `p{n}`: mail-parser part index, emitted by Email/get bodyStructure.
   if let Some(rest) = part_id.strip_prefix('p') {
      return !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit());
   }
   false
}

/// Handle `GET /download/{accountId}/{blobId}/{name}`.
///
/// `name` is ignored beyond the Content-Disposition echo — clients use it to
/// nudge Save-As dialogs.
///
/// # Errors
///
/// Returns [`ApiError::Unauthorized`] if the path account differs from the
/// authenticated account, and [`ApiError::NotFound`] if the blob id is
/// unparseable or resolves to no stored bytes. Storage, sync-task, and
/// timeout failures surface as [`ApiError::Internal`].
pub async fn download_handler(
   State(state): State<AppState>,
   Extension(AuthedAccount(auth)): Extension<AuthedAccount>,
   Path((account_id, blob_id, name)): Path<(String, String, String)>,
   Query(query): Query<DownloadQuery>,
) -> Result<Response, ApiError> {
   if account_id != auth.id {
      // Do NOT leak whether the account exists. Match the "auth-only sees
      // their own account" contract enforced everywhere else.
      return Err(ApiError::Unauthorized);
   }

   let Some(blob) = parse_blob_id(&blob_id) else {
      return Err(ApiError::NotFound);
   };

   match blob {
      BlobRef::Message { msgid } => {
         serve_message(
            &state,
            &account_id,
            &msgid,
            &name,
            query.mime_type.as_deref(),
         )
         .await
      },
      BlobRef::Part { msgid, part_id } => {
         serve_part(
            &state,
            &account_id,
            &msgid,
            &part_id,
            &name,
            query.mime_type.as_deref(),
         )
         .await
      },
      BlobRef::Upload { .. } => {
         serve_upload(
            &state,
            &account_id,
            &blob_id,
            &name,
            query.mime_type.as_deref(),
         )
         .await
      },
   }
}

async fn serve_message(
   state: &AppState,
   account_id: &str,
   msgid: &str,
   name: &str,
   requested_mime: Option<&str>,
) -> Result<Response, ApiError> {
   let raw = load_raw_rfc822(state, account_id, msgid).await?;
   let mut headers = HeaderMap::new();
   headers.insert(
      header::CONTENT_TYPE,
      mime_header(requested_mime, "message/rfc822"),
   );
   headers.insert(header::CONTENT_LENGTH, HeaderValue::from(raw.len() as u64));
   insert_download_headers(&mut headers, name);
   Ok((StatusCode::OK, headers, raw).into_response())
}

async fn serve_upload(
   state: &AppState,
   account_id: &str,
   blob_id: &str,
   name: &str,
   requested_mime: Option<&str>,
) -> Result<Response, ApiError> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|err| ApiError::Internal(format!("db pool: {err}")))?;
   let row = client
      .query_opt(
         "SELECT bytes, content_type FROM uploaded_blobs WHERE account_id = $1 AND blob_id = $2 \
          AND expires_at > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT",
         &[&account_id, &blob_id],
      )
      .await
      .map_err(|err| ApiError::Internal(format!("load uploaded blob: {err}")))?
      .ok_or(ApiError::NotFound)?;
   let bytes = row.get::<_, Vec<u8>>(0);
   let stored_mime = row.get::<_, String>(1);
   let mut headers = HeaderMap::new();
   headers.insert(
      header::CONTENT_TYPE,
      mime_header(requested_mime, &stored_mime),
   );
   headers.insert(
      header::CONTENT_LENGTH,
      HeaderValue::from(bytes.len() as u64),
   );
   insert_download_headers(&mut headers, name);
   Ok((StatusCode::OK, headers, bytes).into_response())
}

async fn serve_part(
   state: &AppState,
   account_id: &str,
   msgid: &str,
   part_id: &str,
   name: &str,
   requested_mime: Option<&str>,
) -> Result<Response, ApiError> {
   let raw = load_raw_rfc822(state, account_id, msgid).await?;
   let parsed = mail_parser::MessageParser::default()
      .parse(&*raw)
      .ok_or_else(|| ApiError::Internal("mail-parser could not re-parse cached body".into()))?;

   let (bytes, mime, part_name) = locate_part(&parsed, part_id).ok_or(ApiError::NotFound)?;

   let mut headers = HeaderMap::new();
   headers.insert(header::CONTENT_TYPE, mime_header(requested_mime, &mime));
   headers.insert(
      header::CONTENT_LENGTH,
      HeaderValue::from(bytes.len() as u64),
   );
   // RFC 5987 filename* — URL-encode the UTF-8 name so non-ASCII filenames
   // don't get mangled by the header layer. Falling back to `name` from the
   // URL if the part has no name of its own matches what Perl's handler does.
   let filename = if name.is_empty() {
      part_name.as_deref().unwrap_or_default()
   } else {
      name
   };
   insert_download_headers(&mut headers, filename);

   Ok((StatusCode::OK, headers, bytes).into_response())
}

fn mime_header(requested: Option<&str>, fallback: &str) -> HeaderValue {
   requested
      .and_then(|value| HeaderValue::from_str(value).ok())
      .or_else(|| HeaderValue::from_str(fallback).ok())
      .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream"))
}

fn insert_download_headers(headers: &mut HeaderMap, filename: &str) {
   const RFC5987_UNSAFE: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS
      .add(b' ')
      .add(b'"')
      .add(b'%')
      .add(b'\'')
      .add(b'(')
      .add(b')')
      .add(b'*')
      .add(b',')
      .add(b'/')
      .add(b':')
      .add(b';')
      .add(b'<')
      .add(b'=')
      .add(b'>')
      .add(b'?')
      .add(b'@')
      .add(b'[')
      .add(b'\\')
      .add(b']')
      .add(b'{')
      .add(b'}');

   headers.insert(
      header::CACHE_CONTROL,
      HeaderValue::from_static("private, immutable, max-age=31536000"),
   );
   if filename.is_empty() {
      return;
   }

   let encoded = percent_encoding::utf8_percent_encode(filename, RFC5987_UNSAFE);
   if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{encoded}")) {
      headers.insert(header::CONTENT_DISPOSITION, value);
   }
}

/// Look up a part by id. Returns decoded bytes, its Content-Type, and the
/// original filename (if any). Mirrors the projection rules in
/// `project_to_jmap_with_msgid` so ids round-trip.
fn locate_part(
   parsed: &mail_parser::Message<'_>,
   part_id: &str,
) -> Option<(Vec<u8>, String, Option<String>)> {
   match part_id {
      "text-1" => {
         let body = parsed.body_text(0)?.into_owned();
         Some((body.into_bytes(), "text/plain; charset=utf-8".into(), None))
      },
      "html-1" => {
         let body = parsed.body_html(0)?.into_owned();
         Some((body.into_bytes(), "text/html; charset=utf-8".into(), None))
      },
      _ if part_id.starts_with("att-") => {
         let idx = part_id.strip_prefix("att-")?.parse::<usize>().ok()?;
         let att = parsed.attachments().nth(idx)?;
         let mime = att.content_type().map_or_else(
            || "application/octet-stream".into(),
            |ct| format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("octet-stream")),
         );
         Some((
            att.contents().to_vec(),
            mime,
            att.attachment_name().map(str::to_owned),
         ))
      },
      _ if part_id.starts_with('p') => {
         let idx = part_id.strip_prefix('p')?.parse::<usize>().ok()?;
         let part = parsed.parts.get(idx)?;
         let mime = part.content_type().map_or_else(
            || "application/octet-stream".into(),
            |ct| format!("{}/{}", ct.ctype(), ct.subtype().unwrap_or("octet-stream")),
         );
         Some((
            part.contents().to_vec(),
            mime,
            part.attachment_name().map(str::to_owned),
         ))
      },
      _ => None,
   }
}

/// Fetch the raw RFC 5322 bytes, going through the account task on a miss
/// exactly like `Email/get` with bodyValues does. Once the fetch completes,
/// the row exists in `raw_messages` with `raw_rfc822` populated.
async fn load_raw_rfc822(
   state: &AppState,
   account_id: &str,
   msgid: &str,
) -> Result<Vec<u8>, ApiError> {
   let cached = state
      .pool()
      .get()
      .await
      .map_err(|err| ApiError::Internal(format!("db pool: {err}")))?
      .query_opt(
         "SELECT raw_rfc822 FROM raw_messages WHERE account_id = $1 AND msgid = $2",
         &[&account_id, &msgid],
      )
      .await
      .map_err(|err| ApiError::Internal(format!("raw_messages lookup: {err}")))?
      .map(|row| row.get::<_, Vec<u8>>(0));

   if let Some(bytes) = cached {
      return Ok(bytes);
   }

   let tx = state
      .account_sender(account_id)
      .ok_or_else(|| ApiError::Internal(format!("no sync task for account {account_id}")))?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::FetchBody {
      msgid: msgid.to_owned(),
      respond,
   })
   .await
   .map_err(|_| ApiError::Internal("sync task channel closed".into()))?;
   let result = time::timeout(Duration::from_secs(30), rx)
      .await
      .map_err(|_| ApiError::Internal("body fetch timed out".into()))?
      .map_err(|_| ApiError::Internal("sync task dropped fetch".into()))?;
   result.map_err(|err| ApiError::Internal(format!("body fetch failed: {err}")))?;

   let bytes = state
      .pool()
      .get()
      .await
      .map_err(|err| ApiError::Internal(format!("db pool: {err}")))?
      .query_opt(
         "SELECT raw_rfc822 FROM raw_messages WHERE account_id = $1 AND msgid = $2",
         &[&account_id, &msgid],
      )
      .await
      .map_err(|err| ApiError::Internal(format!("raw_messages re-read: {err}")))?
      .map(|row| row.get::<_, Vec<u8>>(0));
   bytes.ok_or(ApiError::NotFound)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn parse_whole_message() {
      assert_eq!(
         parse_blob_id("blob-m1"),
         Some(BlobRef::Message { msgid: "m1".into() })
      );
   }

   #[test]
   fn parse_part_attachment() {
      assert_eq!(
         parse_blob_id("blob-m1~att-0"),
         Some(BlobRef::Part {
            msgid:   "m1".into(),
            part_id: "att-0".into(),
         })
      );
   }

   #[test]
   fn parse_part_text_html() {
      assert_eq!(
         parse_blob_id("blob-m1~text-1"),
         Some(BlobRef::Part {
            msgid:   "m1".into(),
            part_id: "text-1".into(),
         })
      );
      assert_eq!(
         parse_blob_id("blob-abc123~html-1"),
         Some(BlobRef::Part {
            msgid:   "abc123".into(),
            part_id: "html-1".into(),
         })
      );
   }

   #[test]
   fn parse_msgid_containing_hyphen() {
      // Gmail's `UID-<n>` style msgids include hyphens; the `~` separator
      // makes that unambiguous.
      assert_eq!(
         parse_blob_id("blob-UID-12345"),
         Some(BlobRef::Message {
            msgid: "UID-12345".into(),
         })
      );
      assert_eq!(
         parse_blob_id("blob-UID-12345~att-0"),
         Some(BlobRef::Part {
            msgid:   "UID-12345".into(),
            part_id: "att-0".into(),
         })
      );
   }

   #[test]
   fn parse_msgid_ending_with_part_id_form() {
      // Audit regression: with the old `-` separator, a msgid that
      // happened to end in `-att-0` was indistinguishable from a part
      // form. With `~` the whole-message blob remains reachable.
      assert_eq!(
         parse_blob_id("blob-m-att-0"),
         Some(BlobRef::Message {
            msgid: "m-att-0".into(),
         })
      );
   }

   #[test]
   fn parse_rejects_unknown_part_id_after_tilde() {
      // Bogus part-id falls back to "this is a Message blob whose msgid
      // contains `~bogus`" — the part-id whitelist is the gate.
      assert_eq!(
         parse_blob_id("blob-m1~bogus"),
         Some(BlobRef::Message {
            msgid: "m1~bogus".into(),
         })
      );
   }

   #[test]
   fn parse_upload() {
      assert_eq!(
         parse_blob_id("blob-upload-deadbeef"),
         Some(BlobRef::Upload {
            sha: "deadbeef".into(),
         })
      );
   }

   #[test]
   fn parse_rejects_unknown_prefixes() {
      assert_eq!(parse_blob_id("m-123"), None);
      assert_eq!(parse_blob_id("blob-"), None);
      assert_eq!(parse_blob_id(""), None);
      assert_eq!(parse_blob_id("blob-upload-"), None);
   }

   #[test]
   fn blob_ref_msgid_accessor() {
      assert_eq!(BlobRef::Message { msgid: "m1".into() }.msgid(), Some("m1"));
      assert_eq!(
         BlobRef::Part {
            msgid:   "m1".into(),
            part_id: "att-0".into(),
         }
         .msgid(),
         Some("m1")
      );
      assert_eq!(
         BlobRef::Upload {
            sha: "deadbeef".into(),
         }
         .msgid(),
         None
      );
   }

   #[test]
   fn download_headers_are_cacheable_and_filename_safe() {
      let mut headers = HeaderMap::new();
      insert_download_headers(&mut headers, "résumé \"final\".pdf");
      assert_eq!(
         headers
            .get(header::CACHE_CONTROL)
            .and_then(|value| value.to_str().ok()),
         Some("private, immutable, max-age=31536000")
      );
      let disposition = headers
         .get(header::CONTENT_DISPOSITION)
         .and_then(|value| value.to_str().ok())
         .unwrap();
      assert!(disposition.starts_with("attachment; filename*=UTF-8''"));
      assert!(!disposition.contains('"'));
   }
}
