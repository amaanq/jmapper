//! Email object and filter types (RFC 8621 §4).

use std::collections::HashMap;

use chrono::{
   DateTime,
   Utc,
};
use serde::{
   Deserialize,
   Serialize,
};

use crate::ids::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
   pub id:          Id,
   #[serde(rename = "blobId")]
   pub blob_id:     Id,
   #[serde(rename = "threadId")]
   pub thread_id:   Id,
   #[serde(rename = "mailboxIds")]
   pub mailbox_ids: HashMap<Id, bool>,
   pub keywords:    HashMap<String, bool>,
   pub size:        u64,
   #[serde(rename = "receivedAt")]
   pub received_at: DateTime<Utc>,

   // Header forms (RFC 8621 §4.1.2).
   #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
   pub message_id:  Option<Vec<String>>,
   #[serde(rename = "inReplyTo", skip_serializing_if = "Option::is_none")]
   pub in_reply_to: Option<Vec<String>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub references:  Option<Vec<String>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub sender:      Option<Vec<EmailAddress>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub from:        Option<Vec<EmailAddress>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub to:          Option<Vec<EmailAddress>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub cc:          Option<Vec<EmailAddress>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub bcc:         Option<Vec<EmailAddress>>,
   #[serde(rename = "replyTo", skip_serializing_if = "Option::is_none")]
   pub reply_to:    Option<Vec<EmailAddress>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub subject:     Option<String>,
   #[serde(rename = "sentAt", skip_serializing_if = "Option::is_none")]
   pub sent_at:     Option<DateTime<Utc>>,

   // Body (RFC 8621 §4.1.3).
   #[serde(rename = "hasAttachment")]
   pub has_attachment: bool,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub preview:        Option<String>,
   #[serde(rename = "bodyValues", skip_serializing_if = "Option::is_none")]
   pub body_values:    Option<HashMap<String, EmailBodyValue>>,
   #[serde(rename = "textBody", skip_serializing_if = "Option::is_none")]
   pub text_body:      Option<Vec<EmailBodyPart>>,
   #[serde(rename = "htmlBody", skip_serializing_if = "Option::is_none")]
   pub html_body:      Option<Vec<EmailBodyPart>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub attachments:    Option<Vec<EmailBodyPart>>,

   /// Any other selected properties get tacked on here. Lets callers emit
   /// `header:foo:asText`-style header forms without us enumerating them.
   #[serde(flatten)]
   pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAddress {
   #[serde(skip_serializing_if = "Option::is_none")]
   pub name:  Option<String>,
   pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailBodyValue {
   pub value:               String,
   #[serde(rename = "isEncodingProblem")]
   pub is_encoding_problem: bool,
   #[serde(rename = "isTruncated")]
   pub is_truncated:        bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailBodyPart {
   #[serde(rename = "partId", skip_serializing_if = "Option::is_none")]
   pub part_id:     Option<String>,
   #[serde(rename = "blobId", skip_serializing_if = "Option::is_none")]
   pub blob_id:     Option<Id>,
   pub size:        u64,
   #[serde(rename = "type")]
   pub mime_type:   String,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub charset:     Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub disposition: Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub name:        Option<String>,
   #[serde(rename = "cid", skip_serializing_if = "Option::is_none")]
   pub content_id:  Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub language:    Option<Vec<String>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub location:    Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub headers:     Option<Vec<EmailHeader>>,
   #[serde(rename = "subParts", skip_serializing_if = "Option::is_none")]
   pub sub_parts:   Option<Vec<Self>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailHeader {
   pub name:  String,
   pub value: String,
}

/// RFC 8621 §4.4.1 — Email/query `FilterCondition`.
///
/// All fields are optional; a condition can specify any subset. Unknown fields
/// surface as `UnsupportedFilter` at the query-compilation layer.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EmailFilter {
   #[serde(rename = "inMailbox", skip_serializing_if = "Option::is_none")]
   pub in_mailbox:                  Option<Id>,
   #[serde(rename = "inMailboxOtherThan", skip_serializing_if = "Option::is_none")]
   pub in_mailbox_other_than:       Option<Vec<Id>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub before:                      Option<DateTime<Utc>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub after:                       Option<DateTime<Utc>>,
   #[serde(rename = "minSize", skip_serializing_if = "Option::is_none")]
   pub min_size:                    Option<u64>,
   #[serde(rename = "maxSize", skip_serializing_if = "Option::is_none")]
   pub max_size:                    Option<u64>,
   #[serde(
      rename = "allInThreadHaveKeyword",
      skip_serializing_if = "Option::is_none"
   )]
   pub all_in_thread_have_keyword:  Option<String>,
   #[serde(
      rename = "someInThreadHaveKeyword",
      skip_serializing_if = "Option::is_none"
   )]
   pub some_in_thread_have_keyword: Option<String>,
   #[serde(
      rename = "noneInThreadHaveKeyword",
      skip_serializing_if = "Option::is_none"
   )]
   pub none_in_thread_have_keyword: Option<String>,
   #[serde(rename = "hasKeyword", skip_serializing_if = "Option::is_none")]
   pub has_keyword:                 Option<String>,
   #[serde(rename = "notKeyword", skip_serializing_if = "Option::is_none")]
   pub not_keyword:                 Option<String>,
   #[serde(rename = "hasAttachment", skip_serializing_if = "Option::is_none")]
   pub has_attachment:              Option<bool>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub text:                        Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub from:                        Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub to:                          Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub cc:                          Option<Vec<String>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub bcc:                         Option<Vec<String>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub subject:                     Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub body:                        Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub header:                      Option<Vec<String>>,
}
