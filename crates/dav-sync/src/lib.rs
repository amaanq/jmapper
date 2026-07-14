//! CalDAV/CardDAV transport, conversion, persistence, and account sync.
//!
//! [`dav`] provides `WebDAV` operations, [`caldav`] and [`carddav`] add
//! discovery, [`convert`] adapts calcard's data model to JMAP JSON, and
//! [`service`] exposes the per-account sync task used by `jmap-server`.

pub mod caldav;
pub mod carddav;
pub mod convert;
pub mod dav;
pub mod engine;
pub mod error;
pub mod http;
pub mod service;
pub mod store;
