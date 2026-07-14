//! Per-account DAV service task.
//!
//! An mpsc-driven loop that owns the
//! account's DAV traffic, mirroring the account-task shape `imap-sync`
//! uses for mail. `jmap-server` method handlers talk to it through
//! [`DavHandle`]; requests are processed strictly in order, which is the
//! per-account concurrency bound (cross-account parallelism comes from
//! running one task per account).
//!
//! Writes go remote-first: the PUT/DELETE/MOVE must succeed on the server
//! before the cache row changes, so a transport failure leaves the cache
//! consistent with the server and the caller sees the typed error.

use deadpool_postgres::Pool;
use tokio::{
   sync::{
      mpsc,
      oneshot,
   },
   task::JoinHandle,
};

use crate::{
   dav::{
      client::DavClient,
      href,
   },
   engine::{
      self,
      SyncStats,
   },
   error::{
      DavError,
      Result,
   },
   store::{
      self,
      DavKind,
      ResourceRow,
   },
};

/// Requests a [`DavHandle`] can send to the account task.
pub enum DavRequest {
   /// Run a sync round now. `force_full` ignores stored sync tokens.
   SyncNow {
      kind:       DavKind,
      force_full: bool,
      respond:    oneshot::Sender<Result<SyncStats>>,
   },
   /// Drop all sync tokens and resync from scratch.
   ForceResync {
      kind:    DavKind,
      respond: oneshot::Sender<Result<SyncStats>>,
   },
   /// Create (`resource_id: None`) or update a resource. `raw` is the
   /// iCalendar/vCard payload; it is validated locally before the PUT.
   PutResource {
      kind:          DavKind,
      collection_id: String,
      resource_id:   Option<String>,
      raw:           String,
      respond:       oneshot::Sender<Result<ResourceRow>>,
   },
   DeleteResource {
      kind:        DavKind,
      resource_id: String,
      respond:     oneshot::Sender<Result<()>>,
   },
   /// MOVE a resource to another collection on the same server.
   MoveResource {
      kind:               DavKind,
      resource_id:        String,
      dest_collection_id: String,
      respond:            oneshot::Sender<Result<ResourceRow>>,
   },
   Shutdown {
      respond: oneshot::Sender<()>,
   },
}

#[derive(Clone)]
pub struct DavHandle {
   tx: mpsc::Sender<DavRequest>,
}

/// The channel closed because the task shut down.
fn gone<T>() -> Result<T> {
   Err(DavError::Other("dav service task is gone".to_owned()))
}

impl DavHandle {
   /// # Errors
   ///
   /// Returns an error when the service task has stopped.
   #[inline]
   pub async fn sync_now(&self, kind: DavKind, force_full: bool) -> Result<SyncStats> {
      let (respond, rx) = oneshot::channel();
      if self
         .tx
         .send(DavRequest::SyncNow {
            kind,
            force_full,
            respond,
         })
         .await
         .is_err()
      {
         return gone();
      }
      rx.await.unwrap_or_else(|_| gone())
   }

   /// # Errors
   ///
   /// Returns an error when the service task has stopped.
   #[inline]
   pub async fn force_resync(&self, kind: DavKind) -> Result<SyncStats> {
      let (respond, rx) = oneshot::channel();
      if self
         .tx
         .send(DavRequest::ForceResync { kind, respond })
         .await
         .is_err()
      {
         return gone();
      }
      rx.await.unwrap_or_else(|_| gone())
   }

   /// # Errors
   ///
   /// Returns an error when the service task has stopped or the resource
   /// operation fails.
   #[inline]
   pub async fn put_resource(
      &self,
      kind: DavKind,
      collection_id: String,
      resource_id: Option<String>,
      raw: String,
   ) -> Result<ResourceRow> {
      let (respond, rx) = oneshot::channel();
      if self
         .tx
         .send(DavRequest::PutResource {
            kind,
            collection_id,
            resource_id,
            raw,
            respond,
         })
         .await
         .is_err()
      {
         return gone();
      }
      rx.await.unwrap_or_else(|_| gone())
   }

