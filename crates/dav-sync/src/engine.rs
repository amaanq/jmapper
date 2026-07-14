//! Initial + incremental CalDAV/CardDAV synchronization.
//!
//! State-advance discipline: each collection's resource writes, tombstones,
//! and sync-token update commit in ONE transaction. A failure anywhere —
//! network, XML, DB — aborts the transaction, so the stored token still
//! describes the last fully-applied round and the retry replays the same
//! window. Upserts key on the href-derived id and tombstoning is
//! idempotent, so replays converge instead of duplicating.
//!
//! A resource that fetches but fails conversion is the one deliberate
//! exception: it is skipped, counted in [`SyncStats::parse_failures`], and
//! logged at ERROR — but the token still advances. Refusing to advance
//! would let a single permanently-malformed object freeze the whole
//! collection, including every future valid change. The raw payload is
//! not cached in that case; it is refetched whenever its `ETag` changes.

use std::collections::{
   BTreeMap,
   BTreeSet,
};

use deadpool_postgres::Pool;
use futures::stream::{
   self,
   StreamExt as _,
};

use crate::{
   caldav,
   carddav,
   convert::{
      calendar,
      contact,
   },
   dav::client::{
      DavClient,
      MemberEtag,
   },
   error::{
      DavError,
      Result,
   },
   store::{
      self,
      DavEndpoint,
      DavKind,
   },
};

/// Collections synced concurrently per endpoint.
const COLLECTION_CONCURRENCY: usize = 4;
/// Member hrefs per multiget REPORT.
const MULTIGET_CHUNK: usize = 50;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncStats {
   pub collections_seen:    u64,
   pub collections_created: u64,
   pub collections_updated: u64,
   pub collections_removed: u64,
   pub resources_created:   u64,
   pub resources_updated:   u64,
   pub resources_removed:   u64,
   /// Collections that fell back to the `ETag` listing because the server
   /// rejected or doesn't support sync-collection.
   pub etag_fallbacks:      u64,
   /// `(href, error)` for resources skipped because conversion failed.
   pub parse_failures:      Vec<(String, String)>,
}

impl SyncStats {
   fn merge(&mut self, other: Self) {
      self.collections_seen += other.collections_seen;
      self.collections_created += other.collections_created;
      self.collections_updated += other.collections_updated;
      self.collections_removed += other.collections_removed;
      self.resources_created += other.resources_created;
      self.resources_updated += other.resources_updated;
      self.resources_removed += other.resources_removed;
      self.etag_fallbacks += other.etag_fallbacks;
      self.parse_failures.extend(other.parse_failures);
   }
}

/// Protocol-agnostic view of a discovered remote collection.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RemoteCollection {
   href:          String,
   name:          String,
   color:         Option<String>,
   description:   Option<String>,
   supports_sync: bool,
}

/// Run one full sync round for an endpoint: discovery (cached), collection
/// reconciliation, then per-collection member sync. Records
/// `last_sync_at`/`last_sync_error` on the endpoint row.
///
/// # Errors
///
/// Returns the synchronization or persistence error encountered during the
/// round.
#[inline]
pub async fn sync_endpoint(
   pool: &Pool,
   endpoint: &DavEndpoint,
   force_full: bool,
   now: i64,
) -> Result<SyncStats> {
   let client = DavClient::new(&endpoint.base_url, endpoint.credentials()?)?;
   let result = sync_endpoint_inner(pool, &client, endpoint, force_full).await;

   let connection = store::client(pool).await?;
   match result.as_ref() {
      Ok(stats) => {
         store::set_sync_ok(&connection, &endpoint.account_id, endpoint.kind, now).await?;
         tracing::info!(
             account = %endpoint.account_id,
             kind = endpoint.kind.as_str(),
             created = stats.resources_created,
             updated = stats.resources_updated,
             removed = stats.resources_removed,
             parse_failures = stats.parse_failures.len(),
             "dav sync completed"
         );
      },
      Err(err) => {
         store::set_sync_error(
            &connection,
            &endpoint.account_id,
            endpoint.kind,
            &err.to_string(),
         )
         .await?;
         tracing::error!(
             account = %endpoint.account_id,
             kind = endpoint.kind.as_str(),
             error = %err,
             "dav sync failed"
         );
      },
   }
   result
}

