//! 207 Multi-Status parsing (RFC 4918 §13) with real namespace resolution.
//!
//! The property model is deliberately generic: every `<prop>` child keeps
//! its resolved `(namespace, localname)`, its concatenated text, any
//! descendant `<DAV:href>` values, and the names of its descendant
//! elements. That one shape covers getetag (text), resourcetype (element
//! names), current-user-principal / home-set (hrefs), supported-report-set
//! (element names), and calendar-data / address-data (text) without a
//! per-property parser zoo.

use quick_xml::{
   NsReader,
   XmlVersion,
   events::{
      BytesEnd,
      BytesStart,
      Event,
   },
   name::ResolveResult,
};

use crate::{
   dav::href,
   error::{
      DavError,
      Result,
   },
};

pub const NS_DAV: &str = "DAV:";
pub const NS_CALDAV: &str = "urn:ietf:params:xml:ns:caldav";
pub const NS_CARDDAV: &str = "urn:ietf:params:xml:ns:carddav";
pub const NS_CALSERVER: &str = "http://calendarserver.org/ns/";
pub const NS_APPLE: &str = "http://apple.com/ns/ical/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiStatus {
   pub responses:  Vec<DavResponse>,
   /// RFC 6578 top-level `<sync-token>`, when the body was a
   /// sync-collection REPORT result.
   pub sync_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DavResponse {
   /// Normalized href (see [`crate::dav::href`]).
   pub href:      String,
   /// Per-resource `<status>` outside any propstat (sync-collection uses
   /// this to report removals as 404).
   pub status:    Option<u16>,
   pub propstats: Vec<Propstat>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Propstat {
   pub status: u16,
   pub props:  Vec<DavProp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DavProp {
   pub ns:       String,
   pub name:     String,
   /// Concatenated character data of the property element.
   pub text:     String,
   /// Normalized values of any descendant `DAV:href` elements.
   pub hrefs:    Vec<String>,
   /// `(ns, localname)` of descendant elements (e.g. resourcetype's
   /// `<collection/><C:calendar/>`, supported-report-set's report names).
   pub elements: Vec<(String, String)>,
}

impl DavResponse {
   /// The 2xx propstat's property by namespace+name.
   #[must_use]
   #[inline]
   pub fn prop(&self, ns: &str, name: &str) -> Option<&DavProp> {
      self
         .propstats
         .iter()
         .filter(|ps| (200..300).contains(&ps.status))
         .flat_map(|ps| ps.props.iter())
         .find(|property| property.ns == ns && property.name.eq_ignore_ascii_case(name))
   }

   #[must_use]
   #[inline]
   pub fn etag(&self) -> Option<&str> {
      self
         .prop(NS_DAV, "getetag")
         .map(|property| property.text.trim())
   }

   /// True when the 2xx resourcetype includes `(ns, name)`.
   #[must_use]
   #[inline]
   pub fn has_resourcetype(&self, ns: &str, name: &str) -> bool {
      self.prop(NS_DAV, "resourcetype").is_some_and(|property| {
         property.elements.iter().any(|element| {
            let &(ref namespace, ref element_name) = element;
            namespace == ns && element_name.eq_ignore_ascii_case(name)
         })
      })
   }
}

/// Parse a Multi-Status body. Malformed XML is a typed error; unexpected
/// elements are skipped, unknown namespaces preserved.
///
/// # Errors
///
/// Returns an error when the XML is malformed or contains an invalid status.
#[inline]
pub fn parse_multistatus(body: &[u8]) -> Result<MultiStatus> {
   // No trim_text: calendar-data/address-data payloads are text nodes
   // whose leading/trailing whitespace is significant (final CRLF).
   // Whitespace-insignificant values (hrefs, tokens, status lines) are
   // trimmed individually where they are read.
   let mut reader = NsReader::from_reader(body);

   let mut responses = Vec::<DavResponse>::new();
   let mut sync_token = None::<String>;
   let mut saw_multistatus = false;
   let mut buf = Vec::<u8>::new();

   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(event)) => {
            let (ns, local) = resolve(&reader, &event);
            if ns == NS_DAV && local == "multistatus" {
               saw_multistatus = true;
            } else if ns == NS_DAV && local == "response" {
               responses.push(parse_response(&mut reader)?);
            } else if ns == NS_DAV && local == "sync-token" {
               sync_token = Some(read_text(&mut reader)?.trim().to_owned());
            }
         },
         Ok(Event::Eof) => break,
         Ok(_) => {},
         Err(error) => return Err(DavError::Xml(error.to_string())),
      }
   }
   if !saw_multistatus {
      return Err(DavError::Xml("body is not a DAV:multistatus".into()));
   }
   Ok(MultiStatus {
      responses,
      sync_token,
   })
}

