//! Shared application state passed to every handler.
//!
//! The mutable portion — per-account bearer tokens and mpsc senders — lives
//! behind [`arc_swap::ArcSwap`] so a `SIGHUP` handler can reconcile accounts
//! atomically without waking any in-flight request.

use std::{
   collections::HashMap,
   fmt,
   sync::Arc,
};

use arc_swap::ArcSwap;
use dav_sync::service::DavHandle;
use deadpool_postgres::Pool;
use imap_sync::account::AccountRequest;
use tokio::sync::{
   broadcast,
   mpsc,
};

/// JMAP state-change notification (RFC 8620 §7.1 / §7.3).
#[derive(Debug, Clone)]
pub struct StateChange {
   pub account_id: String,
   pub kind:       StateKind,
   pub new_state:  String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StateKind {
   Email,
   Mailbox,
   Thread,
   EmailSubmission,
   Calendar,
   CalendarEvent,
   AddressBook,
   ContactCard,
}

impl StateKind {
   #[must_use]
   pub const fn as_jmap_type(self) -> &'static str {
      match self {
         Self::Email => "Email",
         Self::Mailbox => "Mailbox",
         Self::Thread => "Thread",
         Self::EmailSubmission => "EmailSubmission",
         Self::Calendar => "Calendar",
         Self::CalendarEvent => "CalendarEvent",
         Self::AddressBook => "AddressBook",
         Self::ContactCard => "ContactCard",
      }
   }

   #[must_use]
   pub fn parse(name: &str) -> Option<Self> {
      match name {
         "Email" => Some(Self::Email),
         "Mailbox" => Some(Self::Mailbox),
         "Thread" => Some(Self::Thread),
         "EmailSubmission" => Some(Self::EmailSubmission),
         "Calendar" => Some(Self::Calendar),
         "CalendarEvent" => Some(Self::CalendarEvent),
         "AddressBook" => Some(Self::AddressBook),
         "ContactCard" => Some(Self::ContactCard),
         _ => None,
      }
   }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DavAvailability {
   pub calendars: bool,
   pub contacts:  bool,
}

impl DavAvailability {
   #[must_use]
   pub const fn any(self) -> bool {
      self.calendars || self.contacts
   }
}

#[derive(Clone)]
pub struct AppState {
   pool:          Pool,
   session_url:   Arc<str>,
   live:          Arc<ArcSwap<AppStateLive>>,
   /// Bounded SSE fan-out; lagged clients recover by refreshing state.
   state_changes: broadcast::Sender<StateChange>,
}

impl fmt::Debug for AppState {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      let live = self.live();
      f.debug_struct("AppState")
         .field("session_url", &&*self.session_url)
         .field("accounts", &live.accounts.len())
         .finish_non_exhaustive()
   }
}

/// The hot-swappable half of the state. Handlers see a consistent snapshot
/// for the duration of a single request.
pub struct AppStateLive {
   pub accounts:         Vec<AccountInfo>,
   pub account_senders:  HashMap<String, mpsc::Sender<AccountRequest>>,
   pub dav_handles:      HashMap<String, DavHandle>,
   pub dav_availability: HashMap<String, DavAvailability>,
}

impl AppState {
   #[must_use]
   pub fn new(
      pool: Pool,
      accounts: Vec<AccountInfo>,
      session_url: String,
      account_senders: HashMap<String, mpsc::Sender<AccountRequest>>,
   ) -> Self {
      Self::new_with_dav(
         pool,
         accounts,
         session_url,
         account_senders,
         HashMap::new(),
         HashMap::new(),
      )
   }

   #[must_use]
   pub fn new_with_dav(
      pool: Pool,
      accounts: Vec<AccountInfo>,
      session_url: String,
      account_senders: HashMap<String, mpsc::Sender<AccountRequest>>,
      dav_handles: HashMap<String, DavHandle>,
      dav_availability: HashMap<String, DavAvailability>,
   ) -> Self {
      let live = AppStateLive {
         accounts,
         account_senders,
         dav_handles,
         dav_availability,
      };
      let (state_changes, _) = broadcast::channel(256);
      Self {
         pool,
         session_url: Arc::from(session_url),
         live: Arc::new(ArcSwap::from_pointee(live)),
         state_changes,
      }
   }

