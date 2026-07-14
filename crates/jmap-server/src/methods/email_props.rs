//! `bodyStructure` and `header:*` properties for Email/get (RFC 8621 §4.1).
//!
//! Both materialize from the cached raw RFC 5322 bytes: bodyStructure walks
//! mail-parser's flattened part list back into the spec's tree; header forms
//! re-parse the single raw value through a synthetic one-header message so
//! the decoding path (RFC 2047, address grammar, msg-id grammar, date) is
//! always mail-parser's, never a second hand-rolled one.

use std::str;

use jmap_protocol::{
   email::{
      EmailBodyPart,
      EmailHeader,
   },
   ids::Id,
};
use mail_parser::{
   Message,
   MessageParser,
   MimeHeaders as _,
   PartType,
};

pub(crate) const DEFAULT_BODY_PROPERTIES: &[&str] = &[
   "partId",
   "blobId",
   "size",
   "name",
   "type",
   "charset",
   "disposition",
   "cid",
   "language",
   "location",
];
pub(crate) const BODY_PROPERTIES: &[&str] = &[
   "partId",
   "blobId",
   "size",
   "name",
   "type",
   "charset",
   "disposition",
   "cid",
   "language",
   "location",
   "headers",
   "subParts",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeaderForm {
   Raw,
   Text,
   Addresses,
   GroupedAddresses,
   MessageIds,
   Date,
   Urls,
}

#[derive(Debug, Clone)]
pub(crate) struct HeaderProp {
   /// The exact property key requested — the response key must match it.
   pub key:  String,
   pub name: String,
   pub form: HeaderForm,
   pub all:  bool,
}

/// Parse `header:{name}`, `header:{name}:as{Form}`,
/// `header:{name}[:as{Form}]:all`. Returns None for malformed shapes — the
/// caller rejects those loudly.
pub(crate) fn parse_header_prop(key: &str) -> Option<HeaderProp> {
   let rest = key.strip_prefix("header:")?;
   let mut parts = rest.split(':').collect::<Vec<&str>>();
   if parts.is_empty()
      || parts[0].is_empty()
      || !parts[0]
         .bytes()
         .all(|byte| byte.is_ascii_graphic() && byte != b':')
   {
      return None;
   }
   let all = if parts.last() == Some(&"all") {
      parts.pop();
      true
   } else {
      false
   };
   let form = match parts.get(1) {
      None | Some(&"asRaw") => HeaderForm::Raw,
      Some(&"asText") => HeaderForm::Text,
      Some(&"asAddresses") => HeaderForm::Addresses,
      Some(&"asGroupedAddresses") => HeaderForm::GroupedAddresses,
      Some(&"asMessageIds") => HeaderForm::MessageIds,
      Some(&"asDate") => HeaderForm::Date,
      Some(&"asURLs") => HeaderForm::Urls,
      Some(_) => return None,
   };
   if parts.len() > 2 {
      return None;
   }
   if !header_form_is_allowed(parts[0], form) {
      return None;
   }
   Some(HeaderProp {
      key: key.to_owned(),
      name: parts[0].to_owned(),
      form,
      all,
   })
}

fn header_form_is_allowed(name: &str, form: HeaderForm) -> bool {
   if form == HeaderForm::Raw {
      return true;
   }
   let name = name.to_ascii_lowercase();
   let known = matches!(
      name.as_str(),
      "date"
         | "from"
         | "sender"
         | "reply-to"
         | "to"
         | "cc"
         | "bcc"
         | "message-id"
         | "in-reply-to"
         | "references"
         | "subject"
         | "comments"
         | "keywords"
         | "resent-date"
         | "resent-from"
         | "resent-sender"
         | "resent-reply-to"
         | "resent-to"
         | "resent-cc"
         | "resent-bcc"
         | "resent-message-id"
         | "return-path"
         | "received"
         | "list-help"
         | "list-unsubscribe"
         | "list-subscribe"
         | "list-post"
         | "list-owner"
         | "list-archive"
   );
   if !known {
      return true;
   }
   match form {
      HeaderForm::Raw => true,
      HeaderForm::Text => matches!(name.as_str(), "subject" | "comments" | "keywords"),
      HeaderForm::Addresses | HeaderForm::GroupedAddresses => {
         matches!(
            name.as_str(),
            "from"
               | "sender"
               | "reply-to"
               | "to"
               | "cc"
               | "bcc"
               | "resent-from"
               | "resent-sender"
               | "resent-reply-to"
               | "resent-to"
               | "resent-cc"
               | "resent-bcc"
         )
      },
      HeaderForm::MessageIds => {
         matches!(
            name.as_str(),
            "message-id" | "in-reply-to" | "references" | "resent-message-id"
         )
      },
      HeaderForm::Date => matches!(name.as_str(), "date" | "resent-date"),
      HeaderForm::Urls => {
         matches!(
            name.as_str(),
            "list-help"
               | "list-unsubscribe"
               | "list-subscribe"
               | "list-post"
               | "list-owner"
               | "list-archive"
         )
      },
   }
}

pub(crate) fn validate_body_properties(properties: Option<&[String]>) -> Option<&str> {
   properties?
      .iter()
      .find(|property| {
         !BODY_PROPERTIES.contains(&property.as_str())
            && (!property.starts_with("header:") || parse_header_prop(property).is_none())
      })
      .map(String::as_str)
}

pub(crate) fn requested_body_properties(properties: Option<&[String]>) -> Vec<String> {
   properties.map_or_else(
      || {
         DEFAULT_BODY_PROPERTIES
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
      },
      <[String]>::to_vec,
   )
}

pub(crate) fn project_body_properties(value: &mut serde_json::Value, properties: &[String]) {
   match value {
      serde_json::Value::Array(values) => {
         for value in values {
            project_body_properties(value, properties);
         }
      },
      serde_json::Value::Object(object) => {
         if let Some(sub_parts) = object.get_mut("subParts") {
            project_body_properties(sub_parts, properties);
         }
         let has_sub_parts = object
            .get("subParts")
            .is_some_and(|sub_parts| !sub_parts.is_null());
         for property in properties
            .iter()
            .filter(|property| property.starts_with("header:"))
         {
            let header = parse_header_prop(property).expect("body properties were validated");
            let value = object
               .get("headers")
               .and_then(serde_json::Value::as_array)
               .map_or_else(
                  || {
                     if header.all {
                        serde_json::Value::Array(vec![])
                     } else {
                        serde_json::Value::Null
                     }
                  },
                  |headers| header_value_from_json(headers, &header),
               );
            object.insert(property.clone(), value);
         }
         object.retain(|property, _| {
            properties.iter().any(|wanted| wanted == property)
               || (property == "subParts" && has_sub_parts)
         });
         for property in properties {
            object
               .entry(property.clone())
               .or_insert(serde_json::Value::Null);
         }
      },
      _ => {},
   }
}

fn header_value_from_json(
   headers: &[serde_json::Value],
   property: &HeaderProp,
) -> serde_json::Value {
   let values = headers
      .iter()
      .filter(|header| {
         header
            .get("name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|name| name.eq_ignore_ascii_case(&property.name))
      })
      .filter_map(|header| header.get("value").and_then(serde_json::Value::as_str))
      .collect::<Vec<_>>();
   if property.all {
      serde_json::Value::Array(
         values
            .iter()
            .map(|value| form_value(value, property.form))
            .collect::<Vec<_>>(),
      )
   } else {
      values.last().map_or(serde_json::Value::Null, |value| {
         form_value(value, property.form)
      })
   }
}

pub(crate) fn raw_headers(
   message: &Message<'_>,
   headers: &[mail_parser::Header<'_>],
) -> Vec<EmailHeader> {
   headers
      .iter()
      .map(|header| {
         EmailHeader {
            name:  header.name().to_owned(),
            value: usize::try_from(header.offset_start())
               .ok()
               .zip(usize::try_from(header.offset_end()).ok())
               .and_then(|(start, end)| message.raw_message.get(start..end))
               .and_then(|bytes| str::from_utf8(bytes).ok())
               .unwrap_or_default()
               .trim_end_matches(['\r', '\n'])
               .to_owned(),
         }
      })
      .collect::<Vec<_>>()
}

/// The value for one requested header property (RFC 8621 §4.1.2/4.1.3):
/// without `:all`, the *last* instance of the header (or null); with `:all`,
/// every instance in message order.
pub(crate) fn header_value(parsed: &Message<'_>, prop: &HeaderProp) -> serde_json::Value {
   let raws = parsed
      .headers_raw()
      .filter(|(name, _)| name.eq_ignore_ascii_case(&prop.name))
      .map(|(_, raw)| raw.trim_end_matches(['\r', '\n']))
      .collect::<Vec<&str>>();
   if prop.all {
      serde_json::Value::Array(raws.iter().map(|raw| form_value(raw, prop.form)).collect())
   } else {
      raws
         .last()
         .map_or(serde_json::Value::Null, |raw| form_value(raw, prop.form))
   }
}

/// Decode one raw header value into the requested form by round-tripping it
/// through a synthetic single-header message.
fn form_value(raw: &str, form: HeaderForm) -> serde_json::Value {
   match form {
      HeaderForm::Raw => serde_json::Value::String(raw.to_owned()),
      HeaderForm::Text => {
         let synth = format!("Subject:{raw}\r\n\r\n");
         let text = MessageParser::default()
            .parse(synth.as_bytes())
            .and_then(|msg| msg.subject().map(str::to_owned));
         text.map_or(serde_json::Value::Null, serde_json::Value::String)
      },
      HeaderForm::Addresses => {
         let synth = format!("To:{raw}\r\n\r\n");
         let addrs = MessageParser::default()
            .parse(synth.as_bytes())
            .and_then(|msg| {
               msg.to().map(|addresses| {
                  addresses
                     .iter()
                     .map(|addr| {
                        serde_json::json!({
                            "name": addr.name().map(str::to_owned),
                            "email": addr.address().unwrap_or_default().to_owned(),
                        })
                     })
                     .collect::<Vec<_>>()
               })
            })
            .unwrap_or_default();
         serde_json::Value::Array(addrs)
      },
      HeaderForm::MessageIds => {
         let ids = raw
            .split_whitespace()
            .map(|token| token.trim_matches(['<', '>', ',']))
            .filter(|token| !token.is_empty())
            .map(|token| serde_json::Value::String(token.to_owned()))
            .collect::<Vec<serde_json::Value>>();
         if ids.is_empty() {
            serde_json::Value::Null
         } else {
            serde_json::Value::Array(ids)
         }
      },
      HeaderForm::Date => {
         let synth = format!("Date:{raw}\r\n\r\n");
         MessageParser::default()
            .parse(synth.as_bytes())
            .and_then(|msg| msg.date().map(mail_parser::DateTime::to_rfc3339))
            .map_or(serde_json::Value::Null, serde_json::Value::String)
      },
      HeaderForm::GroupedAddresses => {
         let synth = format!("To:{raw}\r\n\r\n");
         let groups = MessageParser::default()
            .parse(synth.as_bytes())
            .map(|msg| {
               match msg.to() {
                  Some(mail_parser::Address::Group(groups)) => groups
                     .iter()
                     .map(|group| {
                        serde_json::json!({
                            "name": group.name.as_deref(),
                            "addresses": group.addresses.iter().map(addr_json).collect::<Vec<_>>(),
                        })
                     })
                     .collect::<Vec<_>>(),
                  // A flat list is one anonymous group (RFC 8621 §4.1.2.3).
                  Some(mail_parser::Address::List(addrs)) => {
                     vec![serde_json::json!({
                         "name": serde_json::Value::Null,
                         "addresses": addrs.iter().map(addr_json).collect::<Vec<_>>(),
                     })]
                  },
                  None => vec![],
               }
            })
            .unwrap_or_default();
         serde_json::Value::Array(groups)
      },
      HeaderForm::Urls => {
         // RFC 2369 list-header syntax: comma-separated <URL>s, comments
         // in parens between them. Split on the angle brackets directly.
         let urls = raw
            .split('<')
            .skip(1)
            .filter_map(|chunk| chunk.split('>').next())
            .map(str::trim)
            .filter(|url| !url.is_empty())
            .map(|url| serde_json::Value::String(url.to_owned()))
            .collect::<Vec<serde_json::Value>>();
         if urls.is_empty() {
            serde_json::Value::Null
         } else {
            serde_json::Value::Array(urls)
         }
      },
   }
}

fn addr_json(addr: &mail_parser::Addr<'_>) -> serde_json::Value {
   serde_json::json!({
       "name": addr.name().map(str::to_owned),
       "email": addr.address().unwrap_or_default().to_owned(),
   })
}

/// Rebuild the spec's bodyStructure tree from mail-parser's flattened part
/// list. Leaf partIds use the `p{index}` scheme, which the blob download
/// endpoint resolves by index.
pub(crate) fn body_structure(parsed: &Message<'_>, msgid: &str) -> EmailBodyPart {
   build_part(parsed, 0, msgid)
}

fn build_part(parsed: &Message<'_>, idx: usize, msgid: &str) -> EmailBodyPart {
   let Some(part) = parsed.parts.get(idx) else {
      return empty_part();
   };
   let ct = part.content_type();
   let mime_type = ct.map_or_else(
      || {
         match &part.body {
            PartType::Html(_) => "text/html".to_owned(),
            PartType::Text(_) => "text/plain".to_owned(),
            PartType::Message(_) => "message/rfc822".to_owned(),
            PartType::Multipart(_) => "multipart/mixed".to_owned(),
            _ => "application/octet-stream".to_owned(),
         }
      },
      |content_type| {
         content_type.subtype().map_or_else(
            || content_type.ctype().to_owned(),
            |sub| format!("{}/{}", content_type.ctype(), sub),
         )
      },
   );

   if let PartType::Multipart(children) = &part.body {
      let sub_parts = children
         .iter()
         .filter_map(|child| usize::try_from(*child).ok())
         .map(|child| build_part(parsed, child, msgid))
         .collect();
      return EmailBodyPart {
         part_id: None,
         blob_id: None,
         size: 0,
         mime_type,
         charset: None,
         disposition: None,
         name: None,
         content_id: None,
         language: None,
         location: None,
         headers: Some(raw_headers(parsed, &part.headers)),
         sub_parts: Some(sub_parts),
      };
   }

   let charset = mime_type.starts_with("text/").then(|| {
      ct.and_then(|content_type| content_type.attribute("charset"))
         .unwrap_or("us-ascii")
         .to_owned()
   });
   let name = part.attachment_name().map(str::to_owned).or_else(|| {
      ct.and_then(|content_type| content_type.attribute("name"))
         .map(str::to_owned)
   });
   let disposition = part
      .content_disposition()
      .map(|disposition| disposition.ctype().to_ascii_lowercase());
   let size = match &part.body {
      PartType::Text(text) | PartType::Html(text) => text.len() as u64,
      PartType::Binary(bytes) | PartType::InlineBinary(bytes) => bytes.len() as u64,
      PartType::Message(msg) => u64::from(msg.root_part().raw_len()),
      PartType::Multipart(_) => 0,
   };

   EmailBodyPart {
      part_id: Some(format!("p{idx}")),
      blob_id: Some(Id(format!("blob-{msgid}~p{idx}"))),
      size,
      mime_type,
      charset,
      disposition,
      name,
      content_id: part
         .content_id()
         .map(|cid| cid.trim_matches(['<', '>']).to_owned()),
      language: None,
      location: part.content_location().map(str::to_owned),
      headers: Some(raw_headers(parsed, &part.headers)),
      sub_parts: None,
   }
}

fn empty_part() -> EmailBodyPart {
   EmailBodyPart {
      part_id:     None,
      blob_id:     None,
      size:        0,
      mime_type:   "application/octet-stream".into(),
      charset:     None,
      disposition: None,
      name:        None,
      content_id:  None,
      language:    None,
      location:    None,
      headers:     None,
      sub_parts:   None,
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   const MULTIPART: &[u8] = b"From: a@x.com\r\n\
To: Bob <b@x.com>, c@x.com\r\n\
Subject: =?UTF-8?B?R3LDvMOfZQ==?=\r\n\
Date: Sun, 13 Jul 2026 10:00:00 +0000\r\n\
References: <one@x.com> <two@x.com>\r\n\
X-Custom: hello world\r\n\
X-Custom: second value\r\n\
Content-Type: multipart/mixed; boundary=\"b1\"\r\n\
\r\n\
--b1\r\n\
Content-Type: text/plain; charset=utf-8\r\n\
\r\n\
hi\r\n\
--b1\r\n\
Content-Type: application/pdf; name=\"doc.pdf\"\r\n\
Content-Disposition: attachment; filename=\"doc.pdf\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
JVBERi0=\r\n\
--b1--\r\n";

   fn parsed() -> Message<'static> {
      MessageParser::default().parse(MULTIPART).unwrap()
   }

   #[test]
   fn parses_header_prop_grammar() {
      let prop = parse_header_prop("header:X-Foo").unwrap();
      assert_eq!(
         (prop.name.as_str(), prop.form, prop.all),
         ("X-Foo", HeaderForm::Raw, false)
      );
      let prop = parse_header_prop("header:From:asAddresses").unwrap();
      assert_eq!(prop.form, HeaderForm::Addresses);
      let prop = parse_header_prop("header:X-Foo:asText:all").unwrap();
      assert!(prop.all);
      assert_eq!(prop.form, HeaderForm::Text);
      assert!(parse_header_prop("header:X:asBogus").is_none());
      assert!(parse_header_prop("header:From:asDate").is_none());
      assert!(parse_header_prop("header:Date:asAddresses").is_none());
      assert!(parse_header_prop("header:Bad Name:asText").is_none());
      assert!(parse_header_prop("header:é:asText").is_none());
      assert!(parse_header_prop("notheader:X").is_none());
   }

   #[test]
   fn header_forms_decode() {
      let msg = parsed();
      let raw = header_value(&msg, &parse_header_prop("header:Subject").unwrap());
      assert_eq!(raw, serde_json::json!(" =?UTF-8?B?R3LDvMOfZQ==?="));
      let text = header_value(&msg, &parse_header_prop("header:Subject:asText").unwrap());
      assert_eq!(text, serde_json::json!("Grüße"));
      let addrs = header_value(&msg, &parse_header_prop("header:To:asAddresses").unwrap());
      assert_eq!(addrs[0]["email"], "b@x.com");
      assert_eq!(addrs[0]["name"], "Bob");
      assert_eq!(addrs[1]["email"], "c@x.com");
      let ids = header_value(
         &msg,
         &parse_header_prop("header:References:asMessageIds").unwrap(),
      );
      assert_eq!(ids, serde_json::json!(["one@x.com", "two@x.com"]));
      let date = header_value(&msg, &parse_header_prop("header:Date:asDate").unwrap());
      assert!(date.as_str().unwrap().starts_with("2026-07-13T10:00:00"));
      let missing = header_value(&msg, &parse_header_prop("header:X-Absent").unwrap());
      assert_eq!(missing, serde_json::Value::Null);
   }

   #[test]
   fn urls_and_grouped_addresses_forms() {
      let raw = b"List-Unsubscribe: <mailto:u@x.com>, (web) <https://x.com/u?a=1>\r\n\
To: Team:a@x.com,b@x.com;, solo@x.com\r\n\r\n";
      let msg = MessageParser::default().parse(&raw[..]).unwrap();
      let urls = header_value(
         &msg,
         &parse_header_prop("header:List-Unsubscribe:asURLs").unwrap(),
      );
      assert_eq!(
         urls,
         serde_json::json!(["mailto:u@x.com", "https://x.com/u?a=1"])
      );
      let groups = header_value(
         &msg,
         &parse_header_prop("header:To:asGroupedAddresses").unwrap(),
      );
      let arr = groups.as_array().unwrap();
      assert!(!arr.is_empty(), "{groups}");
      let all = arr
         .iter()
         .flat_map(|group| group["addresses"].as_array().unwrap())
         .map(|addr| addr["email"].as_str().unwrap())
         .collect::<Vec<&str>>();
      assert!(
         all.contains(&"a@x.com") && all.contains(&"solo@x.com"),
         "{groups}"
      );
   }

   #[test]
   fn header_all_returns_every_instance_in_order() {
      let msg = parsed();
      let one = header_value(&msg, &parse_header_prop("header:X-Custom:asText").unwrap());
      assert_eq!(one, serde_json::json!("second value"));
      let all = header_value(
         &msg,
         &parse_header_prop("header:X-Custom:asText:all").unwrap(),
      );
      assert_eq!(all, serde_json::json!(["hello world", "second value"]));
   }

   #[test]
   fn body_structure_tree_shape() {
      let msg = parsed();
      let root = body_structure(&msg, "msg1");
      assert_eq!(root.mime_type, "multipart/mixed");
      assert!(root.part_id.is_none());
      let subs = root.sub_parts.as_ref().unwrap();
      assert_eq!(subs.len(), 2);
      assert_eq!(subs[0].mime_type, "text/plain");
      assert_eq!(subs[0].charset.as_deref(), Some("utf-8"));
      assert!(subs[0].part_id.as_deref().unwrap().starts_with('p'));
      assert_eq!(subs[1].mime_type, "application/pdf");
      assert_eq!(subs[1].disposition.as_deref(), Some("attachment"));
      assert_eq!(subs[1].name.as_deref(), Some("doc.pdf"));
      assert!(
         subs[1]
            .blob_id
            .as_ref()
            .unwrap()
            .as_ref()
            .starts_with("blob-msg1~p")
      );
   }

   #[test]
   fn body_projection_includes_requested_null_properties() {
      let mut value = serde_json::json!({
          "size": 0,
          "type": "multipart/mixed",
          "subParts": [{"partId": "p1", "size": 2, "type": "text/plain"}]
      });
      let properties = requested_body_properties(None);
      project_body_properties(&mut value, &properties);
      assert!(value["partId"].is_null());
      assert!(value["blobId"].is_null());
      assert!(value["subParts"].is_array());
   }

   #[test]
   fn body_projection_supports_header_properties() {
      let mut value = serde_json::json!({
          "partId": "p1",
          "headers": [{"name": "Content-Type", "value": " text/plain; charset=utf-8"}],
          "size": 2,
          "type": "text/plain"
      });
      let properties = vec!["partId".to_owned(), "header:Content-Type".to_owned()];
      assert!(validate_body_properties(Some(&properties)).is_none());
      project_body_properties(&mut value, &properties);
      assert_eq!(value["header:Content-Type"], " text/plain; charset=utf-8");
      assert!(value.get("headers").is_none());
   }
}