async fn sync_endpoint_inner(
   pool: &Pool,
   client: &DavClient,
   endpoint: &DavEndpoint,
   force_full: bool,
) -> Result<SyncStats> {
   let home = ensure_discovery(pool, client, endpoint).await?;
   let remote = discover_collections(client, endpoint.kind, &home).await?;
   let mut stats = reconcile_collections(pool, endpoint, &remote).await?;
   stats.collections_seen = remote.len() as u64;

   let live = {
      let connection = store::client(pool).await?;
      store::list_collections(&connection, &endpoint.account_id, endpoint.kind).await?
   };

   let collection_stats = stream::iter(live)
      .map(|col| async move { sync_collection(pool, client, endpoint, col, force_full).await })
      .buffer_unordered(COLLECTION_CONCURRENCY)
      .collect::<Vec<Result<SyncStats>>>()
      .await;
   for collection_result in collection_stats {
      stats.merge(collection_result?);
   }
   Ok(stats)
}

/// Resolve (and cache) principal + home-set hrefs.
async fn ensure_discovery(
   pool: &Pool,
   client: &DavClient,
   endpoint: &DavEndpoint,
) -> Result<String> {
   if endpoint.principal_href.is_some()
      && let Some(home) = endpoint.home_href.as_ref()
   {
      return Ok(home.clone());
   }
   let start = client.conn().base().path().to_owned();
   let principal = caldav::discover_principal(client, &start).await?;
   let home = match endpoint.kind {
      DavKind::CalDav => caldav::discover_calendar_home(client, &principal).await?,
      DavKind::CardDav => carddav::discover_addressbook_home(client, &principal).await?,
   };
   let connection = store::client(pool).await?;
   store::set_discovery(
      &connection,
      &endpoint.account_id,
      endpoint.kind,
      &principal,
      &home,
   )
   .await?;
   Ok(home)
}

async fn discover_collections(
   client: &DavClient,
   kind: DavKind,
   home: &str,
) -> Result<Vec<RemoteCollection>> {
   Ok(match kind {
      DavKind::CalDav => {
         caldav::list_calendars(client, home)
            .await?
            .into_iter()
            .map(|collection| {
               RemoteCollection {
                  href:          collection.href,
                  name:          collection.name,
                  color:         collection.color,
                  description:   collection.description,
                  supports_sync: collection.supports_sync,
               }
            })
            .collect()
      },
      DavKind::CardDav => {
         carddav::list_addressbooks(client, home)
            .await?
            .into_iter()
            .map(|collection| {
               RemoteCollection {
                  href:          collection.href,
                  name:          collection.name,
                  color:         None,
                  description:   collection.description,
                  supports_sync: collection.supports_sync,
               }
            })
            .collect()
      },
   })
}