   /// # Errors
   ///
   /// Returns an error when the service task has stopped or deletion fails.
   #[inline]
   pub async fn delete_resource(&self, kind: DavKind, resource_id: String) -> Result<()> {
      let (respond, rx) = oneshot::channel();
      if self
         .tx
         .send(DavRequest::DeleteResource {
            kind,
            resource_id,
            respond,
         })
         .await
         .is_err()
      {
         return gone();
      }
      rx.await.unwrap_or_else(|_| gone())
   }

   /// # Errors
   ///
   /// Returns an error when the service task has stopped or moving fails.
   #[inline]
   pub async fn move_resource(
      &self,
      kind: DavKind,
      resource_id: String,
      dest_collection_id: String,
   ) -> Result<ResourceRow> {
      let (respond, rx) = oneshot::channel();
      if self
         .tx
         .send(DavRequest::MoveResource {
            kind,
            resource_id,
            dest_collection_id,
            respond,
         })
         .await
         .is_err()
      {
         return gone();
      }
      rx.await.unwrap_or_else(|_| gone())
   }

   #[inline]
   pub async fn shutdown(&self) {
      let (respond, rx) = oneshot::channel();
      if self.tx.send(DavRequest::Shutdown { respond }).await.is_ok() {
         let _ = rx.await;
      }
   }

   #[must_use]
   #[inline]
   pub fn is_closed(&self) -> bool {
      self.tx.is_closed()
   }
}

pub struct DavTask {
   pub handle: DavHandle,
   pub task:   JoinHandle<()>,
}

/// Spawn the account task. Returns the handle method handlers keep.
#[must_use]
#[inline]
pub fn spawn(pool: Pool, account_id: String) -> DavHandle {
   let task = spawn_managed(pool, account_id);
   task.handle
}

/// Spawn an account task while retaining its join handle for daemon-owned
/// lifecycle management and supervision.
#[must_use]
#[inline]
pub fn spawn_managed(pool: Pool, account_id: String) -> DavTask {
   let (tx, rx) = mpsc::channel::<DavRequest>(32);
   let handle = DavHandle { tx };
   let task = tokio::spawn(run(pool, account_id, rx));
   DavTask { handle, task }
}

async fn run(pool: Pool, account_id: String, mut rx: mpsc::Receiver<DavRequest>) {
   while let Some(req) = rx.recv().await {
      match req {
         DavRequest::SyncNow {
            kind,
            force_full,
            respond,
         } => {
            let _ = respond.send(do_sync(&pool, &account_id, kind, force_full).await);
         },
         DavRequest::ForceResync { kind, respond } => {
            let _ = respond.send(do_sync(&pool, &account_id, kind, true).await);
         },
         DavRequest::PutResource {
            kind,
            collection_id,
            resource_id,
            raw,
            respond,
         } => {
            let _ = respond
               .send(do_put(&pool, &account_id, kind, &collection_id, resource_id, raw).await);
         },
         DavRequest::DeleteResource {
            kind,
            resource_id,
            respond,
         } => {
            let _ = respond.send(do_delete(&pool, &account_id, kind, &resource_id).await);
         },
         DavRequest::MoveResource {
            kind,
            resource_id,
            dest_collection_id,
            respond,
         } => {
            let _ = respond
               .send(do_move(&pool, &account_id, kind, &resource_id, &dest_collection_id).await);
         },
         DavRequest::Shutdown { respond } => {
            let _ = respond.send(());
            break;
         },
      }
   }
   tracing::debug!(account = %account_id, "dav service task exiting");
}

async fn endpoint_client(
   pool: &Pool,
   account_id: &str,
   kind: DavKind,
) -> Result<(store::DavEndpoint, DavClient)> {
   let connection = store::client(pool).await?;
   let endpoint = store::get_endpoint(&connection, account_id, kind)
      .await?
      .ok_or_else(|| DavError::Other(format!("no {} endpoint for {account_id}", kind.as_str())))?;
   let client = DavClient::new(&endpoint.base_url, endpoint.credentials()?)?;
   Ok((endpoint, client))
}

