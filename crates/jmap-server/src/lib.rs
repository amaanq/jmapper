//! HTTP layer — the JMAP-facing server.
//!
//! Hosts `/.well-known/jmap`, `/api`, and CORS preflight. Method dispatch is
//! in [`api`]; actual `Email/*` / `Mailbox/*` logic lives in sibling crates.

pub mod api;
pub mod auth;
pub mod blob;
pub mod error;
pub mod events;
pub mod methods;
pub mod mime;
pub mod observability;
pub mod resolve;
pub mod router;
pub mod scheduler;
pub mod session;
pub mod state;
pub mod upload;

pub use router::build_router;
pub use state::{
   AccountInfo,
   AppState,
};
