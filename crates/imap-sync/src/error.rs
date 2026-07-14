//! Errors raised by the sync layer.

use std::{
   io,
   result::Result as StdResult,
};

use async_imap::error::Error as AsyncImapError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
   #[error("imap error: {0}")]
   Imap(#[from] AsyncImapError),

   #[error("postgres error: {0}")]
   Pg(#[from] tokio_postgres::Error),

   #[error("postgres pool: {0}")]
   PgPool(#[from] deadpool_postgres::PoolError),

   #[error("oauth: {0}")]
   OAuth(String),

   #[error("tls: {0}")]
   Tls(String),

   #[error("io: {0}")]
   Io(#[from] io::Error),

   #[error("http: {0}")]
   Http(#[from] reqwest::Error),

   #[error("smtp: {0}")]
   Smtp(#[from] mail_send::Error),

   #[error("server does not advertise the required IDLE capability")]
   IdleUnsupported,

   #[error("UIDVALIDITY changed on {folder}: was {was}, now {now}")]
   UidValidityChanged {
      folder: String,
      was:    u32,
      now:    u32,
   },

   /// Mailbox/set destroy on a folder with children. Distinct variant so the
   /// server layer can map it to the spec's `mailboxHasChild` `SetError`.
   #[error("mailbox {0} has child mailboxes")]
   MailboxHasChild(String),

   #[error("{0}")]
   Other(String),
}

pub type Result<T> = StdResult<T, SyncError>;
