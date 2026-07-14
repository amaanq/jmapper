//! JMAP result references (RFC 8620 §3.7) and createdIds substitution.
//!
//! When a client issues multiple method calls in one request, later calls can
//! refer to the results of earlier ones with a `ResultReference` object keyed
//! by `#` prefix — e.g. `{"ids": {"resultOf": "c0", "name": "Mailbox/query",
//! "path": "/ids"}}` reads `/ids` from the earlier invocation tagged `c0`.
//!
//! The substitution rules mirror Perl's `_parsepath` in `JMAP/API.pm:233`:
//!   * `~1` and `~0` escapes are decoded per RFC 6901 (JSON Pointer).
//!   * `*` acts as array-splat — a wildcard over every element, flattening one
//!     level when the segment continues past the splat.
//!   * Numeric segments index arrays; anything else does hash lookup.
//!
//! Unknown `resultOf` tags and type mismatches produce an
//! `invalidResultReference` method error (RFC 8620 §3.7.1), never a silent
//! empty substitution.
//!
//! This module is pure protocol: no IMAP, no SQL, no network.

use jmap_protocol::{
   error::MethodError,
   ids::MethodCallId,
   method::Invocation,
};
use serde::Deserialize;
use serde_json::{
   Map,
   Value,
};

/// A single `ResultReference` as it appears in JMAP args under a `#`-prefixed
/// key. The client supplies all three fields; the server resolves them against
/// the running response buffer.
#[derive(Debug, Deserialize)]
struct ResultReference {
   #[serde(rename = "resultOf")]
   result_of: MethodCallId,
   name:      String,
   path:      String,
}

/// Resolve every `#key` reference in `args` against `prior` invocations,
/// returning a new args object with the resolved values under the
/// `key` (no leading `#`) name. Non-`#` keys pass through unchanged.
///
/// # Errors
///
/// Returns [`MethodError::InvalidResultReference`] when a `#key` value is not a
/// well-formed reference or cannot be resolved (unknown `resultOf`, wrong
/// method name, or malformed JSON Pointer). The error flags the *reference*,
/// not the args as a whole, so a later call that mixes literal and referenced
/// fields still fails in a way the client can correlate.
pub fn resolve_args(args: &Value, prior: &[Invocation]) -> Result<Value, MethodError> {
   let Some(obj) = args.as_object() else {
      return Ok(args.clone());
   };
   let mut out = Map::with_capacity(obj.len());
   for (key, value) in obj {
      if let Some(stripped) = key.strip_prefix('#') {
         let reference =
            serde_json::from_value::<ResultReference>(value.clone()).map_err(|err| {
               MethodError::InvalidResultReference {
                  description: Some(format!("malformed reference under #{stripped}: {err}")),
               }
            })?;
         let resolved = resolve_reference(&reference, prior).map_err(|msg| {
            MethodError::InvalidResultReference {
               description: Some(format!("#{stripped}: {msg}")),
            }
         })?;
         out.insert(stripped.to_owned(), resolved);
      } else {
         out.insert(key.clone(), value.clone());
      }
   }
   Ok(Value::Object(out))
}

/// Find the invocation with `call_id == result_of`, verify its name matches,
/// and apply the JSON Pointer path. Wraps single values in a one-element
/// array on the way out — same shape Perl produces, and the shape every
/// consumer of a reference expects.
fn resolve_reference(reference: &ResultReference, prior: &[Invocation]) -> Result<Value, String> {
   let target = prior
      .iter()
      .find(|inv| inv.call_id == reference.result_of)
      .ok_or_else(|| format!("no method call with id '{}'", reference.result_of))?;

   // Error responses at the same callId do not satisfy result references.
   if target.name != reference.name {
      return Err(format!(
         "result of '{}' is '{}' not '{}'",
         reference.result_of, target.name, reference.name,
      ));
   }

   // RFC 6901 §3: a non-empty pointer MUST start with `/`. Empty pointer
   // refers to the whole document. Reject malformed paths loudly instead
   // of silently treating them as whole-document references.
   if !reference.path.is_empty() && !reference.path.starts_with('/') {
      return Err(format!(
         "malformed pointer {:?}: must be empty or start with '/'",
         reference.path,
      ));
   }

   let picked = apply_pointer(&reference.path, &target.arguments);
   Ok(normalize_to_array(picked))
}

