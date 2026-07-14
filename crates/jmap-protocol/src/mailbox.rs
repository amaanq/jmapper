//! Mailbox object + filter (RFC 8621 §2).

use serde::{
   Deserialize,
   Serialize,
};

use crate::ids::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mailbox {
   pub id:             Id,
   pub name:           String,
   #[serde(rename = "parentId")]
   pub parent_id:      Option<Id>,
   pub role:           Option<String>,
   #[serde(rename = "sortOrder")]
   pub sort_order:     u32,
   #[serde(rename = "totalEmails")]
   pub total_emails:   u64,
   #[serde(rename = "unreadEmails")]
   pub unread_emails:  u64,
   #[serde(rename = "totalThreads")]
   pub total_threads:  u64,
   #[serde(rename = "unreadThreads")]
   pub unread_threads: u64,
   #[serde(rename = "myRights")]
   pub my_rights:      MailboxRights,
   #[serde(rename = "isSubscribed")]
   pub is_subscribed:  bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailboxRights {
   #[serde(rename = "mayReadItems")]
   pub may_read_items:   bool,
   #[serde(rename = "mayAddItems")]
   pub may_add_items:    bool,
   #[serde(rename = "mayRemoveItems")]
   pub may_remove_items: bool,
   #[serde(rename = "maySetSeen")]
   pub may_set_seen:     bool,
   #[serde(rename = "maySetKeywords")]
   pub may_set_keywords: bool,
   #[serde(rename = "mayCreateChild")]
   pub may_create_child: bool,
   #[serde(rename = "mayRename")]
   pub may_rename:       bool,
   #[serde(rename = "mayDelete")]
   pub may_delete:       bool,
   #[serde(rename = "maySubmit")]
   pub may_submit:       bool,
}

impl MailboxRights {
   #[inline]
   #[must_use]
   pub const fn read_only() -> Self {
      Self {
         may_read_items:   true,
         may_add_items:    false,
         may_remove_items: false,
         may_set_seen:     false,
         may_set_keywords: false,
         may_create_child: false,
         may_rename:       false,
         may_delete:       false,
         may_submit:       false,
      }
   }

   /// Full mail-client rights. Role mailboxes (inbox, sent, …) can hold and
   /// mutate messages but the folder object itself is immutable.
   #[inline]
   #[must_use]
   pub const fn writable(is_role: bool) -> Self {
      Self {
         may_read_items:   true,
         may_add_items:    true,
         may_remove_items: true,
         may_set_seen:     true,
         may_set_keywords: true,
         may_create_child: true,
         may_rename:       !is_role,
         may_delete:       !is_role,
         may_submit:       true,
      }
   }
}

/// RFC 8621 §2.3 — Mailbox filter conditions.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MailboxFilter {
   #[serde(rename = "parentId", skip_serializing_if = "Option::is_none")]
   pub parent_id:     Option<Option<Id>>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub name:          Option<String>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub role:          Option<Option<String>>,
   #[serde(rename = "hasAnyRole", skip_serializing_if = "Option::is_none")]
   pub has_any_role:  Option<bool>,
   #[serde(rename = "isSubscribed", skip_serializing_if = "Option::is_none")]
   pub is_subscribed: Option<bool>,
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn top_level_mailbox_keeps_nullable_properties() {
      let value = serde_json::to_value(Mailbox {
         id:             Id("mailbox".into()),
         name:           "Inbox".into(),
         parent_id:      None,
         role:           None,
         sort_order:     0,
         total_emails:   0,
         unread_emails:  0,
         total_threads:  0,
         unread_threads: 0,
         my_rights:      MailboxRights::read_only(),
         is_subscribed:  true,
      })
      .unwrap();

      assert!(
         value
            .get("parentId")
            .is_some_and(serde_json::Value::is_null)
      );
      assert!(value.get("role").is_some_and(serde_json::Value::is_null));
   }
}
