//! Reusable, protocol-agnostic DAV operations.
//!
//! `CalDAV` and `CardDAV` policy is layered on top of these operations.

use std::{
   collections::BTreeSet,
   fmt::Write as _,
};

use crate::{
   dav::{
      href,
      xml::{
         self,
         MultiStatus,
         NS_DAV,
      },
   },
   error::{
      DavError,
      Result,
   },
   http::{
      Credentials,
      DavConnection,
   },
};

pub struct DavClient {
   conn: DavConnection,
}

/// One changed-or-present member reported by the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberEtag {
   pub href: String,
   pub etag: Option<String>,
}

/// Outcome of a sync-collection round (RFC 6578).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncCollection {
   pub changed:   Vec<MemberEtag>,
   pub removed:   Vec<String>,
   pub new_token: String,
}

/// Body + metadata of one fetched resource.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedResource {
   pub href: String,
   pub etag: Option<String>,
   pub data: String,
}

/// Complete result of one multiget REPORT. A resource may disappear after
/// the sync report but before the multiget; that race is represented as a
/// removal instead of silently advancing past it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultigetResult {
   pub fetched: Vec<FetchedResource>,
   pub removed: Vec<String>,
}

impl DavClient {
   /// # Errors
   ///
   /// Returns an error when the base URL cannot be constructed.
   #[inline]
   pub fn new(base_url: &str, creds: Credentials) -> Result<Self> {
      Ok(Self {
         conn: DavConnection::new(base_url, creds)?,
      })
   }

   #[must_use]
   #[inline]
   pub const fn conn(&self) -> &DavConnection {
      &self.conn
   }

   /// PROPFIND with the given `(namespace, name)` properties.
   ///
   /// # Errors
   ///
   /// Returns the DAV request or response parsing error.
   #[inline]
   pub async fn propfind(
      &self,
      href: &str,
      depth: &str,
      props: &[(&str, &str)],
   ) -> Result<MultiStatus> {
      let body = propfind_body(props);
      let resp = self
         .conn
         .request(
            "PROPFIND",
            href,
            &[("Depth", depth.to_owned())],
            Some(("application/xml; charset=utf-8", body.into_bytes())),
         )
         .await?;
      if resp.status != 207 {
         return Err(status_error("PROPFIND", href, resp.status));
      }
      xml::parse_multistatus(&resp.body)
   }

   /// Arbitrary REPORT with a prebuilt XML body.
   ///
   /// # Errors
   ///
   /// Returns the DAV request or response parsing error.
   #[inline]
   pub async fn report(&self, href: &str, depth: &str, body: String) -> Result<MultiStatus> {
      let resp = self
         .conn
         .request(
            "REPORT",
            href,
            &[("Depth", depth.to_owned())],
            Some(("application/xml; charset=utf-8", body.into_bytes())),
         )
         .await?;
      if resp.status != 207 {
         return Err(status_error("REPORT", href, resp.status));
      }
      xml::parse_multistatus(&resp.body)
   }

   /// RFC 6578 sync-collection. `token: None` requests the initial sync.
   /// A rejected token surfaces as [`DavError::SyncTokenInvalid`] so the
   /// engine can fall back to a full listing.
   ///
   /// # Errors
   ///
   /// Returns the DAV request, response parsing, or invalid-token error.
   #[inline]
   pub async fn sync_collection(
      &self,
      collection_href: &str,
      token: Option<&str>,
   ) -> Result<SyncCollection> {
      let token_xml = token.map_or_else(
         || "<D:sync-token/>".to_owned(),
         |token_value| {
            format!(
               "<D:sync-token>{}</D:sync-token>",
               xml::xml_escape(token_value)
            )
         },
      );
      let body = format!(
         r#"<?xml version="1.0" encoding="utf-8"?>
<D:sync-collection xmlns:D="DAV:">
  {token_xml}
  <D:sync-level>1</D:sync-level>
  <D:prop><D:getetag/></D:prop>
</D:sync-collection>"#
      );
      let resp = self
         .conn
         .request(
            "REPORT",
            collection_href,
            &[("Depth", "0".to_owned())],
            Some(("application/xml; charset=utf-8", body.into_bytes())),
         )
         .await?;
      if resp.status == 403 || resp.status == 409 || resp.status == 400 {
         if xml::is_sync_token_error(&resp.body) {
            return Err(DavError::SyncTokenInvalid {
               href: collection_href.to_owned(),
            });
         }
         return Err(status_error(
            "REPORT sync-collection",
            collection_href,
            resp.status,
         ));
      }
      if resp.status != 207 {
         return Err(status_error(
            "REPORT sync-collection",
            collection_href,
            resp.status,
         ));
      }
      let ms = xml::parse_multistatus(&resp.body)?;
      let new_token = ms.sync_token.clone().ok_or(DavError::MissingProperty {
         what: "sync-token",
         href: collection_href.to_owned(),
      })?;

      let collection = href::normalize_href(collection_href);
      let mut changed = Vec::<MemberEtag>::new();
      let mut removed = Vec::<String>::new();
      for response in &ms.responses {
         // The collection itself may appear in the result; skip it.
         if response.href == collection {
            continue;
         }
         if matches!(response.status, Some(404)) {
            removed.push(response.href.clone());
         } else {
            changed.push(MemberEtag {
               href: response.href.clone(),
               etag: response.etag().map(str::to_owned),
            });
         }
      }
      Ok(SyncCollection {
         changed,
         removed,
         new_token,
      })
   }

