//! IMAP fetch, per-account cache, and sync loop.
//!
//! Per-backend quirks (Gmail vs generic IMAP — authentication, special-use
//! folder mapping) are parameterized by [`provider::ProviderKind`] + the
//! runtime structs in [`account`]. The sync loop drives a long-lived
//! `async-imap` connection in IDLE and projects envelope state into the
//! PostgreSQL cache laid out in [`db`].

pub mod account;
pub mod cache;
pub mod db;
pub mod error;
pub mod imap;
pub mod oauth;
pub mod provider;
pub mod smtp;
pub mod sync;
pub mod testkit;
pub mod threading;
