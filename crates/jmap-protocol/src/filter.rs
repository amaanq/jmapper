//! JMAP filter and sort AST (RFC 8620 §5.5, §5.6).
//!
//! Filters form a tree of AND/OR/NOT operators over `FilterCondition` leaves.
//! Each object type defines its own condition struct (see
//! [`crate::mailbox::MailboxFilter`], [`crate::email::EmailFilter`]), and the
//! generic [`Filter<C>`] wraps them so the AST shape is shared.
//!
//! Wire encoding: the operator node is distinguished from a condition by the
//! presence of the `operator` field. We use `#[serde(untagged)]` and try the
//! operator variant first so conditions don't accidentally absorb it.

use serde::{
   Deserialize,
   Serialize,
};
use serde_json::Value;

/// Returns whether a filter tree asks for an unsupported condition field or
/// boolean operator.
///
/// Malformed values are left to serde so callers can distinguish
/// `invalidArguments` from `unsupportedFilter`.
#[inline]
pub fn has_unsupported_fields(value: &Value, condition_fields: &[&str]) -> bool {
   let Some(object) = value.as_object() else {
      return false;
   };

   if let Some(operator) = object.get("operator") {
      if !matches!(operator.as_str(), Some("AND" | "OR" | "NOT")) {
         return true;
      }
      if object
         .keys()
         .any(|key| !matches!(key.as_str(), "operator" | "conditions"))
      {
         return true;
      }
      return object
         .get("conditions")
         .and_then(Value::as_array)
         .is_some_and(|conditions| {
            conditions
               .iter()
               .any(|child| has_unsupported_fields(child, condition_fields))
         });
   }

   object
      .keys()
      .any(|key| !condition_fields.contains(&key.as_str()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum FilterOp {
   And,
   Or,
   Not,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Filter<C> {
   /// Boolean combinator over child filters.
   Operator {
      operator:   FilterOp,
      conditions: Vec<Self>,
   },
   /// Leaf filter condition; shape is type-specific.
   Condition(C),
}

/// RFC 8620 §5.5 — comparator for sort order.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comparator {
   pub property:     String,
   #[serde(default = "default_true", rename = "isAscending")]
   pub is_ascending: bool,
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub collation:    Option<String>,
   /// Additional per-property comparator arguments (e.g. `keyword` for
   /// `allInThreadHaveKeyword`).
   #[serde(flatten)]
   pub extra:        serde_json::Map<String, serde_json::Value>,
}

const fn default_true() -> bool {
   true
}

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;
   use serde::{
      Deserialize,
      Serialize,
   };

   use super::*;

   #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
   struct StubCond {
      subject: Option<String>,
   }

   #[test]
   fn bare_condition_parses() {
      let serialized = r#"{"subject":"hi"}"#;
      let filter = serde_json::from_str::<Filter<StubCond>>(serialized).unwrap();
      assert_eq!(
         filter,
         Filter::Condition(StubCond {
            subject: Some("hi".into()),
         })
      );
   }

   #[test]
   fn operator_parses() {
      let serialized = r#"{"operator":"AND","conditions":[{"subject":"x"},{"subject":"y"}]}"#;
      let filter = serde_json::from_str::<Filter<StubCond>>(serialized).unwrap();
      let Filter::Operator {
         operator,
         conditions,
      } = filter
      else {
         unreachable!("serde accepted a non-operator filter");
      };
      assert_eq!(operator, FilterOp::And);
      assert_eq!(conditions.len(), 2);
   }

   #[test]
   fn comparator_round_trip() {
      let comparator = Comparator {
         property:     "receivedAt".into(),
         is_ascending: false,
         collation:    None,
         extra:        serde_json::Map::default(),
      };
      let serialized = serde_json::to_string(&comparator).unwrap();
      assert_eq!(
         serialized,
         r#"{"property":"receivedAt","isAscending":false}"#
      );
   }

   #[test]
   fn unsupported_fields_are_found_recursively() {
      let filter = serde_json::json!({
          "operator": "AND",
          "conditions": [
              {"subject": "ok"},
              {"operator": "OR", "conditions": [{"madeUp": true}]}
          ]
      });
      assert!(has_unsupported_fields(&filter, &["subject"]));
      assert!(!has_unsupported_fields(
         &serde_json::json!({
             "operator": "NOT",
             "conditions": [{"subject": "ok"}]
         }),
         &["subject"]
      ));
   }

   #[test]
   fn unsupported_operator_and_operator_properties_are_rejected() {
      assert!(has_unsupported_fields(
         &serde_json::json!({"operator": "XOR", "conditions": []}),
         &["subject"]
      ));
      assert!(has_unsupported_fields(
         &serde_json::json!({"operator": "AND", "conditions": [], "extra": true}),
         &["subject"]
      ));
   }

   #[test]
   fn malformed_operator_cannot_fall_back_to_empty_condition() {
      use crate::email::EmailFilter;

      for malformed in [
         serde_json::json!({"operator": "AND"}),
         serde_json::json!({"operator": "AND", "conditions": {}}),
      ] {
         serde_json::from_value::<Filter<EmailFilter>>(malformed).unwrap_err();
      }
   }
}
