//! `CardDAV` policy above the DAV core: principal → addressbook-home-set →
//! address book collections, and addressbook-multiget (RFC 6352).

use crate::{
   caldav,
   dav::{
      client::{
         DavClient,
         MultigetResult,
      },
      xml::{
         NS_CARDDAV,
         NS_DAV,
      },
   },
   error::{
      DavError,
      Result,
   },
};

/// A discovered address book collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressBookCollection {
   pub href:          String,
   pub name:          String,
   pub description:   Option<String>,
   pub supports_sync: bool,
}

/// addressbook-home-set for a principal (RFC 6352 §7.1.1).
///
/// # Errors
///
/// Returns the DAV error produced by the home-set lookup.
#[inline]
pub async fn discover_addressbook_home(client: &DavClient, principal_href: &str) -> Result<String> {
   let ms = client
      .propfind(principal_href, "0", &[(NS_CARDDAV, "addressbook-home-set")])
      .await?;
   for response in &ms.responses {
      if let Some(property) = response.prop(NS_CARDDAV, "addressbook-home-set")
         && let Some(href) = property.hrefs.first()
      {
         return Ok(href.clone());
      }
   }
   Err(DavError::MissingProperty {
      what: "addressbook-home-set",
      href: principal_href.to_owned(),
   })
}

/// Enumerate address book collections under the home set.
///
/// # Errors
///
/// Returns the DAV error produced by the collection lookup.
#[inline]
pub async fn list_addressbooks(
   client: &DavClient,
   home_href: &str,
) -> Result<Vec<AddressBookCollection>> {
   let ms = client
      .propfind(home_href, "1", &[
         (NS_DAV, "resourcetype"),
         (NS_DAV, "displayname"),
         (NS_DAV, "supported-report-set"),
         (NS_CARDDAV, "addressbook-description"),
      ])
      .await?;

   let mut out = Vec::<AddressBookCollection>::new();
   for response in &ms.responses {
      if !response.has_resourcetype(NS_CARDDAV, "addressbook") {
         continue;
      }
      out.push(AddressBookCollection {
         href:          response.href.clone(),
         name:          caldav::display_name(response),
         description:   response
            .prop(NS_CARDDAV, "addressbook-description")
            .map(|property| property.text.trim().to_owned())
            .filter(|text| !text.is_empty()),
         supports_sync: caldav::supports_sync_collection(response),
      });
   }
   Ok(out)
}

/// addressbook-multiget (RFC 6352 §8.7) for a chunk of member hrefs.
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
         (NS_CARDDAV, "addressbook-multiget"),
         (NS_CARDDAV, "address-data"),
         hrefs,
      )
      .await
}
