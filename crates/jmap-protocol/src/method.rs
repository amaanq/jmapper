//! JMAP request/response envelopes and method invocations (RFC 8620 §3).

use std::collections::HashMap;

use serde::{
   Deserialize,
   Serialize,
};

use crate::ids::MethodCallId;

/// RFC 8620 §3.2 — top-level request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
   /// Capability URIs the client intends to use for this request.
   pub using:        Vec<String>,
   /// Ordered list of method invocations.
   #[serde(rename = "methodCalls")]
   pub method_calls: Vec<Invocation>,
   /// Optional client-provided id map for server-assigned ids.
   #[serde(rename = "createdIds", skip_serializing_if = "Option::is_none")]
   pub created_ids:  Option<HashMap<String, String>>,
}

/// RFC 8620 §3.5 — top-level response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
   #[serde(rename = "methodResponses")]
   pub method_responses: Vec<Invocation>,
   #[serde(rename = "createdIds", skip_serializing_if = "Option::is_none")]
   pub created_ids:      Option<HashMap<String, String>>,
   #[serde(rename = "sessionState")]
   pub session_state:    String,
}

/// RFC 8620 §3.2 — a single `[method, args, callId]` tuple.
///
/// JMAP represents this as a heterogeneous 3-array. We preserve the wire
/// shape with a custom Serialize/Deserialize impl instead of forcing users
/// through a tuple.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
   pub name:      String,
   pub arguments: serde_json::Value,
   pub call_id:   MethodCallId,
}

impl Invocation {
   #[inline]
   pub fn new<Name, CallId>(name: Name, arguments: serde_json::Value, call_id: CallId) -> Self
   where
      Name: Into<String>,
      CallId: Into<MethodCallId>,
   {
      Self {
         name: name.into(),
         arguments,
         call_id: call_id.into(),
      }
   }

   /// Deserialize the `arguments` payload into a typed struct.
   ///
   /// # Errors
   ///
   /// Returns the serde error produced when the arguments do not match `T`.
   #[inline]
   pub fn args_as<T>(&self) -> Result<T, serde_json::Error>
   where
      T: for<'de> Deserialize<'de>,
   {
      T::deserialize(&self.arguments)
   }
}

impl Serialize for Invocation {
   #[inline]
   fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
   where
      S: serde::Serializer,
   {
      use serde::ser::SerializeTuple as _;
      let mut tup = serializer.serialize_tuple(3)?;
      tup.serialize_element(&self.name)?;
      tup.serialize_element(&self.arguments)?;
      tup.serialize_element(&self.call_id)?;
      tup.end()
   }
}

impl<'de> Deserialize<'de> for Invocation {
   #[inline]
   fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
   where
      D: serde::Deserializer<'de>,
   {
      let (name, arguments, call_id) =
         <(String, serde_json::Value, MethodCallId)>::deserialize(deserializer)?;
      Ok(Self {
         name,
         arguments,
         call_id,
      })
   }

   #[inline]
   fn deserialize_in_place<D>(deserializer: D, place: &mut Self) -> Result<(), D::Error>
   where
      D: serde::Deserializer<'de>,
   {
      *place = Self::deserialize(deserializer)?;
      Ok(())
   }
}

#[cfg(test)]
mod tests {
   use pretty_assertions::assert_eq;

   use super::*;

   #[test]
   fn invocation_round_trip() {
      let inv = Invocation::new("Core/echo", serde_json::json!({"hello": "world"}), "c0");
      let serialized = serde_json::to_string(&inv).unwrap();
      assert_eq!(serialized, r#"["Core/echo",{"hello":"world"},"c0"]"#);
      let back = serde_json::from_str::<Invocation>(&serialized).unwrap();
      assert_eq!(back, inv);
   }

   #[test]
   fn request_round_trip() {
      let req = Request {
         using:        vec!["urn:ietf:params:jmap:core".to_owned()],
         method_calls: vec![Invocation::new(
            "Core/echo",
            serde_json::json!({"x": 1}),
            "1",
         )],
         created_ids:  None,
      };
      let serialized = serde_json::to_string(&req).unwrap();
      let back = serde_json::from_str::<Request>(&serialized).unwrap();
      assert_eq!(back.using, req.using);
      assert_eq!(back.method_calls.len(), 1);
   }
}