fn parse_response(reader: &mut NsReader<&[u8]>) -> Result<DavResponse> {
   let mut href = String::new();
   let mut status = None::<u16>;
   let mut propstats = Vec::<Propstat>::new();
   let mut buf = Vec::<u8>::new();

   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(event)) => {
            let (ns, local) = resolve(reader, &event);
            match (ns.as_str(), local.as_str()) {
               (NS_DAV, "href") if href.is_empty() => {
                  href = href::normalize_href(read_text(reader)?.trim());
               },
               (NS_DAV, "status") => {
                  status = parse_status_line(&read_text(reader)?);
               },
               (NS_DAV, "propstat") => {
                  propstats.push(parse_propstat(reader)?);
               },
               _ => skip_element(reader)?,
            }
         },
         Ok(Event::End(event)) => {
            let (ns, local) = resolve_end(reader, &event);
            if ns == NS_DAV && local == "response" {
               break;
            }
         },
         Ok(Event::Eof) => {
            return Err(DavError::Xml("truncated <response>".into()));
         },
         Ok(_) => {},
         Err(error) => return Err(DavError::Xml(error.to_string())),
      }
   }
   Ok(DavResponse {
      href,
      status,
      propstats,
   })
}

fn parse_propstat(reader: &mut NsReader<&[u8]>) -> Result<Propstat> {
   let mut status = 0_u16;
   let mut props = Vec::<DavProp>::new();
   let mut buf = Vec::<u8>::new();

   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(event)) => {
            let (ns, local) = resolve(reader, &event);
            match (ns.as_str(), local.as_str()) {
               (NS_DAV, "status") => {
                  status = parse_status_line(&read_text(reader)?).unwrap_or(0);
               },
               (NS_DAV, "prop") => {
                  parse_props(reader, &mut props)?;
               },
               _ => skip_element(reader)?,
            }
         },
         Ok(Event::End(event)) => {
            let (ns, local) = resolve_end(reader, &event);
            if ns == NS_DAV && local == "propstat" {
               break;
            }
         },
         Ok(Event::Eof) => {
            return Err(DavError::Xml("truncated <propstat>".into()));
         },
         Ok(_) => {},
         Err(error) => return Err(DavError::Xml(error.to_string())),
      }
   }
   Ok(Propstat { status, props })
}

/// Children of `<prop>`: one [`DavProp`] each, capturing text, hrefs, and
/// descendant element names until the matching end tag.
fn parse_props(reader: &mut NsReader<&[u8]>, out: &mut Vec<DavProp>) -> Result<()> {
   let mut buf = Vec::<u8>::new();
   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(event)) => {
            let (ns, name) = resolve(reader, &event);
            out.push(parse_prop_value(reader, ns, name)?);
         },
         Ok(Event::Empty(event)) => {
            let (ns, name) = resolve(reader, &event);
            out.push(DavProp {
               ns,
               name,
               text: String::new(),
               hrefs: Vec::new(),
               elements: Vec::new(),
            });
         },
         Ok(Event::End(event)) => {
            let (ns, local) = resolve_end(reader, &event);
            if ns == NS_DAV && local == "prop" {
               return Ok(());
            }
         },
         Ok(Event::Eof) => return Err(DavError::Xml("truncated <prop>".into())),
         Ok(_) => {},
         Err(error) => return Err(DavError::Xml(error.to_string())),
      }
   }
}