/// Upsert new/changed collections and tombstone vanished ones (with their
/// resources), all in one transaction with at most one modseq bump per
/// stream.
async fn reconcile_collections(
   pool: &Pool,
   endpoint: &DavEndpoint,
   remote: &[RemoteCollection],
) -> Result<SyncStats> {
   let account_id = &endpoint.account_id;
   let mut stats = SyncStats::default();

   let mut connection = store::client(pool).await?;
   let tx = connection.transaction().await?;

   let local = store::list_collections(&tx, account_id, endpoint.kind).await?;
   let local_by_id = local
      .iter()
      .map(|collection| (collection.id.clone(), collection))
      .collect::<BTreeMap<String, &store::CollectionRow>>();
   let remote_ids = remote
      .iter()
      .map(|resource| store::id_for_href(endpoint.kind, &resource.href))
      .collect::<BTreeSet<String>>();

   let mut collection_modseq = None::<i64>;
   let mut resource_modseq = None::<i64>;

   for rc in remote {
      let id = store::id_for_href(endpoint.kind, &rc.href);
      let existing = local_by_id.get(&id);
      let unchanged = existing.is_some_and(|collection| {
         collection.name == rc.name
            && collection.color == rc.color
            && collection.description == rc.description
            && collection.supports_sync == rc.supports_sync
      });
      if unchanged {
         continue;
      }
      if collection_modseq.is_none() {
         collection_modseq =
            Some(store::bump_collection_modseq(&tx, account_id, endpoint.kind).await?);
      }
      // Preserve the stored sync token: metadata changes must not force
      // a full member resync.
      let token = existing.and_then(|collection| collection.sync_token.clone());
      store::upsert_collection(
         &tx,
         account_id,
         &id,
         endpoint.kind,
         &rc.href,
         &rc.name,
         rc.color.as_deref(),
         rc.description.as_deref(),
         token.as_deref(),
         rc.supports_sync,
         collection_modseq.expect("set above"),
      )
      .await?;
      if existing.is_some() {
         stats.collections_updated += 1;
      } else {
         stats.collections_created += 1;
      }
   }

   for collection in &local {
      if remote_ids.contains(&collection.id) {
         continue;
      }
      if collection_modseq.is_none() {
         collection_modseq =
            Some(store::bump_collection_modseq(&tx, account_id, endpoint.kind).await?);
      }
      if resource_modseq.is_none() {
         resource_modseq = Some(store::bump_resource_modseq(&tx, account_id, endpoint.kind).await?);
      }
      store::tombstone_collection(
         &tx,
         account_id,
         &collection.id,
         collection_modseq.expect("set above"),
      )
      .await?;
      store::tombstone_resources_in_collection(
         &tx,
         account_id,
         &collection.id,
         resource_modseq.expect("set above"),
      )
      .await?;
      stats.collections_removed += 1;
   }

   tx.commit().await?;
   Ok(stats)
}

/// What a change-detection round produced, before fetching bodies.
struct ChangeSet {
   to_fetch:      Vec<String>,
   removed_ids:   Vec<String>,
   new_token:     Option<String>,
   used_fallback: bool,
}