async fn do_sync(
   pool: &Pool,
   account_id: &str,
   kind: DavKind,
   force_full: bool,
) -> Result<SyncStats> {
   let connection = store::client(pool).await?;
   let endpoint = store::get_endpoint(&connection, account_id, kind)
      .await?
      .ok_or_else(|| DavError::Other(format!("no {} endpoint for {account_id}", kind.as_str())))?;
   drop(connection);
   let now = chrono::Utc::now().timestamp();
   engine::sync_endpoint(pool, &endpoint, force_full, now).await
}

const fn content_type(kind: DavKind) -> &'static str {
   match kind {
      DavKind::CalDav => "text/calendar; charset=utf-8",
      DavKind::CardDav => "text/vcard; charset=utf-8",
   }
}

const fn extension(kind: DavKind) -> &'static str {
   match kind {
      DavKind::CalDav => ".ics",
      DavKind::CardDav => ".vcf",
   }
}

async fn do_put(
   pool: &Pool,
   account_id: &str,
   kind: DavKind,
   collection_id: &str,
   resource_id: Option<String>,
   raw: String,
) -> Result<ResourceRow> {
   let (uid, _) = engine::convert_resource(kind, &raw)?;
   let (_endpoint, client) = endpoint_client(pool, account_id, kind).await?;

   let connection = store::client(pool).await?;
   let collection = store::get_collection(&connection, account_id, collection_id)
      .await?
      .filter(|col| !col.destroyed && col.kind == kind.collection_kind())
      .ok_or_else(|| DavError::Other(format!("unknown collection {collection_id}")))?;

   let existing = match resource_id.as_ref() {
      Some(id) => {
         Some(
            store::get_resource(&connection, account_id, id)
               .await?
               .filter(|resource| !resource.destroyed && resource.kind == kind.resource_kind())
               .ok_or_else(|| DavError::Other(format!("unknown resource {id}")))?,
         )
      },
      None => None,
   };
   if let Some(row) = existing.as_ref() {
      if row.collection_id != collection_id {
         return Err(DavError::Other(format!(
            "resource {} belongs to collection {}; MOVE it before updating",
            row.id, row.collection_id
         )));
      }
      if row.uid != uid {
         return Err(DavError::Other(
            "a DAV resource uid cannot be changed by an update".to_owned(),
         ));
      }
   } else {
      let id = store::id_for_uid(kind, &uid);
      if store::get_resource(&connection, account_id, &id)
         .await?
         .is_some_and(|row| !row.destroyed)
      {
         return Err(DavError::Other(format!(
            "a {} with uid {uid:?} already exists",
            kind.resource_kind()
         )));
      }
   }
   drop(connection);

   let (href, if_match) = if let Some(resource) = existing.as_ref() {
      (resource.href.clone(), resource.etag.clone())
   } else {
      let name = format!(
         "{}{}",
         percent_encoding::utf8_percent_encode(&uid, percent_encoding::NON_ALPHANUMERIC),
         extension(kind)
      );
      let base = collection.href.trim_end_matches('/');
      (href::normalize_href(&format!("{base}/{name}")), None)
   };

   client
      .put(&href, content_type(kind), raw, if_match.as_deref())
      .await?;
   // ETag presence does not imply the server stored our bytes verbatim.
   // Always fetch the authoritative representation after PUT so server-side
   // normalization cannot leave the cache disagreeing with the backend.
   let fetched = client.get(&href).await?;
   let (stored_uid, stored_json) = engine::convert_resource(kind, &fetched.data)?;
   if stored_uid != uid {
      return Err(DavError::Other(format!(
         "DAV server rewrote resource uid from {uid:?} to {stored_uid:?}"
      )));
   }

   let mut cache_connection = store::client(pool).await?;
   let tx = cache_connection.transaction().await?;
   let modseq = store::bump_resource_modseq(&tx, account_id, kind).await?;
   let id = existing.as_ref().map_or_else(
      || store::id_for_uid(kind, &stored_uid),
      |resource| resource.id.clone(),
   );
   store::upsert_resource(
      &tx,
      account_id,
      &id,
      collection_id,
      kind,
      &href,
      fetched.etag.as_deref(),
      &stored_uid,
      &fetched.data,
      &stored_json,
      modseq,
   )
   .await?;
   let row = store::get_resource(&tx, account_id, &id)
      .await?
      .ok_or_else(|| DavError::Other(format!("resource {id} missing after upsert")))?;
   tx.commit().await?;
   Ok(row)
}

