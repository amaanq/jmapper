//! Email/parse (RFC 8621 §4.9).

use std::collections::{
   HashMap,
   HashSet,
};

use imap_sync::sync;
use jmap_protocol::{
   email::EmailBodyValue,
   error::MethodError,
   ids::{
      AccountId,
      Id,
   },
};
use jmapper_codegen::queries::blobs;
use mail_parser::{
   Message,
   MimeHeaders as _,
   PartType,
};
use serde::Deserialize;
use sha2::{
   Digest as _,
   Sha256,
};

use super::{
   MethodResult,
   bad_args,
   enforce_get_limit,
   ids_or_null,
   object_or_null,
   server_fail,
};
use crate::{
   blob,
   methods::email_props,
   state::{
      AccountInfo,
      AppState,
   },
   upload,
};

const DEFAULT_PROPERTIES: &[&str] = &[
   "messageId",
   "inReplyTo",
   "references",
   "sender",
   "from",
   "to",
   "cc",
   "bcc",
   "replyTo",
   "subject",
   "sentAt",
   "hasAttachment",
   "preview",
   "bodyValues",
   "textBody",
   "htmlBody",
   "attachments",
];
const EMAIL_PROPERTIES: &[&str] = &[
   "id",
   "blobId",
   "threadId",
   "mailboxIds",
   "keywords",
   "size",
   "receivedAt",
   "messageId",
   "inReplyTo",
   "references",
   "sender",
   "from",
   "to",
   "cc",
   "bcc",
   "replyTo",
   "subject",
   "sentAt",
   "hasAttachment",
   "preview",
   "bodyValues",
   "textBody",
   "htmlBody",
   "attachments",
   "bodyStructure",
   "headers",
];

#[derive(Deserialize)]
struct ParseArgs {
   #[serde(rename = "accountId")]
   account_id:             AccountId,
   #[serde(rename = "blobIds")]
   blob_ids:               Vec<Id>,
   #[serde(default)]
   properties:             Option<Vec<String>>,
   #[serde(rename = "bodyProperties", default)]
   body_properties:        Option<Vec<String>>,
   #[serde(rename = "fetchTextBodyValues", default)]
   fetch_text_body_values: bool,
   #[serde(rename = "fetchHTMLBodyValues", default)]
   fetch_html_body_values: bool,
   #[serde(rename = "fetchAllBodyValues", default)]
   fetch_all_body_values:  bool,
   #[serde(rename = "maxBodyValueBytes", default)]
   max_body_value_bytes:   u32,
}

/// # Errors
///
/// Returns a `bad_args` error if the arguments fail to deserialize or request
/// an unsupported property, an auth error if `auth` does not match the
/// requested account, a limit error if too many `blobIds` are requested, and a
/// `server_fail` error if loading a blob, staging a parsed MIME part, or
/// serializing the parsed result fails.
pub async fn parse(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let req = serde_json::from_value::<ParseArgs>(args)
      .map_err(|error| bad_args(format!("invalid Email/parse args: {error}")))?;
   let account_id = req.account_id.as_ref();
   super::require_auth_match(auth, account_id)?;
   enforce_get_limit(req.blob_ids.len())?;
   validate_properties(&req)?;

   let mut parsed = serde_json::Map::new();
   let mut not_found = Vec::<String>::new();
   let mut not_parsable = Vec::<String>::new();
   for blob_id in &req.blob_ids {
      let bytes = blob::load_blob_bytes(state, account_id, blob_id.as_ref())
         .await
         .map_err(|error| server_fail(error.to_string()))?;
      let Some(bytes) = bytes else {
         not_found.push(blob_id.0.clone());
         continue;
      };
      let Some(message) = mail_parser::MessageParser::default().parse(&bytes) else {
         not_parsable.push(blob_id.0.clone());
         continue;
      };
      let value = parsed_email(state, account_id, blob_id.as_ref(), &bytes, &message, &req).await?;
      parsed.insert(blob_id.0.clone(), value);
   }

   Ok(serde_json::json!({
       "accountId": account_id,
       "parsed": object_or_null(parsed),
       "notParsable": ids_or_null(not_parsable),
       "notFound": ids_or_null(not_found),
   }))
}