async fn sync_collection(
   pool: &Pool,
   client: &DavClient,
   endpoint: &DavEndpoint,
   collection: store::CollectionRow,
   force_full: bool,
) -> Result<SyncStats> {
   let account_id = &endpoint.account_id;
   let local_etags = {
      let connection = store::client(pool).await?;
      store::list_resource_etags(&connection, account_id, &collection.id).await?
   };

   let changes = detect_changes(client, &collection, &local_etags, force_full).await?;
   let local_ids_by_href = local_etags
      .iter()
      .map(|row| (row.href.clone(), row.id.clone()))
      .collect::<BTreeMap<String, String>>();
   let mut removed_ids = changes
      .removed_ids
      .into_iter()
      .collect::<BTreeSet<String>>();

   let mut fetched = Vec::new();
   for chunk in changes.to_fetch.chunks(MULTIGET_CHUNK) {
      let batch = match endpoint.kind {
         DavKind::CalDav => caldav::multiget(client, &collection.href, chunk).await?,
         DavKind::CardDav => carddav::multiget(client, &collection.href, chunk).await?,
      };
      fetched.extend(batch.fetched);
      removed_ids.extend(
         batch
            .removed
            .iter()
            .filter_map(|href| local_ids_by_href.get(href).cloned()),
      );
   }

   let mut stats = SyncStats {
      etag_fallbacks: u64::from(changes.used_fallback),
      ..SyncStats::default()
   };

   let mut connection = store::client(pool).await?;
   let tx = connection.transaction().await?;
   let mut modseq = None::<i64>;
   let mut fetched_ids = BTreeSet::<String>::new();

   for resource in &fetched {
      let (uid, json) = match convert_resource(endpoint.kind, &resource.data) {
         Ok(converted) => converted,
         Err(err) => {
            tracing::error!(
                account = %account_id,
                href = %resource.href,
                error = %err,
                "dav resource failed conversion; skipped"
            );
            stats
               .parse_failures
               .push((resource.href.clone(), err.to_string()));
            // If this href used to contain a valid object, keeping that
            // stale version visible after the server replaced it with an
            // unreadable payload is worse than omitting it. Tombstone the
            // cached object while still allowing the collection token to
            // advance past the permanently malformed resource.
            if let Some(id) = local_ids_by_href.get(&resource.href)
               && store::get_resource(&tx, account_id, id)
                  .await?
                  .is_some_and(|row| !row.destroyed)
            {
               if modseq.is_none() {
                  modseq = Some(store::bump_resource_modseq(&tx, account_id, endpoint.kind).await?);
               }
               store::tombstone_resource(&tx, account_id, id, modseq.expect("set above")).await?;
               stats.resources_removed += 1;
            }
            continue;
         },
      };
      let id = store::id_for_uid(endpoint.kind, &uid);
      if !fetched_ids.insert(id.clone()) {
         return Err(DavError::Other(format!(
            "DAV server returned duplicate uid {uid:?} in one sync round"
         )));
      }
      if let Some(replaced_id) = local_ids_by_href
         .get(&resource.href)
         .filter(|replaced_id| *replaced_id != &id)
      {
         // Same href, new uid: this is a destroy+create, not an update of
         // the old JMAP object. Remove the old row before the href-unique
         // upsert of the replacement.
         if store::get_resource(&tx, account_id, replaced_id)
            .await?
            .is_some_and(|row| !row.destroyed)
         {
            if modseq.is_none() {
               modseq = Some(store::bump_resource_modseq(&tx, account_id, endpoint.kind).await?);
            }
            store::tombstone_resource(&tx, account_id, replaced_id, modseq.expect("set above"))
               .await?;
            stats.resources_removed += 1;
         }
      }
      let existing = store::get_resource(&tx, account_id, &id).await?;
      let unchanged = existing.as_ref().is_some_and(|stored_resource| {
         !stored_resource.destroyed
            && stored_resource.collection_id == collection.id
            && stored_resource.href == resource.href
            && stored_resource.etag == resource.etag
            && stored_resource.json == json
      });
      if unchanged {
         continue;
      }
      if modseq.is_none() {
         modseq = Some(store::bump_resource_modseq(&tx, account_id, endpoint.kind).await?);
      }
      store::upsert_resource(
         &tx,
         account_id,
         &id,
         &collection.id,
         endpoint.kind,
         &resource.href,
         resource.etag.as_deref(),
         &uid,
         &resource.data,
         &json,
         modseq.expect("set above"),
      )
      .await?;
      if existing.is_some_and(|stored_resource| !stored_resource.destroyed) {
         stats.resources_updated += 1;
      } else {
         stats.resources_created += 1;
      }
   }

   for id in &removed_ids {
      // A MOVE can appear as "new href changed + old href removed" in one
      // sync report. Resource ids are uid-based, so the fetched upsert has
      // already preserved the object under this id.
      if fetched_ids.contains(id) {
         continue;
      }
      if store::get_resource(&tx, account_id, id)
         .await?
         .is_none_or(|stored_resource| stored_resource.destroyed)
      {
         continue;
      }
      if modseq.is_none() {
         modseq = Some(store::bump_resource_modseq(&tx, account_id, endpoint.kind).await?);
      }
      store::tombstone_resource(&tx, account_id, id, modseq.expect("set above")).await?;
      stats.resources_removed += 1;
   }

   store::set_collection_sync_token(
      &tx,
      account_id,
      &collection.id,
      changes.new_token.as_deref(),
   )
   .await?;
   tx.commit().await?;
   Ok(stats)
}

