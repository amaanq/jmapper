//! HTTP transport for DAV: custom methods, Basic/Bearer credentials, and
//! manual redirect handling.
//!
//! Redirects are handled by hand because reqwest's automatic follower
//! refuses to replay request bodies for 307/308 and may rewrite methods on
//! 301/302 — both wrong for PROPFIND/REPORT, where servers routinely
//! redirect to add a trailing slash and expect the method+body repeated.
//! Redirects are restricted to the original origin. This protects both the
//! Authorization header and private calendar/contact request bodies.

use std::{
   fmt,
   time::Duration,
};

use base64::engine::general_purpose::STANDARD;
use reqwest::redirect::Policy;
use url::Url;

use crate::error::{
   DavError,
   Result,
};

#[derive(Clone)]
pub enum Credentials {
   Basic { username: String, password: String },
   Bearer { token: String },
   None,
}

impl fmt::Debug for Credentials {
   #[inline]
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      match *self {
         Self::Basic { ref username, .. } => {
            f.debug_struct("Basic")
               .field("username", username)
               .field("password", &"[REDACTED]")
               .finish()
         },
         Self::Bearer { .. } => {
            f.debug_struct("Bearer")
               .field("token", &"[REDACTED]")
               .finish()
         },
         Self::None => f.write_str("None"),
      }
   }
}

impl Credentials {
   fn header_value(&self) -> Option<String> {
      use base64::Engine as _;
      match *self {
         Self::Basic {
            ref username,
            ref password,
         } => {
            Some(format!(
               "Basic {}",
               STANDARD.encode(format!("{username}:{password}"))
            ))
         },
         Self::Bearer { ref token } => Some(format!("Bearer {token}")),
         Self::None => None,
      }
   }
}

/// One authenticated origin. `base` supplies scheme+authority; requests
/// take absolute paths (normalized hrefs) or absolute URLs on the same
/// origin.
pub struct DavConnection {
   http:  reqwest::Client,
   base:  Url,
   creds: Credentials,
}

pub struct DavHttpResponse {
   pub status:   u16,
   pub etag:     Option<String>,
   pub location: Option<String>,
   pub body:     Vec<u8>,
}

const MAX_REDIRECTS: usize = 5;
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

fn response_would_exceed_limit(current: usize, incoming: usize) -> bool {
   current
      .checked_add(incoming)
      .is_none_or(|total| total > MAX_RESPONSE_BYTES)
}

impl DavConnection {
   /// # Errors
   ///
   /// Returns an error when the base URL or HTTP client cannot be created.
   #[inline]
   pub fn new(base_url: &str, creds: Credentials) -> Result<Self> {
      let base = Url::parse(base_url).map_err(|err| DavError::Url(format!("{base_url}: {err}")))?;
      if !matches!(base.scheme(), "http" | "https") {
         return Err(DavError::Url(format!(
            "{base_url}: DAV requires http or https"
         )));
      }
      if base.host_str().is_none() {
         return Err(DavError::Url(format!("{base_url}: missing host")));
      }
      if !base.username().is_empty() || base.password().is_some() {
         return Err(DavError::Url(format!(
            "{base_url}: credentials must not be embedded in the URL"
         )));
      }
      let http = reqwest::Client::builder()
         .redirect(Policy::none())
         .timeout(Duration::from_secs(60))
         .build()?;
      Ok(Self { http, base, creds })
   }

   #[must_use]
   #[inline]
   pub const fn base(&self) -> &Url {
      &self.base
   }

   /// Resolve an href (absolute path) or absolute URL against the base.
   ///
   /// # Errors
   ///
   /// Returns an error for malformed or cross-origin URLs.
   #[inline]
   pub fn url_for(&self, href: &str) -> Result<Url> {
      let url = self
         .base
         .join(href)
         .map_err(|err| DavError::Url(format!("{href}: {err}")))?;
      self.require_same_origin(&url)?;
      Ok(url)
   }

   fn same_origin(&self, url: &Url) -> bool {
      url.scheme() == self.base.scheme()
         && url.host_str() == self.base.host_str()
         && url.port_or_known_default() == self.base.port_or_known_default()
   }

   fn require_same_origin(&self, url: &Url) -> Result<()> {
      if !url.username().is_empty() || url.password().is_some() {
         return Err(DavError::Url(format!(
            "refusing DAV URL with embedded credentials: {url}"
         )));
      }
      if !self.same_origin(url) {
         return Err(DavError::Url(format!(
            "refusing cross-origin DAV request to {url}"
         )));
      }
      Ok(())
   }