fn validate_properties(req: &ParseArgs) -> Result<(), MethodError> {
   for property in req.properties.iter().flatten() {
      if property.starts_with("header:") {
         if email_props::parse_header_prop(property).is_none() {
            return Err(bad_args(format!("unsupported header form: {property}")));
         }
      } else if !EMAIL_PROPERTIES.contains(&property.as_str()) {
         return Err(bad_args(format!("unsupported Email property: {property}")));
      }
   }
   if let Some(property) = email_props::validate_body_properties(req.body_properties.as_deref()) {
      return Err(bad_args(format!(
         "unsupported EmailBodyPart property: {property}"
      )));
   }
   Ok(())
}

async fn parsed_email(
   state: &AppState,
   account_id: &str,
   source_blob_id: &str,
   raw: &[u8],
   message: &Message<'_>,
   req: &ParseArgs,
) -> Result<serde_json::Value, MethodError> {
   let mut body = sync::project_to_jmap_with_msgid(message, None);
   let part_blobs = stage_parts(state, account_id, message, &mut body).await?;
   select_body_values(&mut body, req);
   let body_properties = email_props::requested_body_properties(req.body_properties.as_deref());

   let mut values = serde_json::Map::new();
   values.insert("id".into(), serde_json::Value::Null);
   values.insert(
      "blobId".into(),
      serde_json::Value::String(source_blob_id.into()),
   );
   values.insert("threadId".into(), serde_json::Value::Null);
   values.insert("mailboxIds".into(), serde_json::Value::Null);
   values.insert("keywords".into(), serde_json::Value::Null);
   values.insert("size".into(), serde_json::Value::from(raw.len() as u64));
   values.insert("receivedAt".into(), serde_json::Value::Null);
   values.insert(
      "messageId".into(),
      standard_header(message, "Message-ID", "asMessageIds"),
   );
   values.insert(
      "inReplyTo".into(),
      standard_header(message, "In-Reply-To", "asMessageIds"),
   );
   values.insert(
      "references".into(),
      standard_header(message, "References", "asMessageIds"),
   );
   for (property, header) in [
      ("sender", "Sender"),
      ("from", "From"),
      ("to", "To"),
      ("cc", "Cc"),
      ("bcc", "Bcc"),
      ("replyTo", "Reply-To"),
   ] {
      values.insert(
         property.into(),
         standard_header(message, header, "asAddresses"),
      );
   }
   values.insert(
      "subject".into(),
      message
         .subject()
         .map_or(serde_json::Value::Null, |subject| {
            serde_json::Value::String(subject.to_owned())
         }),
   );
   values.insert("sentAt".into(), standard_header(message, "Date", "asDate"));
   values.insert(
      "headers".into(),
      serde_json::to_value(email_props::raw_headers(message, message.headers()))
         .map_err(|error| server_fail(error.to_string()))?,
   );
   values.insert(
      "hasAttachment".into(),
      serde_json::Value::Bool(!body.attachments.is_empty()),
   );
   values.insert(
      "preview".into(),
      body
         .preview
         .clone()
         .map_or(serde_json::Value::Null, serde_json::Value::String),
   );
   values.insert(
      "bodyValues".into(),
      serde_json::to_value(&body.body_values).map_err(|error| server_fail(error.to_string()))?,
   );
   for (property, parts) in [
      ("textBody", &body.text_body),
      ("htmlBody", &body.html_body),
      ("attachments", &body.attachments),
   ] {
      let mut value =
         serde_json::to_value(parts).map_err(|error| server_fail(error.to_string()))?;
      email_props::project_body_properties(&mut value, &body_properties);
      values.insert(property.into(), value);
   }
   let mut structure = serde_json::to_value(email_props::body_structure(message, source_blob_id))
      .map_err(|error| server_fail(error.to_string()))?;
   replace_part_blob_ids(&mut structure, &part_blobs);
   email_props::project_body_properties(&mut structure, &body_properties);
   values.insert("bodyStructure".into(), structure);

   let default_properties = DEFAULT_PROPERTIES
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>();
   let properties = req.properties.as_deref().unwrap_or(&default_properties);
   let mut output = serde_json::Map::new();
   for property in properties {
      if property.starts_with("header:") {
         let parsed =
            email_props::parse_header_prop(property).expect("header properties were validated");
         output.insert(
            property.clone(),
            email_props::header_value(message, &parsed),
         );
      } else if let Some(value) = values.get(property) {
         output.insert(property.clone(), value.clone());
      }
   }
   Ok(serde_json::Value::Object(output))
}

fn standard_header(message: &Message<'_>, name: &str, form: &str) -> serde_json::Value {
   let property = email_props::parse_header_prop(&format!("header:{name}:{form}"))
      .expect("standard header form is valid");
   email_props::header_value(message, &property)
}

