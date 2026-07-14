//! Per-provider quirks (Gmail vs generic IMAP).

use serde::{
   Deserialize,
   Serialize,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
   Gmail,
   Imap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImapTls {
   /// TLS from the first byte (port 993).
   Implicit,
   /// Plain text with STARTTLS upgrade (port 143).
   Starttls,
}
