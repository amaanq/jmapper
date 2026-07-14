//! JMAP protocol types. Pure data — no I/O, no async.
//!
//! Implements the shapes from RFC 8620 (core) and RFC 8621 (mail) that the
//! server emits and ingests. Filter and comparator ASTs live here so they can
//! be compiled into SQL by `imap-sync` without a crate cycle.

pub mod email;
pub mod error;
pub mod filter;
pub mod ids;
pub mod mailbox;
pub mod method;
pub mod methods;
pub mod session;
pub mod thread;