async fn detect_changes(
   client: &DavClient,
   col: &store::CollectionRow,
   local_etags: &[store::ResourceEtag],
   force_full: bool,
) -> Result<ChangeSet> {
   let local_by_href = local_etags
      .iter()
      .map(|resource| (resource.href.as_str(), resource))
      .collect::<BTreeMap<&str, &store::ResourceEtag>>();

   // Full listing (initial sync, forced resync, invalidated token, or no
   // sync-collection support): every reported member is a candidate and
   // locals absent from the listing are removals.
   let full = |members: Vec<MemberEtag>,
               new_token: Option<String>,
               used_fallback: bool,
               fetch_all: bool| {
      let reported = members
         .iter()
         .map(|member| member.href.as_str())
         .collect::<BTreeSet<&str>>();
      let to_fetch = members
         .iter()
         .filter(|member| {
            fetch_all
               || local_by_href.get(member.href.as_str()).is_none_or(|local| {
                  local.etag.is_none() || member.etag.is_none() || local.etag != member.etag
               })
         })
         .map(|member| member.href.clone())
         .collect();
      let removed_ids = local_etags
         .iter()
         .filter(|local| !reported.contains(local.href.as_str()))
         .map(|local| local.id.clone())
         .collect();
      ChangeSet {
         to_fetch,
         removed_ids,
         new_token,
         used_fallback,
      }
   };

   if col.supports_sync {
      let token = if force_full {
         None
      } else {
         col.sync_token.as_deref()
      };
      match client.sync_collection(&col.href, token).await {
         Ok(sc) if token.is_some() => {
            // Incremental: the server enumerated exactly what changed.
            let to_fetch = sc
               .changed
               .iter()
               .filter(|member| {
                  local_by_href.get(member.href.as_str()).is_none_or(|local| {
                     local.etag.is_none() || member.etag.is_none() || local.etag != member.etag
                  })
               })
               .map(|member| member.href.clone())
               .collect();
            let removed_ids = sc
               .removed
               .iter()
               .filter_map(|href| local_by_href.get(href.as_str()))
               .map(|local| local.id.clone())
               .collect();
            return Ok(ChangeSet {
               to_fetch,
               removed_ids,
               new_token: Some(sc.new_token),
               used_fallback: false,
            });
         },
         Ok(sc) => return Ok(full(sc.changed, Some(sc.new_token), false, force_full)),
         Err(DavError::SyncTokenInvalid { .. }) => {
            // Stale token: rerun as an initial sync-collection round.
            let sc = client.sync_collection(&col.href, None).await?;
            return Ok(full(sc.changed, Some(sc.new_token), false, false));
         },
         Err(DavError::Status {
            status: 400 | 403 | 501,
            ..
         }) => {
            // Advertised but not honored: degrade to the ETag listing.
         },
         Err(err) => return Err(err),
      }
   }

   let members = client.list_etags(&col.href).await?;
   Ok(full(members, None, true, force_full))
}

/// Parse + normalize one raw payload; returns `(uid, canonical json)`.
///
/// # Errors
///
/// Returns a conversion or serialization error for the remote resource.
#[inline]
pub fn convert_resource(kind: DavKind, raw: &str) -> Result<(String, String)> {
   match kind {
      DavKind::CalDav => {
         let (uid, event) = calendar::ical_to_event(raw)?;
         let json = serde_json::to_string(&event)
            .map_err(|err| DavError::Other(format!("serialize event: {err}")))?;
         Ok((uid, json))
      },
      DavKind::CardDav => {
         let (uid, card) = contact::vcard_to_card(raw)?;
         let json = serde_json::to_string(&card)
            .map_err(|err| DavError::Other(format!("serialize card: {err}")))?;
         Ok((uid, json))
      },
   }
}