   pub fn publish_state_change(&self, change: StateChange) {
      let thread_change = (change.kind == StateKind::Email).then(|| {
         StateChange {
            account_id: change.account_id.clone(),
            kind:       StateKind::Thread,
            new_state:  change.new_state.clone(),
         }
      });
      let _ = self.state_changes.send(change);
      if let Some(change) = thread_change {
         let _ = self.state_changes.send(change);
      }
   }

   #[must_use]
   pub fn state_changes(&self) -> broadcast::Receiver<StateChange> {
      self.state_changes.subscribe()
   }

   #[must_use]
   pub const fn pool(&self) -> &Pool {
      &self.pool
   }

   #[must_use]
   pub fn session_url(&self) -> &str {
      &self.session_url
   }

   /// Cheap snapshot of the live state. Hold the returned `Arc` only as long
   /// as the handler needs it — a SIGHUP reload swaps the `Arc` under us but
   /// existing snapshots stay valid.
   #[must_use]
   pub fn live(&self) -> Arc<AppStateLive> {
      self.live.load_full()
   }

   /// Atomically replace the live state. Called only by the reload path.
   pub fn swap(&self, next: AppStateLive) {
      self.live.store(Arc::new(next));
   }

   #[must_use]
   pub fn accounts(&self) -> Vec<AccountInfo> {
      self.live().accounts.clone()
   }

   #[must_use]
   pub fn account_sender(&self, account_id: &str) -> Option<mpsc::Sender<AccountRequest>> {
      self.live().account_senders.get(account_id).cloned()
   }

   #[must_use]
   pub fn dav_handle(&self, account_id: &str) -> Option<DavHandle> {
      self.live().dav_handles.get(account_id).cloned()
   }

   #[must_use]
   pub fn dav_availability(&self, account_id: &str) -> DavAvailability {
      self
         .live()
         .dav_availability
         .get(account_id)
         .copied()
         .unwrap_or_default()
   }

   #[must_use]
   pub fn account_by_bearer(&self, token: &str) -> Option<AccountInfo> {
      use sha2::{
         Digest as _,
         Sha256,
      };
      use subtle::ConstantTimeEq as _;
      let hash = {
         let mut hasher = Sha256::new();
         hasher.update(token.as_bytes());
         hasher.finalize()
      };
      self
         .live()
         .accounts
         .iter()
         .find(|account| account.bearer_token_hash.ct_eq(hash.as_slice()).into())
         .cloned()
   }

   #[must_use]
   pub fn account_by_basic(&self, username: &str, token: &str) -> Option<AccountInfo> {
      self
         .account_by_bearer(token)
         .filter(|account| account.id == username || account.email.eq_ignore_ascii_case(username))
   }
}

#[derive(Debug, Clone)]
pub struct AccountInfo {
   pub id:                String,
   pub email:             String,
   pub display_name:      String,
   /// SHA-256 of the bearer token. We keep only the hash in process memory
   /// so a core dump or logging mishap doesn't leak the token itself.
   /// SHA-256 (not SHA-1) was chosen to match what other bits of the stack
   /// use for integrity, and to avoid a SHA-1 deprecation flag surface.
   pub bearer_token_hash: [u8; 32],
}

impl AccountInfo {
   pub fn from_bearer_token(
      id: impl Into<String>,
      email: impl Into<String>,
      display_name: impl Into<String>,
      token: &str,
   ) -> Self {
      use sha2::{
         Digest as _,
         Sha256,
      };
      let mut hasher = Sha256::new();
      hasher.update(token.as_bytes());
      let digest = hasher.finalize();
      let mut arr = [0_u8; 32];
      arr.copy_from_slice(&digest);
      Self {
         id:                id.into(),
         email:             email.into(),
         display_name:      display_name.into(),
         bearer_token_hash: arr,
      }
   }
}
