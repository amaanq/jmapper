//! RFC 8620 §2 — Session Resource.

use std::collections::HashMap;

use serde::{
   Deserialize,
   Serialize,
};

use crate::ids::AccountId;

pub const URN_CORE: &str = "urn:ietf:params:jmap:core";
pub const URN_MAIL: &str = "urn:ietf:params:jmap:mail";
pub const URN_SUBMISSION: &str = "urn:ietf:params:jmap:submission";
pub const URN_QUOTA: &str = "urn:ietf:params:jmap:quota";
pub const URN_VACATION_RESPONSE: &str = "urn:ietf:params:jmap:vacationresponse";
pub const URN_CALENDARS: &str = "urn:ietf:params:jmap:calendars";
pub const URN_CONTACTS: &str = "urn:ietf:params:jmap:contacts";
pub const MAX_OBJECTS_IN_GET: usize = 500;
pub const MAX_OBJECTS_IN_SET: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
   pub capabilities:     HashMap<String, serde_json::Value>,
   pub accounts:         HashMap<AccountId, Account>,
   #[serde(rename = "primaryAccounts")]
   pub primary_accounts: HashMap<String, AccountId>,
   pub username:         String,
   #[serde(rename = "apiUrl")]
   pub api_url:          String,
   #[serde(rename = "downloadUrl")]
   pub download_url:     String,
   #[serde(rename = "uploadUrl")]
   pub upload_url:       String,
   #[serde(rename = "eventSourceUrl")]
   pub event_source_url: String,
   pub state:            String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
   pub name:                 String,
   #[serde(rename = "isPersonal")]
   pub is_personal:          bool,
   #[serde(rename = "isReadOnly")]
   pub is_read_only:         bool,
   #[serde(rename = "accountCapabilities")]
   pub account_capabilities: HashMap<String, serde_json::Value>,
}

/// The core capability object (RFC 8620 §2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreCapability {
   #[serde(rename = "maxSizeUpload")]
   pub max_size_upload:         u64,
   #[serde(rename = "maxConcurrentUpload")]
   pub max_concurrent_upload:   u32,
   #[serde(rename = "maxSizeRequest")]
   pub max_size_request:        u64,
   #[serde(rename = "maxConcurrentRequests")]
   pub max_concurrent_requests: u32,
   #[serde(rename = "maxCallsInRequest")]
   pub max_calls_in_request:    u32,
   #[serde(rename = "maxObjectsInGet")]
   pub max_objects_in_get:      u32,
   #[serde(rename = "maxObjectsInSet")]
   pub max_objects_in_set:      u32,
   #[serde(rename = "collationAlgorithms")]
   pub collation_algorithms:    Vec<String>,
}

impl Default for CoreCapability {
   #[inline]
   fn default() -> Self {
      Self {
         max_size_upload:         50 * 1024 * 1024,
         max_concurrent_upload:   4,
         max_size_request:        10 * 1024 * 1024,
         max_concurrent_requests: 4,
         max_calls_in_request:    32,
         max_objects_in_get:      u32::try_from(MAX_OBJECTS_IN_GET)
            .expect("JMAP object limit fits in u32"),
         max_objects_in_set:      u32::try_from(MAX_OBJECTS_IN_SET)
            .expect("JMAP object limit fits in u32"),
         // PostgreSQL's default collation is deployment-dependent and the
         // query layer currently rejects explicit comparator collations.
         collation_algorithms:    vec![],
      }
   }
}

/// The mail capability object (RFC 8621 §2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailCapability {
   #[serde(rename = "maxMailboxesPerEmail")]
   pub max_mailboxes_per_email:        Option<u32>,
   #[serde(rename = "maxMailboxDepth")]
   pub max_mailbox_depth:              Option<u32>,
   #[serde(rename = "maxSizeMailboxName")]
   pub max_size_mailbox_name:          u32,
   #[serde(rename = "maxSizeAttachmentsPerEmail")]
   pub max_size_attachments_per_email: u64,
   #[serde(rename = "emailQuerySortOptions")]
   pub email_query_sort_options:       Vec<String>,
   #[serde(rename = "mayCreateTopLevelMailbox")]
   pub may_create_top_level_mailbox:   bool,
}