async fn do_delete(pool: &Pool, account_id: &str, kind: DavKind, resource_id: &str) -> Result<()> {
   let (_endpoint, client) = endpoint_client(pool, account_id, kind).await?;
   let connection = store::client(pool).await?;
   let row = store::get_resource(&connection, account_id, resource_id)
      .await?
      .filter(|resource| !resource.destroyed && resource.kind == kind.resource_kind())
      .ok_or_else(|| DavError::Other(format!("unknown resource {resource_id}")))?;
   drop(connection);

   client.delete(&row.href, row.etag.as_deref()).await?;

   let mut cache_connection = store::client(pool).await?;
   let tx = cache_connection.transaction().await?;
   let modseq = store::bump_resource_modseq(&tx, account_id, kind).await?;
   store::tombstone_resource(&tx, account_id, resource_id, modseq).await?;
   tx.commit().await?;
   Ok(())
}

async fn do_move(
   pool: &Pool,
   account_id: &str,
   kind: DavKind,
   resource_id: &str,
   dest_collection_id: &str,
) -> Result<ResourceRow> {
   let (_endpoint, client) = endpoint_client(pool, account_id, kind).await?;
   let connection = store::client(pool).await?;
   let row = store::get_resource(&connection, account_id, resource_id)
      .await?
      .filter(|resource| !resource.destroyed && resource.kind == kind.resource_kind())
      .ok_or_else(|| DavError::Other(format!("unknown resource {resource_id}")))?;
   let dest = store::get_collection(&connection, account_id, dest_collection_id)
      .await?
      .filter(|col| !col.destroyed && col.kind == kind.collection_kind())
      .ok_or_else(|| DavError::Other(format!("unknown collection {dest_collection_id}")))?;
   if row.collection_id == dest_collection_id {
      return Ok(row);
   }
   drop(connection);

   let name = href::last_segment(&row.href);
   let encoded =
      percent_encoding::utf8_percent_encode(&name, percent_encoding::NON_ALPHANUMERIC).to_string();
   let dest_href =
      href::normalize_href(&format!("{}/{}", dest.href.trim_end_matches('/'), encoded));

   client.move_(&row.href, &dest_href).await?;
   // The ETag after MOVE is server-defined; refetch for the fresh one.
   let fetched = client.get(&dest_href).await?;
   let (uid, json) = engine::convert_resource(kind, &fetched.data)?;
   if uid != row.uid {
      return Err(DavError::Other(format!(
         "DAV server rewrote resource uid from {:?} to {uid:?} during MOVE",
         row.uid
      )));
   }

   let mut cache_connection = store::client(pool).await?;
   let tx = cache_connection.transaction().await?;
   let modseq = store::bump_resource_modseq(&tx, account_id, kind).await?;
   store::upsert_resource(
      &tx,
      account_id,
      &row.id,
      dest_collection_id,
      kind,
      &dest_href,
      fetched.etag.as_deref(),
      &uid,
      &fetched.data,
      &json,
      modseq,
   )
   .await?;
   let new_row = store::get_resource(&tx, account_id, &row.id)
      .await?
      .ok_or_else(|| DavError::Other(format!("resource {} missing after move", row.id)))?;
   tx.commit().await?;
   Ok(new_row)
}
