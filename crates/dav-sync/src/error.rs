//! Typed errors for the DAV stack.
//!
//! Variants are grouped by which layer can
//! act on them: transport failures retry, `SyncTokenInvalid` triggers a
//! full resync, `PreconditionFailed` surfaces to the client as an `ETag`
//! conflict, conversion errors mark the single resource and never abort a
//! batch.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DavError {
   #[error("http transport: {0}")]
   Transport(#[from] reqwest::Error),

   /// Non-2xx (and non-207) response to a DAV operation.
   #[error("http status {status} for {method} {url}")]
   Status {
      status: u16,
      method: String,
      url:    String,
   },

   /// The server rejected our `If-Match` — someone else changed the
   /// resource since the `ETag` we hold. RFC 4918 §12.
   #[error("etag precondition failed for {url}")]
   PreconditionFailed { url: String },

   /// RFC 6578 §3.2: the sync token is no longer valid; the caller must
   /// drop it and run a full resync of the collection.
   #[error("sync token no longer valid for {href}")]
   SyncTokenInvalid { href: String },

   #[error("xml parse: {0}")]
   Xml(String),

   /// The response was structurally valid but missing something the
   /// operation requires (e.g. a PROPFIND without the requested prop).
   #[error("dav response missing {what} for {href}")]
   MissingProperty { what: &'static str, href: String },

   #[error("redirect loop or limit exceeded for {url}")]
   TooManyRedirects { url: String },

   #[error("DAV response from {url} exceeds the {limit} byte limit")]
   ResponseTooLarge { url: String, limit: usize },

   #[error("invalid url: {0}")]
   Url(String),

   #[error("conversion: {0}")]
   Convert(#[from] ConvertError),

   #[error("postgres: {0}")]
   Pg(#[from] tokio_postgres::Error),

   #[error("postgres pool: {0}")]
   PgPool(#[from] deadpool_postgres::PoolError),

   #[error("{0}")]
   Other(String),
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ConvertError {
   #[error("invalid {format}: {reason}")]
   Invalid {
      format: &'static str,
      reason: String,
   },
}

use std::result::Result as StdResult;

pub type Result<T, E = DavError> = StdResult<T, E>;
