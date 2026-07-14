//! `CalDAV` policy above the DAV core: principal → calendar-home-set →
//! calendar collections, and the calendar-multiget REPORT (RFC 4791).

use crate::{
   dav::{
      client::{
         DavClient,
         MultigetResult,
      },
      href,
      xml::{
         DavResponse,
         NS_APPLE,
         NS_CALDAV,
         NS_CALSERVER,
         NS_DAV,
      },
   },
   error::{
      DavError,
      Result,
   },
};

/// A discovered calendar collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarCollection {
   pub href:          String,
   pub name:          String,
   pub color:         Option<String>,
   pub description:   Option<String>,
   /// Server advertises RFC 6578 sync-collection for this collection.
   pub supports_sync: bool,
}

/// Resolve the current user's principal href.
///
/// Starts from any URL on the server and falls back to that URL when the
/// principal property is absent.
///
/// # Errors
///
/// Returns the DAV error produced by the principal lookup.
#[inline]
pub async fn discover_principal(client: &DavClient, start_href: &str) -> Result<String> {
   let ms = client
      .propfind(start_href, "0", &[(NS_DAV, "current-user-principal")])
      .await?;
   for response in &ms.responses {
      if let Some(property) = response.prop(NS_DAV, "current-user-principal")
         && let Some(href) = property.hrefs.first()
      {
         return Ok(href.clone());
      }
   }
   Ok(href::normalize_href(start_href))
}

/// calendar-home-set for a principal (RFC 4791 §6.2.1).
///
/// # Errors
///
/// Returns the DAV error produced by the home-set lookup.
#[inline]
pub async fn discover_calendar_home(client: &DavClient, principal_href: &str) -> Result<String> {
   let ms = client
      .propfind(principal_href, "0", &[(NS_CALDAV, "calendar-home-set")])
      .await?;
   for response in &ms.responses {
      if let Some(property) = response.prop(NS_CALDAV, "calendar-home-set")
         && let Some(href) = property.hrefs.first()
      {
         return Ok(href.clone());
      }
   }
   Err(DavError::MissingProperty {
      what: "calendar-home-set",
      href: principal_href.to_owned(),
   })
}

/// Enumerate calendar collections under the home set.
///
/// # Errors
///
/// Returns the DAV error produced by the collection lookup.
#[inline]
pub async fn list_calendars(
   client: &DavClient,
   home_href: &str,
) -> Result<Vec<CalendarCollection>> {
   let ms = client
      .propfind(home_href, "1", &[
         (NS_DAV, "resourcetype"),
         (NS_DAV, "displayname"),
         (NS_DAV, "supported-report-set"),
         (NS_APPLE, "calendar-color"),
         (NS_CALDAV, "calendar-description"),
         (NS_CALSERVER, "getctag"),
      ])
      .await?;

   let mut out = Vec::<CalendarCollection>::new();
   for response in &ms.responses {
      if !response.has_resourcetype(NS_CALDAV, "calendar") {
         continue;
      }
      out.push(CalendarCollection {
         href:          response.href.clone(),
         name:          display_name(response),
         color:         response
            .prop(NS_APPLE, "calendar-color")
            .map(|property| property.text.trim().to_owned())
            .filter(|text| !text.is_empty()),
         description:   response
            .prop(NS_CALDAV, "calendar-description")
            .map(|property| property.text.trim().to_owned())
            .filter(|text| !text.is_empty()),
         supports_sync: supports_sync_collection(response),
      });
   }
   Ok(out)
}

/// calendar-multiget (RFC 4791 §7.9) for a chunk of member hrefs.
///
/// # Errors
///
/// Returns the DAV error produced by the multiget request.
#[inline]
pub async fn multiget(
   client: &DavClient,
   collection_href: &str,
   hrefs: &[String],
) -> Result<MultigetResult> {
   client
      .multiget(
         collection_href,
         (NS_CALDAV, "calendar-multiget"),
         (NS_CALDAV, "calendar-data"),
         hrefs,
      )
      .await
}

pub(crate) fn display_name(response: &DavResponse) -> String {
   response
      .prop(NS_DAV, "displayname")
      .map(|property| property.text.trim().to_owned())
      .filter(|text| !text.is_empty())
      .unwrap_or_else(|| href::last_segment(&response.href))
}

pub(crate) fn supports_sync_collection(response: &DavResponse) -> bool {
   response
      .prop(NS_DAV, "supported-report-set")
      .is_none_or(|property| {
         property.elements.iter().any(|element| {
            let &(ref namespace, ref name) = element;
            namespace == NS_DAV && name.eq_ignore_ascii_case("sync-collection")
         })
      })
}
