//! Newtype wrappers for JMAP identifier strings.

use std::fmt;

use serde::{
   Deserialize,
   Serialize,
};

macro_rules! opaque_id {
   ($name:ident, $doc:expr) => {
      #[doc = $doc]
      #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
      #[serde(transparent)]
      pub struct $name(pub String);

      impl fmt::Display for $name {
         #[inline]
         fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str(&self.0)
         }
      }

      impl From<String> for $name {
         #[inline]
         fn from(value: String) -> Self {
            Self(value)
         }
      }

      impl From<&str> for $name {
         #[inline]
         fn from(value: &str) -> Self {
            Self(value.to_owned())
         }
      }

      impl AsRef<str> for $name {
         #[inline]
         fn as_ref(&self) -> &str {
            &self.0
         }
      }
   };
}

opaque_id!(Id, "RFC 8620 §1.2: a JMAP object Id.");
opaque_id!(AccountId, "RFC 8620 §1.6.2: account identifier.");
opaque_id!(State, "RFC 8620 §5.2: per-type opaque state string.");
opaque_id!(MethodCallId, "RFC 8620 §3.2: per-invocation client tag.");