fn parse_prop_value(reader: &mut NsReader<&[u8]>, ns: String, name: String) -> Result<DavProp> {
   let mut prop = DavProp {
      ns,
      name,
      text: String::new(),
      hrefs: Vec::new(),
      elements: Vec::new(),
   };
   let mut depth = 0_i32;
   // In-progress <D:href> content: text may arrive in several events
   // (entity references split text nodes), so accumulate until the end
   // tag rather than pushing per event.
   let mut href_buf = None::<String>;
   let mut buf = Vec::<u8>::new();
   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(event)) => {
            let (ens, elocal) = resolve(reader, &event);
            if ens == NS_DAV && elocal == "href" {
               href_buf = Some(String::new());
            }
            prop.elements.push((ens, elocal));
            depth += 1;
         },
         Ok(Event::Empty(event)) => {
            let (ens, elocal) = resolve(reader, &event);
            prop.elements.push((ens, elocal));
         },
         Ok(Event::Text(event)) => {
            let text = event
               .xml_content(XmlVersion::Implicit1_0)
               .map_err(|error| DavError::Xml(error.to_string()))?;
            href_buf.as_mut().map_or_else(
               || prop.text.push_str(&text),
               |href_text| href_text.push_str(&text),
            );
         },
         Ok(Event::GeneralRef(event)) => {
            let text = resolve_ref(&event)?;
            href_buf.as_mut().map_or_else(
               || prop.text.push_str(&text),
               |href_text| href_text.push_str(&text),
            );
         },
         Ok(Event::CData(event)) => {
            let text = String::from_utf8_lossy(event.into_inner().as_ref()).into_owned();
            href_buf.as_mut().map_or_else(
               || prop.text.push_str(&text),
               |href_text| href_text.push_str(&text),
            );
         },
         Ok(Event::End(_)) => {
            if let Some(href_text) = href_buf.take() {
               prop.hrefs.push(href::normalize_href(href_text.trim()));
               depth -= 1;
               continue;
            }
            if depth == 0 {
               return Ok(prop);
            }
            depth -= 1;
         },
         Ok(Event::Eof) => return Err(DavError::Xml("truncated property".into())),
         Ok(_) => {},
         Err(error) => return Err(DavError::Xml(error.to_string())),
      }
   }
}

/// Resolve a character or predefined general entity reference.
fn resolve_ref(reference: &quick_xml::events::BytesRef) -> Result<String> {
   if let Some(character) = reference
      .resolve_char_ref()
      .map_err(|error| DavError::Xml(error.to_string()))?
   {
      return Ok(character.to_string());
   }
   let name = reference
      .decode()
      .map_err(|error| DavError::Xml(error.to_string()))?;
   Ok(match name.as_ref() {
      "lt" => "<".to_owned(),
      "gt" => ">".to_owned(),
      "amp" => "&".to_owned(),
      "apos" => "'".to_owned(),
      "quot" => "\"".to_owned(),
      other => return Err(DavError::Xml(format!("unknown entity &{other};"))),
   })
}

/// Detect the RFC 6578 `valid-sync-token` precondition in an error body.
#[must_use]
#[inline]
pub fn is_sync_token_error(body: &[u8]) -> bool {
   let mut reader = NsReader::from_reader(body);
   let mut buf = Vec::<u8>::new();
   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(event) | Event::Empty(event)) => {
            let (ns, local) = resolve(&reader, &event);
            if ns == NS_DAV && local == "valid-sync-token" {
               return true;
            }
         },
         Ok(Event::Eof) | Err(_) => return false,
         Ok(_) => {},
      }
   }
}