   /// Issue `method` with optional body, following redirects manually.
   /// `headers` are `(name, value)` pairs applied to every hop.
   ///
   /// # Errors
   ///
   /// Returns the transport, URL, redirect, or response-size error.
   #[inline]
   pub async fn request(
      &self,
      method: &str,
      href: &str,
      headers: &[(&str, String)],
      body: Option<(&'static str, Vec<u8>)>,
   ) -> Result<DavHttpResponse> {
      let mut url = self.url_for(href)?;

      for _hop in 0..=MAX_REDIRECTS {
         let request_method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| DavError::Other(format!("invalid method {method}")))?;
         let mut req = self.http.request(request_method, url.clone());
         if let Some(auth) = self.creds.header_value() {
            req = req.header("Authorization", auth);
         }
         for &(name, ref value) in headers {
            req = req.header(name, value);
         }
         if let Some(&(content_type, ref bytes)) = body.as_ref() {
            req = req.header("Content-Type", content_type).body(bytes.clone());
         }
         let mut resp = req.send().await?;
         let status = resp.status().as_u16();

         if matches!(status, 301 | 302 | 307 | 308) {
            let location = resp
               .headers()
               .get("Location")
               .and_then(|header| header.to_str().ok())
               .map(str::to_owned)
               .ok_or_else(|| {
                  DavError::Status {
                     status,
                     method: method.to_owned(),
                     url: url.to_string(),
                  }
               })?;
            let redirected = url
               .join(&location)
               .map_err(|err| DavError::Url(format!("{location}: {err}")))?;
            // DAV request bodies contain private calendar/contact data.
            // Dropping only Authorization would still leak that body to a
            // hostile redirect target, so cross-origin redirects fail.
            self.require_same_origin(&redirected)?;
            url = redirected;
            continue;
         }

         let etag = resp
            .headers()
            .get("ETag")
            .and_then(|header| header.to_str().ok())
            .map(str::to_owned);
         let location = resp
            .headers()
            .get("Location")
            .and_then(|header| header.to_str().ok())
            .map(str::to_owned);
         if resp
            .content_length()
            .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
         {
            return Err(DavError::ResponseTooLarge {
               url:   url.to_string(),
               limit: MAX_RESPONSE_BYTES,
            });
         }

         let mut bytes = Vec::<u8>::new();
         while let Some(chunk) = resp.chunk().await? {
            if response_would_exceed_limit(bytes.len(), chunk.len()) {
               return Err(DavError::ResponseTooLarge {
                  url:   url.to_string(),
                  limit: MAX_RESPONSE_BYTES,
               });
            }
            bytes.extend_from_slice(&chunk);
         }
         return Ok(DavHttpResponse {
            status,
            etag,
            location,
            body: bytes,
         });
      }
      Err(DavError::TooManyRedirects {
         url: url.to_string(),
      })
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn debug_output_redacts_secrets() {
      let basic = Credentials::Basic {
         username: "ada".into(),
         password: "correct horse battery staple".into(),
      };
      let bearer = Credentials::Bearer {
         token: "top-secret-token".into(),
      };

      let output = format!("{basic:?} {bearer:?}");
      assert!(output.contains("ada"));
      assert!(output.contains("[REDACTED]"));
      assert!(!output.contains("correct horse battery staple"));
      assert!(!output.contains("top-secret-token"));
   }

   #[test]
   fn only_http_same_origin_urls_are_accepted() {
      assert!(DavConnection::new("ftp://dav.example.test/", Credentials::None).is_err());
      assert!(
         DavConnection::new("https://user:secret@dav.example.test/", Credentials::None).is_err()
      );

      let conn = DavConnection::new("https://dav.example.test/root/", Credentials::None)
         .expect("valid DAV origin");
      conn.url_for("/calendar/a.ics").unwrap();
      conn
         .url_for("https://dav.example.test/calendar/a.ics")
         .unwrap();
      conn
         .url_for("https://attacker.example/calendar/a.ics")
         .unwrap_err();
      conn
         .url_for("https://user:secret@dav.example.test/calendar/a.ics")
         .unwrap_err();
      conn
         .url_for("http://dav.example.test/calendar/a.ics")
         .unwrap_err();
   }

   #[test]
   fn response_limit_rejects_oversized_and_overflowing_chunks() {
      assert!(!response_would_exceed_limit(MAX_RESPONSE_BYTES - 1, 1));
      assert!(response_would_exceed_limit(MAX_RESPONSE_BYTES, 1));
      assert!(response_would_exceed_limit(usize::MAX, 1));
   }
}