/// RFC 6901 JSON Pointer with the JMAP `*` splat extension.
///
/// Return value mirrors Perl semantics: arrays flatten one level per `*`;
/// missing segments collapse to `null`; non-container segments stop early
/// and return the current node (Perl returns `$item` when neither hash nor
/// array matches).
fn apply_pointer(path: &str, item: &Value) -> Value {
   let Some(rest) = path.strip_prefix('/') else {
      return item.clone();
   };
   let (raw_segment, tail) = rest
      .find('/')
      .map_or((rest, ""), |index| (&rest[..index], &rest[index..]));
   let segment = decode_pointer_segment(raw_segment);

   match item {
      Value::Array(arr) => {
         if segment == "*" {
            let mut flat = Vec::new();
            for elem in arr {
               match apply_pointer(tail, elem) {
                  Value::Array(inner) => flat.extend(inner),
                  other => flat.push(other),
               }
            }
            Value::Array(flat)
         } else if let Ok(idx) = segment.parse::<usize>() {
            arr.get(idx)
               .map_or(Value::Null, |elem| apply_pointer(tail, elem))
         } else {
            item.clone()
         }
      },
      Value::Object(map) => {
         map.get(&segment)
            .map_or(Value::Null, |value| apply_pointer(tail, value))
      },
      _ => item.clone(),
   }
}

/// RFC 6901 escapes: `~1` → `/`, `~0` → `~`. Order matters — `~0` must be
/// decoded *after* `~1`, otherwise `~01` round-trips wrong.
fn decode_pointer_segment(segment: &str) -> String {
   segment.replace("~1", "/").replace("~0", "~")
}

/// Perl's `resolve_backref` wraps non-array results in a one-element array
/// before handing them back. JMAP consumers expect references to produce
/// arrays (e.g. `ids: [...]`), so we match that convention.
fn normalize_to_array(value: Value) -> Value {
   match value {
      Value::Array(_) => value,
      Value::Null => Value::Array(Vec::new()),
      other => Value::Array(vec![other]),
   }
}

#[cfg(test)]
mod tests {
   use serde_json::json;

   use super::*;

   fn inv(name: &str, args: Value, call_id: &str) -> Invocation {
      Invocation::new(name, args, call_id)
   }

   #[test]
   fn pass_through_non_hash() {
      let resolved = resolve_args(&json!([1, 2, 3]), &[]).unwrap();
      assert_eq!(resolved, json!([1, 2, 3]));
   }

   #[test]
   fn literal_keys_pass_through() {
      let args = json!({"accountId": "acctA", "ids": ["m1"]});
      let resolved = resolve_args(&args, &[]).unwrap();
      assert_eq!(resolved, args);
   }

   #[test]
   fn resolve_ids_from_prior_query() {
      let prior = vec![inv("Mailbox/query", json!({"ids": ["mb1", "mb2"]}), "c0")];
      let args = json!({
          "accountId": "acctA",
          "#ids": {"resultOf": "c0", "name": "Mailbox/query", "path": "/ids"}
      });
      let resolved = resolve_args(&args, &prior).unwrap();
      assert_eq!(
         resolved,
         json!({"accountId": "acctA", "ids": ["mb1", "mb2"]})
      );
   }

