//! Thread object (RFC 8621 §3).

use serde::{
   Deserialize,
   Serialize,
};

use crate::ids::Id;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
   pub id:        Id,
   /// Ordered list of Email ids in the thread.
   #[serde(rename = "emailIds")]
   pub email_ids: Vec<Id>,
}