   /// Fallback listing for servers without sync-collection: PROPFIND
   /// Depth:1 for getetag. Non-collection members only.
   ///
   /// # Errors
   ///
   /// Returns the DAV request or response parsing error.
   #[inline]
   pub async fn list_etags(&self, collection_href: &str) -> Result<Vec<MemberEtag>> {
      let ms = self
         .propfind(collection_href, "1", &[
            (NS_DAV, "getetag"),
            (NS_DAV, "resourcetype"),
         ])
         .await?;
      let collection = href::normalize_href(collection_href);
      Ok(ms
         .responses
         .iter()
         .filter(|response| response.href != collection)
         .filter(|response| !response.has_resourcetype(NS_DAV, "collection"))
         .map(|response| {
            MemberEtag {
               href: response.href.clone(),
               etag: response.etag().map(str::to_owned),
            }
         })
         .collect())
   }

   /// Multiget REPORT: fetch `data_prop` (calendar-data / address-data)
   /// for a set of member hrefs in one round trip.
   ///
   /// # Errors
   ///
   /// Returns the DAV request or response validation error.
   #[inline]
   pub async fn multiget(
      &self,
      collection_href: &str,
      report: (&str, &str),
      data_prop: (&str, &str),
      hrefs: &[String],
   ) -> Result<MultigetResult> {
      let (report_ns, report_name) = report;
      let (data_ns, data_name) = data_prop;
      let mut body = format!(
         r#"<?xml version="1.0" encoding="utf-8"?>
<R:{report_name} xmlns:D="DAV:" xmlns:R="{report_ns}">
  <D:prop><D:getetag/><R:{data_name} xmlns:R="{data_ns}"/></D:prop>
"#
      );
      for href in hrefs {
         writeln!(body, "  <D:href>{}</D:href>", xml::xml_escape(href)).unwrap();
      }
      writeln!(body, "</R:{report_name}>").unwrap();

      let ms = self.report(collection_href, "1", body).await?;
      let requested = hrefs
         .iter()
         .map(|href| href::normalize_href(href))
         .collect::<BTreeSet<String>>();
      let mut accounted = BTreeSet::<String>::new();
      let mut fetched = Vec::<FetchedResource>::new();
      let mut removed = Vec::<String>::new();
      for response in &ms.responses {
         if !requested.contains(&response.href) {
            continue;
         }
         if !accounted.insert(response.href.clone()) {
            return Err(DavError::Other(format!(
               "multiget returned duplicate response for {}",
               response.href
            )));
         }

         let data = response.prop(data_ns, data_name);
         let not_found = matches!(response.status, Some(404))
            || (data.is_none() && response.propstats.iter().any(|status| status.status == 404));
         if not_found {
            removed.push(response.href.clone());
            continue;
         }
         if let Some(status) = response
            .status
            .filter(|status| !(200..300).contains(status))
         {
            return Err(status_error(
               "REPORT multiget member",
               &response.href,
               status,
            ));
         }
         let data = data.ok_or_else(|| {
            DavError::Other(format!(
               "multiget response for {} is missing {data_name}",
               response.href
            ))
         })?;
         fetched.push(FetchedResource {
            href: response.href.clone(),
            etag: response.etag().map(str::to_owned),
            data: data.text.clone(),
         });
      }
      if let Some(missing) = requested.difference(&accounted).next() {
         return Err(DavError::Other(format!(
            "multiget omitted requested resource {missing}"
         )));
      }
      Ok(MultigetResult { fetched, removed })
   }