/// The submission capability object (RFC 8621 §1.3.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmissionCapability {
   /// Seconds a submission may be scheduled into the future (RFC 8621
   /// §1.3.1). Bounded so a forgotten queue row can't linger for months.
   #[serde(rename = "maxDelayedSend")]
   pub max_delayed_send:      u32,
   #[serde(rename = "submissionExtensions")]
   pub submission_extensions: HashMap<String, Vec<String>>,
}

impl Default for SubmissionCapability {
   #[inline]
   fn default() -> Self {
      Self {
         max_delayed_send:      7 * 24 * 3600,
         submission_extensions: HashMap::new(),
      }
   }
}

impl Default for MailCapability {
   #[inline]
   fn default() -> Self {
      Self {
         max_mailboxes_per_email:        None,
         max_mailbox_depth:              Some(10),
         max_size_mailbox_name:          490,
         max_size_attachments_per_email: 50 * 1024 * 1024,
         // Capability MUST match what `Email/query` will actually accept
         // (see `sort_expr` in jmap-server).
         email_query_sort_options:       vec![
            "receivedAt".into(),
            "hasKeyword".into(),
            "sentAt".into(),
            "subject".into(),
            "size".into(),
            "from".into(),
            "to".into(),
         ],
         may_create_top_level_mailbox:   true,
      }
   }
}

/// RFC 9610 §1.4.1 account capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactsCapability {
   #[serde(rename = "maxAddressBooksPerCard")]
   pub max_address_books_per_card: Option<u32>,
   #[serde(rename = "mayCreateAddressBook")]
   pub may_create_address_book:    bool,
}

impl ContactsCapability {
   #[must_use]
   #[inline]
   pub const fn dav_proxy() -> Self {
      Self {
         // A DAV resource has one parent collection; emulating multiple
         // memberships would create independently diverging copies.
         max_address_books_per_card: Some(1),
         may_create_address_book:    false,
      }
   }
}

/// draft-ietf-jmap-calendars account capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarsCapability {
   #[serde(rename = "maxCalendarsPerEvent")]
   pub max_calendars_per_event:     Option<u32>,
   #[serde(rename = "minDateTime")]
   pub min_date_time:               String,
   #[serde(rename = "maxDateTime")]
   pub max_date_time:               String,
   #[serde(rename = "maxExpandedQueryDuration")]
   pub max_expanded_query_duration: String,
   #[serde(rename = "maxParticipantsPerEvent")]
   pub max_participants_per_event:  Option<u32>,
   #[serde(rename = "mayCreateCalendar")]
   pub may_create_calendar:         bool,
}

impl CalendarsCapability {
   #[must_use]
   #[inline]
   pub fn dav_proxy() -> Self {
      Self {
         max_calendars_per_event:     Some(1),
         min_date_time:               "0001-01-01T00:00:00Z".to_owned(),
         max_date_time:               "9999-12-31T23:59:59Z".to_owned(),
         // This DAV bridge round-trips recurrence rules but does not mint
         // synthetic occurrence ids. A zero duration advertises that
         // recurrence expansion is unavailable without pretending a
         // positive window will succeed.
         max_expanded_query_duration: "PT0S".to_owned(),
         max_participants_per_event:  None,
         may_create_calendar:         false,
      }
   }
}

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;

   use super::*;

   #[test]
   fn core_capability_round_trip() {
      let capability = CoreCapability::default();
      let serialized = serde_json::to_value(&capability).unwrap();
      assert_eq!(serialized["maxCallsInRequest"], 32);
      assert_eq!(serialized["collationAlgorithms"], serde_json::json!([]));
   }
}
