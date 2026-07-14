//! Generic JMAP method request/response envelopes (RFC 8620 §5).
//!
//! These are parameterized by the per-type filter, comparator-args, and object
//! shapes that live in `email.rs`, `mailbox.rs`, `thread.rs`.

use std::collections::HashMap;

use serde::{
   Deserialize,
   Serialize,
};

use crate::{
   filter::{
      Comparator,
      Filter,
   },
   ids::{
      AccountId,
      Id,
      State,
   },
};

// -------- /get --------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetRequest {
   #[serde(rename = "accountId")]
   pub account_id: AccountId,
   #[serde(default)]
   pub ids:        Option<Vec<Id>>,
   #[serde(default)]
   pub properties: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetResponse<T> {
   #[serde(rename = "accountId")]
   pub account_id: AccountId,
   pub state:      State,
   pub list:       Vec<T>,
   #[serde(rename = "notFound")]
   pub not_found:  Vec<Id>,
}

// -------- /query --------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest<C> {
   #[serde(rename = "accountId")]
   pub account_id:      AccountId,
   #[serde(default)]
   pub filter:          Option<Filter<C>>,
   #[serde(default)]
   pub sort:            Option<Vec<Comparator>>,
   #[serde(default)]
   pub position:        Option<i64>,
   #[serde(default)]
   pub anchor:          Option<Id>,
   #[serde(rename = "anchorOffset", default)]
   pub anchor_offset:   Option<i64>,
   #[serde(default)]
   pub limit:           Option<u32>,
   #[serde(rename = "calculateTotal", default)]
   pub calculate_total: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
   #[serde(rename = "accountId")]
   pub account_id:            AccountId,
   #[serde(rename = "queryState")]
   pub query_state:           State,
   #[serde(rename = "canCalculateChanges")]
   pub can_calculate_changes: bool,
   pub position:              i64,
   pub ids:                   Vec<Id>,
   #[serde(rename = "total", skip_serializing_if = "Option::is_none")]
   pub total:                 Option<u64>,
   #[serde(skip_serializing_if = "Option::is_none")]
   pub limit:                 Option<u32>,
}

// -------- /changes --------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesRequest {
   #[serde(rename = "accountId")]
   pub account_id:  AccountId,
   #[serde(rename = "sinceState")]
   pub since_state: State,
   #[serde(rename = "maxChanges", default)]
   pub max_changes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesResponse {
   #[serde(rename = "accountId")]
   pub account_id:       AccountId,
   #[serde(rename = "oldState")]
   pub old_state:        State,
   #[serde(rename = "newState")]
   pub new_state:        State,
   #[serde(rename = "hasMoreChanges")]
   pub has_more_changes: bool,
   pub created:          Vec<Id>,
   pub updated:          Vec<Id>,
   pub destroyed:        Vec<Id>,
}

// -------- /queryChanges --------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryChangesRequest<C> {
   #[serde(rename = "accountId")]
   pub account_id:        AccountId,
   #[serde(default)]
   pub filter:            Option<Filter<C>>,
   #[serde(default)]
   pub sort:              Option<Vec<Comparator>>,
   #[serde(rename = "sinceQueryState")]
   pub since_query_state: State,
   #[serde(rename = "maxChanges", default)]
   pub max_changes:       Option<u32>,
   #[serde(rename = "upToId", default)]
   pub up_to_id:          Option<Id>,
   #[serde(rename = "calculateTotal", default)]
   pub calculate_total:   bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddedItem {
   pub id:    Id,
   pub index: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryChangesResponse {
   #[serde(rename = "accountId")]
   pub account_id:      AccountId,
   #[serde(rename = "oldQueryState")]
   pub old_query_state: State,
   #[serde(rename = "newQueryState")]
   pub new_query_state: State,
   #[serde(rename = "total", skip_serializing_if = "Option::is_none")]
   pub total:           Option<u64>,
   pub removed:         Vec<Id>,
   pub added:           Vec<AddedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetRequest<T> {
   #[serde(rename = "accountId")]
   pub account_id:  AccountId,
   #[serde(rename = "ifInState", default)]
   pub if_in_state: Option<State>,
   #[serde(default)]
   pub create:      Option<HashMap<String, T>>,
   #[serde(default)]
   pub update:      Option<HashMap<Id, serde_json::Value>>,
   #[serde(default)]
   pub destroy:     Option<Vec<Id>>,
}