fn select_body_values(body: &mut sync::ProjectedBody, req: &ParseArgs) {
   let mut selected = HashSet::<String>::new();
   if req.fetch_all_body_values {
      selected.extend(body.body_values.keys().cloned());
   } else {
      if req.fetch_text_body_values {
         selected.extend(
            body
               .text_body
               .iter()
               .filter_map(|part| part.part_id.clone()),
         );
      }
      if req.fetch_html_body_values {
         selected.extend(
            body
               .html_body
               .iter()
               .filter_map(|part| part.part_id.clone()),
         );
      }
   }
   body.body_values.retain(|part_id, value| {
      if !selected.contains(part_id) {
         return false;
      }
      truncate_body_value(value, req.max_body_value_bytes as usize);
      true
   });
}

fn truncate_body_value(value: &mut EmailBodyValue, maximum: usize) {
   if maximum == 0 || value.value.len() <= maximum {
      return;
   }
   let mut end = maximum;
   while !value.value.is_char_boundary(end) {
      end -= 1;
   }
   value.value.truncate(end);
   value.is_truncated = true;
}

async fn stage_parts(
   state: &AppState,
   account_id: &str,
   message: &Message<'_>,
   body: &mut sync::ProjectedBody,
) -> Result<HashMap<String, Id>, MethodError> {
   let client = state
      .pool()
      .get()
      .await
      .map_err(|error| server_fail(format!("db pool: {error}")))?;
   let mut ids = HashMap::<String, Id>::new();
   for (index, part) in message.parts.iter().enumerate() {
      if matches!(&part.body, PartType::Multipart(_)) {
         continue;
      }
      let mime_type = part.content_type().map_or_else(
         || {
            match &part.body {
               PartType::Text(_) => "text/plain".into(),
               PartType::Html(_) => "text/html".into(),
               PartType::Message(_) => "message/rfc822".into(),
               _ => "application/octet-stream".into(),
            }
         },
         |content_type| {
            format!(
               "{}/{}",
               content_type.ctype(),
               content_type.subtype().unwrap_or("octet-stream")
            )
         },
      );
      let id = stage_blob(&client, account_id, part.contents(), &mime_type).await?;
      ids.insert(format!("p{index}"), id);
   }
   for part in body
      .text_body
      .iter_mut()
      .chain(&mut body.html_body)
      .chain(&mut body.attachments)
   {
      part.blob_id = part
         .part_id
         .as_ref()
         .and_then(|part_id| ids.get(part_id))
         .cloned();
   }
   Ok(ids)
}

async fn stage_blob(
   client: &deadpool_postgres::Object,
   account_id: &str,
   bytes: &[u8],
   content_type: &str,
) -> Result<Id, MethodError> {
   let mut hash = Sha256::new();
   hash.update(bytes);
   let blob_id = format!("blob-upload-{}", hex::encode(hash.finalize()));
   let now = chrono::Utc::now().timestamp();
   let expires_at = now + upload::DEFAULT_EXPIRY_SECS;
   blobs::upsert_uploaded_blob()
      .bind(
         client,
         &account_id,
         &blob_id.as_str(),
         &content_type,
         &bytes,
         &now,
         &expires_at,
      )
      .await
      .map_err(|error| server_fail(format!("staging parsed MIME part: {error}")))?;
   Ok(Id(blob_id))
}

fn replace_part_blob_ids(value: &mut serde_json::Value, ids: &HashMap<String, Id>) {
   match value {
      serde_json::Value::Array(values) => {
         for value in values {
            replace_part_blob_ids(value, ids);
         }
      },
      serde_json::Value::Object(object) => {
         if let Some(part_id) = object
            .get("partId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
         {
            object.insert(
               "blobId".into(),
               ids.get(&part_id).map_or(serde_json::Value::Null, |id| {
                  serde_json::Value::String(id.0.clone())
               }),
            );
         }
         for value in object.values_mut() {
            replace_part_blob_ids(value, ids);
         }
      },
      _ => {},
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn truncation_stays_on_a_utf8_boundary() {
      let mut value = EmailBodyValue {
         value:               "a🎉b".into(),
         is_encoding_problem: false,
         is_truncated:        false,
      };
      truncate_body_value(&mut value, 3);
      assert_eq!(value.value, "a");
      assert!(value.is_truncated);
   }
}