   /// # Errors
   ///
   /// Returns the DAV request or response validation error.
   #[inline]
   pub async fn get(&self, href: &str) -> Result<FetchedResource> {
      let resp = self.conn.request("GET", href, &[], None).await?;
      if !(200..300).contains(&resp.status) {
         return Err(status_error("GET", href, resp.status));
      }
      Ok(FetchedResource {
         href: href::normalize_href(href),
         etag: resp.etag,
         data: String::from_utf8_lossy(&resp.body).into_owned(),
      })
   }

   /// PUT with optimistic concurrency: `if_match: Some(etag)` guards an
   /// update, `if_match: None` sends `If-None-Match: *` so a create can't
   /// clobber a resource that appeared meanwhile. Returns the new `ETag`
   /// when the server provides one.
   ///
   /// # Errors
   ///
   /// Returns the DAV request or concurrency error.
   #[inline]
   pub async fn put(
      &self,
      href: &str,
      content_type: &'static str,
      body: String,
      if_match: Option<&str>,
   ) -> Result<Option<String>> {
      let mut headers = Vec::<(&str, String)>::new();
      match if_match {
         Some(etag) => headers.push(("If-Match", etag.to_owned())),
         None => headers.push(("If-None-Match", "*".to_owned())),
      }
      let resp = self
         .conn
         .request(
            "PUT",
            href,
            &headers,
            Some((content_type, body.into_bytes())),
         )
         .await?;
      if resp.status == 412 {
         return Err(DavError::PreconditionFailed {
            url: href.to_owned(),
         });
      }
      if !(200..300).contains(&resp.status) {
         return Err(status_error("PUT", href, resp.status));
      }
      Ok(resp.etag)
   }

   /// # Errors
   ///
   /// Returns the DAV request or concurrency error.
   #[inline]
   pub async fn delete(&self, href: &str, if_match: Option<&str>) -> Result<()> {
      let mut headers = Vec::<(&str, String)>::new();
      if let Some(etag) = if_match {
         headers.push(("If-Match", etag.to_owned()));
      }
      let resp = self.conn.request("DELETE", href, &headers, None).await?;
      if resp.status == 412 {
         return Err(DavError::PreconditionFailed {
            url: href.to_owned(),
         });
      }
      // 404 on DELETE is success-shaped: the resource is gone either way.
      if !(200..300).contains(&resp.status) && resp.status != 404 {
         return Err(status_error("DELETE", href, resp.status));
      }
      Ok(())
   }

   /// MOVE to a destination href on the same origin.
   ///
   /// # Errors
   ///
   /// Returns the DAV request, URL, or concurrency error.
   #[inline]
   pub async fn move_(&self, src_href: &str, dest_href: &str) -> Result<()> {
      let dest = self.conn.url_for(dest_href)?;
      let resp = self
         .conn
         .request(
            "MOVE",
            src_href,
            &[
               ("Destination", dest.to_string()),
               ("Overwrite", "F".to_owned()),
            ],
            None,
         )
         .await?;
      if resp.status == 412 {
         return Err(DavError::PreconditionFailed {
            url: src_href.to_owned(),
         });
      }
      if !(200..300).contains(&resp.status) {
         return Err(status_error("MOVE", src_href, resp.status));
      }
      Ok(())
   }
}

fn status_error(method: &str, href: &str, status: u16) -> DavError {
   DavError::Status {
      status,
      method: method.to_owned(),
      url: href.to_owned(),
   }
}

fn propfind_body(props: &[(&str, &str)]) -> String {
   let mut body = String::from(
      "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<D:propfind xmlns:D=\"DAV:\">\n  <D:prop>\n",
   );
   for &(namespace, name) in props {
      if namespace == NS_DAV {
         writeln!(body, "    <D:{name}/>").unwrap();
      } else {
         writeln!(body, "    <P:{name} xmlns:P=\"{namespace}\"/>").unwrap();
      }
   }
   body.push_str("  </D:prop>\n</D:propfind>\n");
   body
}
