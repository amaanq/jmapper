//! JMAP error shapes (method-level and request-level).

use serde::{
   Deserialize,
   Serialize,
};

/// RFC 8620 §3.6.2 — method-level error.
///
/// Serialized as `["error", {...}, "<callId>"]` by the method response encoder.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum MethodError {
   ServerUnavailable,
   ServerFail { description: Option<String> },
   ServerPartialFail,
   UnknownMethod,
   InvalidArguments { description: Option<String> },
   InvalidResultReference { description: Option<String> },
   RequestTooLarge,
   Forbidden,
   AccountNotFound,
   AccountNotSupportedByMethod,
   FromAccountNotFound,
   FromAccountNotSupportedByMethod,
   AccountReadOnly,
   AnchorNotFound,
   UnsupportedSort,
   UnsupportedFilter,
   TooManyChanges,
   CannotCalculateChanges,
   CannotCalculateOccurrences,
   ExpandDurationTooLarge,
   StateMismatch,
}

/// RFC 8620 §3.7 — per-`SetError` on Set-method results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase")]
#[non_exhaustive]
pub enum SetError {
   Forbidden {
      description: Option<String>,
   },
   OverQuota {
      description: Option<String>,
   },
   TooLarge {
      description: Option<String>,
   },
   RateLimit {
      description: Option<String>,
   },
   NotFound {
      description: Option<String>,
   },
   InvalidPatch {
      description: Option<String>,
   },
   WillDestroy {
      description: Option<String>,
   },
   InvalidProperties {
      properties:  Option<Vec<String>>,
      description: Option<String>,
   },
   Singleton {
      description: Option<String>,
   },
}

/// RFC 8620 §3.6.1 — request-level problem document.
///
/// This is returned as the top-level body with HTTP 4xx/5xx, not inside the
/// methodResponses array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemDetails {
   #[serde(rename = "type")]
   pub kind:   String,
   pub status: Option<u16>,
   pub title:  Option<String>,
   pub detail: Option<String>,
   pub limit:  Option<String>,
}

impl ProblemDetails {
   #[must_use]
   #[inline]
   pub fn urn(kind: &str) -> Self {
      Self {
         kind:   format!("urn:ietf:params:jmap:error:{kind}"),
         status: None,
         title:  None,
         detail: None,
         limit:  None,
      }
   }

   #[must_use]
   #[inline]
   pub const fn with_status(mut self, status: u16) -> Self {
      self.status = Some(status);
      self
   }

   #[must_use]
   #[inline]
   pub fn with_detail<Detail>(mut self, detail: Detail) -> Self
   where
      Detail: Into<String>,
   {
      self.detail = Some(detail.into());
      self
   }
}