   #[test]
   #[expect(
      clippy::panic,
      reason = "test asserts the error is the expected variant"
   )]
   fn unknown_result_of_errors() {
      let args = json!({
          "#ids": {"resultOf": "nope", "name": "Mailbox/query", "path": "/ids"}
      });
      let err = resolve_args(&args, &[]).unwrap_err();
      match err {
         MethodError::InvalidResultReference { description } => {
            assert!(
               description
                  .unwrap()
                  .contains("no method call with id 'nope'")
            );
         },
         _ => panic!("expected invalidResultReference, got {err:?}"),
      }
   }

   #[test]
   fn wrong_name_errors() {
      let prior = vec![inv("Mailbox/query", json!({"ids": ["mb1"]}), "c0")];
      let args = json!({
          "#ids": {"resultOf": "c0", "name": "Email/query", "path": "/ids"}
      });
      let err = resolve_args(&args, &prior).unwrap_err();
      matches!(err, MethodError::InvalidResultReference { .. });
   }

   #[test]
   fn splat_expands_array() {
      // Email/get result: [{id, threadId}, ...] → splat /list/*/threadId →
      // flat [thrid, thrid].
      let prior = vec![inv(
         "Email/get",
         json!({"list": [
             {"id": "m1", "threadId": "t1"},
             {"id": "m2", "threadId": "t2"},
         ]}),
         "c0",
      )];
      let args = json!({
          "#ids": {"resultOf": "c0", "name": "Email/get", "path": "/list/*/threadId"}
      });
      let resolved = resolve_args(&args, &prior).unwrap();
      assert_eq!(resolved["ids"], json!(["t1", "t2"]));
   }

   #[test]
   fn pointer_tilde_escapes() {
      // Pointer /a~1b decodes to key "a/b".
      let item = json!({"a/b": 42});
      assert_eq!(apply_pointer("/a~1b", &item), json!(42));
      // /a~0b → "a~b".
      let item = json!({"a~b": 43});
      assert_eq!(apply_pointer("/a~0b", &item), json!(43));
   }

   #[test]
   fn missing_segment_becomes_empty_array() {
      let prior = vec![inv("Mailbox/query", json!({"ids": ["mb1"]}), "c0")];
      let args = json!({
          "#ids": {"resultOf": "c0", "name": "Mailbox/query", "path": "/missing"}
      });
      let resolved = resolve_args(&args, &prior).unwrap();
      assert_eq!(resolved["ids"], json!([]));
   }

   #[test]
   fn scalar_wrapped_to_array() {
      // /total → 5 (scalar) → [5] on the way out.
      let prior = vec![inv("Email/query", json!({"ids": ["m1"], "total": 5}), "c0")];
      let args = json!({
          "#ids": {"resultOf": "c0", "name": "Email/query", "path": "/total"}
      });
      let resolved = resolve_args(&args, &prior).unwrap();
      assert_eq!(resolved["ids"], json!([5]));
   }

   #[test]
   #[expect(
      clippy::panic,
      reason = "test asserts the error is the expected variant"
   )]
   fn malformed_pointer_errors() {
      // RFC 6901: non-empty pointer must start with `/`. "ids" without
      // a leading slash used to silently return the whole document; now
      // it surfaces as invalidResultReference.
      let prior = vec![inv("Mailbox/query", json!({"ids": ["mb1"]}), "c0")];
      let args = json!({
          "#ids": {"resultOf": "c0", "name": "Mailbox/query", "path": "ids"}
      });
      let err = resolve_args(&args, &prior).unwrap_err();
      match err {
         MethodError::InvalidResultReference { description } => {
            assert!(description.unwrap().contains("malformed pointer"));
         },
         _ => panic!("expected invalidResultReference"),
      }
   }

   #[test]
   fn empty_pointer_returns_whole_document() {
      // RFC 6901 §5: "" refers to the whole document. Wrapped to array.
      let prior = vec![inv("Mailbox/query", json!({"ids": ["mb1"]}), "c0")];
      let args = json!({
          "#ids": {"resultOf": "c0", "name": "Mailbox/query", "path": ""}
      });
      let resolved = resolve_args(&args, &prior).unwrap();
      // Whole document is the args object {"ids": ["mb1"]}, wrapped.
      assert_eq!(resolved["ids"], json!([{"ids": ["mb1"]}]));
   }

   #[test]
   fn malformed_reference_shape_errors() {
      // Missing required fields should surface as invalidResultReference,
      // not panic.
      let args = json!({"#ids": {"resultOf": "c0"}});
      let err = resolve_args(&args, &[]).unwrap_err();
      matches!(err, MethodError::InvalidResultReference { .. });
   }
}