/// Read text content of the current element and consume its end tag.
fn read_text(reader: &mut NsReader<&[u8]>) -> Result<String> {
   let mut text = String::new();
   let mut depth = 0_i32;
   let mut buf = Vec::<u8>::new();
   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(_)) => depth += 1,
         Ok(Event::Text(event)) => {
            text.push_str(
               &event
                  .xml_content(XmlVersion::Implicit1_0)
                  .map_err(|error| DavError::Xml(error.to_string()))?,
            );
         },
         Ok(Event::GeneralRef(event)) => text.push_str(&resolve_ref(&event)?),
         Ok(Event::CData(event)) => {
            text.push_str(&String::from_utf8_lossy(event.into_inner().as_ref()));
         },
         Ok(Event::End(_)) => {
            if depth == 0 {
               return Ok(text);
            }
            depth -= 1;
         },
         Ok(Event::Eof) => return Err(DavError::Xml("truncated element".into())),
         Ok(_) => {},
         Err(error) => return Err(DavError::Xml(error.to_string())),
      }
   }
}

fn skip_element(reader: &mut NsReader<&[u8]>) -> Result<()> {
   let mut depth = 0_i32;
   let mut buf = Vec::<u8>::new();
   loop {
      buf.clear();
      match reader.read_event_into(&mut buf) {
         Ok(Event::Start(_)) => depth += 1,
         Ok(Event::End(_)) => {
            if depth == 0 {
               return Ok(());
            }
            depth -= 1;
         },
         Ok(Event::Eof) => return Err(DavError::Xml("truncated element".into())),
         Ok(_) => {},
         Err(error) => return Err(DavError::Xml(error.to_string())),
      }
   }
}

/// `HTTP/1.1 200 OK` → 200.
fn parse_status_line(line: &str) -> Option<u16> {
   line.split_whitespace().nth(1)?.parse::<u16>().ok()
}

fn resolve(reader: &NsReader<&[u8]>, event: &BytesStart<'_>) -> (String, String) {
   let (res, local) = reader.resolver().resolve_element(event.name());
   let ns = match res {
      ResolveResult::Bound(ns) => String::from_utf8_lossy(ns.as_ref()).into_owned(),
      _ => String::new(),
   };
   (ns, String::from_utf8_lossy(local.as_ref()).into_owned())
}

fn resolve_end(reader: &NsReader<&[u8]>, event: &BytesEnd<'_>) -> (String, String) {
   let (res, local) = reader.resolver().resolve_element(event.name());
   let ns = match res {
      ResolveResult::Bound(ns) => String::from_utf8_lossy(ns.as_ref()).into_owned(),
      _ => String::new(),
   };
   (ns, String::from_utf8_lossy(local.as_ref()).into_owned())
}

/// Minimal XML text escaping for request bodies we build by hand.
#[must_use]
#[inline]
pub fn xml_escape(value: &str) -> String {
   value
      .replace('&', "&amp;")
      .replace('<', "&lt;")
      .replace('>', "&gt;")
      .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;

   use super::*;

   const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<D:multistatus xmlns:D="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav" xmlns:x1="http://apple.com/ns/ical/">
  <D:response>
    <D:href>https://dav.example.com/cal/alice/work%20stuff/</D:href>
    <D:propstat>
      <D:prop>
        <D:displayname>Work</D:displayname>
        <D:resourcetype><D:collection/><C:calendar/></D:resourcetype>
        <x1:calendar-color>#FF0000FF</x1:calendar-color>
        <D:getetag>"abc123"</D:getetag>
      </D:prop>
      <D:status>HTTP/1.1 200 OK</D:status>
    </D:propstat>
    <D:propstat>
      <D:prop><D:getcontentlength/></D:prop>
      <D:status>HTTP/1.1 404 Not Found</D:status>
    </D:propstat>
  </D:response>
</D:multistatus>"#;

   #[test]
   fn parses_propfind_multistatus() {
      let ms = parse_multistatus(PROPFIND_BODY.as_bytes()).unwrap();
      assert_eq!(ms.responses.len(), 1);
      let resp = &ms.responses[0];
      assert_eq!(resp.href, "/cal/alice/work%20stuff/");
      assert_eq!(resp.prop(NS_DAV, "displayname").unwrap().text, "Work");
      assert!(resp.has_resourcetype(NS_CALDAV, "calendar"));
      assert!(resp.has_resourcetype(NS_DAV, "collection"));
      assert_eq!(resp.etag(), Some("\"abc123\""));
      assert_eq!(
         resp.prop(NS_APPLE, "calendar-color").unwrap().text,
         "#FF0000FF"
      );
      // The 404 propstat must not leak into 2xx lookups.
      assert!(resp.prop(NS_DAV, "getcontentlength").is_none());
   }

   #[test]
   fn parses_sync_collection_report() {
      let body = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/cal/alice/e1.ics</d:href>
    <d:propstat>
      <d:prop><d:getetag>"e1"</d:getetag></d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/cal/alice/gone.ics</d:href>
    <d:status>HTTP/1.1 404 Not Found</d:status>
  </d:response>
  <d:sync-token>http://example.com/sync/42</d:sync-token>
</d:multistatus>"#;
      let ms = parse_multistatus(body.as_bytes()).unwrap();
      assert_eq!(ms.sync_token.as_deref(), Some("http://example.com/sync/42"));
      assert_eq!(ms.responses[0].etag(), Some("\"e1\""));
      assert_eq!(ms.responses[1].status, Some(404));
      assert_eq!(ms.responses[1].href, "/cal/alice/gone.ics");
   }

   #[test]
   fn parses_href_valued_props() {
      let body = r#"<?xml version="1.0"?>
<multistatus xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav">
  <response>
    <href>/principals/alice/</href>
    <propstat>
      <prop>
        <current-user-principal><href>/principals/alice/</href></current-user-principal>
        <C:calendar-home-set><href>/cal/alice/</href></C:calendar-home-set>
      </prop>
      <status>HTTP/1.1 200 OK</status>
    </propstat>
  </response>
</multistatus>"#;
      let ms = parse_multistatus(body.as_bytes()).unwrap();
      let resp = &ms.responses[0];
      assert_eq!(
         resp.prop(NS_DAV, "current-user-principal").unwrap().hrefs,
         vec!["/principals/alice/".to_owned()]
      );
      assert_eq!(
         resp.prop(NS_CALDAV, "calendar-home-set").unwrap().hrefs,
         vec!["/cal/alice/".to_owned()]
      );
   }

   #[test]
   fn calendar_data_text_is_preserved() {
      let body = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:response>
    <d:href>/cal/a/e.ics</d:href>
    <d:propstat>
      <d:prop>
        <d:getetag>"v1"</d:getetag>
        <c:calendar-data>BEGIN:VCALENDAR
VERSION:2.0
END:VCALENDAR
</c:calendar-data>
      </d:prop>
      <d:status>HTTP/1.1 200 OK</d:status>
    </d:propstat>
  </d:response>
</d:multistatus>"#;
      let ms = parse_multistatus(body.as_bytes()).unwrap();
      let data = &ms.responses[0]
         .prop(NS_CALDAV, "calendar-data")
         .unwrap()
         .text;
      assert!(data.contains("BEGIN:VCALENDAR"));
      assert!(data.contains("END:VCALENDAR"));
   }

   #[test]
   fn malformed_xml_is_a_typed_error() {
      let err = parse_multistatus(b"<d:multistatus xmlns:d=\"DAV:\"><d:response>").unwrap_err();
      assert!(matches!(err, DavError::Xml(_)), "{err:?}");
      let err = parse_multistatus(b"not xml at all <<<").unwrap_err();
      assert!(matches!(err, DavError::Xml(_)), "{err:?}");
   }

   #[test]
   fn detects_valid_sync_token_error() {
      let body = br#"<?xml version="1.0"?>
<d:error xmlns:d="DAV:"><d:valid-sync-token/></d:error>"#;
      assert!(is_sync_token_error(body));
      assert!(!is_sync_token_error(br#"<d:error xmlns:d="DAV:"/>"#));
   }
}
