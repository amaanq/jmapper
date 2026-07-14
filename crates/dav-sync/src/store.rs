//! Typed persistence over the cornucopia-generated `queries::dav`
//! statements.
//!
//! Every function is generic over the client so the same call
//! works on a pooled connection and inside a transaction — the sync engine
//! leans on that to keep resource writes, tombstones, and sync-token
//! advances atomic.

use std::fmt;

use deadpool_postgres::Pool;
use jmapper_codegen::{
   client::GenericClient,
   queries::dav as dav_queries,
};

use crate::{
   dav::href,
   error::{
      DavError,
      Result,
   },
   http::Credentials,
};

/// # Errors
///
/// Returns the pool acquisition error.
#[inline]
pub async fn client(pool: &Pool) -> Result<deadpool_postgres::Object> {
   Ok(pool.get().await?)
}

/// Which DAV protocol an endpoint speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DavKind {
   CalDav,
   CardDav,
}

impl DavKind {
   #[must_use]
   #[inline]
   pub const fn as_str(self) -> &'static str {
      match self {
         Self::CalDav => "caldav",
         Self::CardDav => "carddav",
      }
   }

   #[must_use]
   #[inline]
   pub const fn collection_kind(self) -> &'static str {
      match self {
         Self::CalDav => "calendar",
         Self::CardDav => "addressbook",
      }
   }

   #[must_use]
   #[inline]
   pub const fn resource_kind(self) -> &'static str {
      match self {
         Self::CalDav => "event",
         Self::CardDav => "card",
      }
   }

   #[must_use]
   #[inline]
   pub fn parse(value: &str) -> Option<Self> {
      match value {
         "caldav" => Some(Self::CalDav),
         "carddav" => Some(Self::CardDav),
         _ => None,
      }
   }
}

/// One configured DAV endpoint for an account.
#[derive(Clone)]
pub struct DavEndpoint {
   pub account_id:      String,
   pub kind:            DavKind,
   pub base_url:        String,
   pub auth_kind:       String,
   pub auth_user:       Option<String>,
   pub auth_secret:     Option<String>,
   pub principal_href:  Option<String>,
   pub home_href:       Option<String>,
   pub last_sync_at:    Option<i64>,
   pub last_sync_error: Option<String>,
}

impl fmt::Debug for DavEndpoint {
   #[inline]
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
      f.debug_struct("DavEndpoint")
         .field("account_id", &self.account_id)
         .field("kind", &self.kind)
         .field("base_url", &self.base_url)
         .field("auth_kind", &self.auth_kind)
         .field("auth_user", &self.auth_user)
         .field(
            "auth_secret",
            &self.auth_secret.as_ref().map(|_| "[REDACTED]"),
         )
         .field("principal_href", &self.principal_href)
         .field("home_href", &self.home_href)
         .field("last_sync_at", &self.last_sync_at)
         .field("last_sync_error", &self.last_sync_error)
         .finish()
   }
}

impl DavEndpoint {
   /// # Errors
   ///
   /// Returns an error when endpoint credentials are incomplete or invalid.
   #[inline]
   pub fn credentials(&self) -> Result<Credentials> {
      Ok(match self.auth_kind.as_str() {
         "basic" => {
            Credentials::Basic {
               username: self.auth_user.clone().ok_or_else(|| {
                  DavError::Other(format!(
                     "missing DAV basic username for {}",
                     self.account_id
                  ))
               })?,
               password: self.auth_secret.clone().ok_or_else(|| {
                  DavError::Other(format!(
                     "missing DAV basic password for {}",
                     self.account_id
                  ))
               })?,
            }
         },
         "bearer" => {
            Credentials::Bearer {
               token: self.auth_secret.clone().ok_or_else(|| {
                  DavError::Other(format!("missing DAV bearer token for {}", self.account_id))
               })?,
            }
         },
         "none" => Credentials::None,
         other => {
            return Err(DavError::Other(format!(
               "invalid DAV auth kind {other:?} for {}",
               self.account_id
            )));
         },
      })
   }
}

fn endpoint_from_row(row: dav_queries::DavAccountRow) -> Result<DavEndpoint> {
   let kind = DavKind::parse(&row.kind).ok_or_else(|| {
      DavError::Other(format!(
         "invalid DAV endpoint kind {:?} for {}",
         row.kind, row.account_id
      ))
   })?;
   Ok(DavEndpoint {
      kind,
      account_id: row.account_id,
      base_url: row.base_url,
      auth_kind: row.auth_kind,
      auth_user: row.auth_user,
      auth_secret: row.auth_secret,
      principal_href: row.principal_href,
      home_href: row.home_href,
      last_sync_at: row.last_sync_at,
      last_sync_error: row.last_sync_error,
   })
}

/// # Errors
///
/// Returns the database error encountered while updating the endpoint.
#[inline]
pub async fn upsert_endpoint<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   base_url: &str,
   auth_kind: &str,
   auth_user: Option<&str>,
   auth_secret: Option<&str>,
) -> Result<bool>
where
   C: GenericClient,
{
   let force_resync = get_endpoint(client, account_id, kind)
      .await?
      .is_none_or(|existing| existing.base_url != base_url);
   dav_queries::upsert_dav_account()
      .bind(
         client,
         &account_id,
         &kind.as_str(),
         &base_url,
         &auth_kind,
         &auth_user,
         &auth_secret,
      )
      .await?;
   dav_queries::ensure_dav_state()
      .bind(client, &account_id)
      .await?;
   Ok(force_resync)
}

/// # Errors
///
/// Returns the database error encountered while deleting the endpoint.
#[inline]
pub async fn delete_endpoint<C>(client: &C, account_id: &str, kind: DavKind) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::delete_dav_account()
      .bind(client, &account_id, &kind.as_str())
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while loading the endpoint.
#[inline]
pub async fn get_endpoint<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
) -> Result<Option<DavEndpoint>>
where
   C: GenericClient,
{
   dav_queries::get_dav_account()
      .bind(client, &account_id, &kind.as_str())
      .opt()
      .await?
      .map(endpoint_from_row)
      .transpose()
}

/// # Errors
///
/// Returns the database error encountered while listing endpoints.
#[inline]
pub async fn list_endpoints<C>(client: &C) -> Result<Vec<DavEndpoint>>
where
   C: GenericClient,
{
   dav_queries::list_dav_accounts()
      .bind(client)
      .all()
      .await?
      .into_iter()
      .map(endpoint_from_row)
      .collect::<Result<Vec<DavEndpoint>>>()
}

/// # Errors
///
/// Returns the database error encountered while updating discovery state.
#[inline]
pub async fn set_discovery<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   principal_href: &str,
   home_href: &str,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::set_dav_discovery()
      .bind(
         client,
         &principal_href,
         &home_href,
         &account_id,
         &kind.as_str(),
      )
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while recording successful sync.
#[inline]
pub async fn set_sync_ok<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   last_sync_at: i64,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::set_dav_sync_ok()
      .bind(client, &last_sync_at, &account_id, &kind.as_str())
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while recording failed sync.
#[inline]
pub async fn set_sync_error<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   error: &str,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::set_dav_sync_error()
      .bind(client, &error, &account_id, &kind.as_str())
      .await?;
   Ok(())
}

/// The four independent JMAP state streams.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DavState {
   pub calendar_modseq:       i64,
   pub calendar_event_modseq: i64,
   pub addressbook_modseq:    i64,
   pub contact_card_modseq:   i64,
}

/// # Errors
///
/// Returns the database error encountered while loading state.
#[inline]
pub async fn get_state<C>(client: &C, account_id: &str) -> Result<DavState>
where
   C: GenericClient,
{
   Ok(dav_queries::get_dav_state()
      .bind(client, &account_id)
      .opt()
      .await?
      .map(|row| {
         DavState {
            calendar_modseq:       row.calendar_modseq,
            calendar_event_modseq: row.calendar_event_modseq,
            addressbook_modseq:    row.addressbook_modseq,
            contact_card_modseq:   row.contact_card_modseq,
         }
      })
      .unwrap_or_default())
}

/// # Errors
///
/// Returns the database error encountered while advancing collection state.
#[inline]
pub async fn bump_collection_modseq<C>(client: &C, account_id: &str, kind: DavKind) -> Result<i64>
where
   C: GenericClient,
{
   Ok(match kind {
      DavKind::CalDav => {
         dav_queries::bump_calendar_modseq()
            .bind(client, &account_id)
            .one()
            .await?
      },
      DavKind::CardDav => {
         dav_queries::bump_addressbook_modseq()
            .bind(client, &account_id)
            .one()
            .await?
      },
   })
}

/// # Errors
///
/// Returns the database error encountered while advancing resource state.
#[inline]
pub async fn bump_resource_modseq<C>(client: &C, account_id: &str, kind: DavKind) -> Result<i64>
where
   C: GenericClient,
{
   Ok(match kind {
      DavKind::CalDav => {
         dav_queries::bump_calendar_event_modseq()
            .bind(client, &account_id)
            .one()
            .await?
      },
      DavKind::CardDav => {
         dav_queries::bump_contact_card_modseq()
            .bind(client, &account_id)
            .one()
            .await?
      },
   })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionRow {
   pub account_id:     String,
   pub id:             String,
   pub kind:           String,
   pub href:           String,
   pub name:           String,
   pub color:          Option<String>,
   pub description:    Option<String>,
   pub sync_token:     Option<String>,
   pub supports_sync:  bool,
   pub created_modseq: i64,
   pub modseq:         i64,
   pub destroyed:      bool,
}

impl From<dav_queries::DavCollectionRow> for CollectionRow {
   #[inline]
   fn from(row: dav_queries::DavCollectionRow) -> Self {
      Self {
         account_id:     row.account_id,
         id:             row.id,
         kind:           row.kind,
         href:           row.href,
         name:           row.name,
         color:          row.color,
         description:    row.description,
         sync_token:     row.sync_token,
         supports_sync:  row.supports_sync != 0,
         created_modseq: row.created_modseq,
         modseq:         row.modseq,
         destroyed:      row.destroyed != 0,
      }
   }
}

#[expect(
   clippy::too_many_arguments,
   reason = "mirrors the collection row columns one-to-one"
)]
/// # Errors
///
/// Returns the database error encountered while saving the collection.
#[inline]
pub async fn upsert_collection<C>(
   client: &C,
   account_id: &str,
   id: &str,
   kind: DavKind,
   href: &str,
   name: &str,
   color: Option<&str>,
   description: Option<&str>,
   sync_token: Option<&str>,
   supports_sync: bool,
   modseq: i64,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::upsert_dav_collection()
      .bind(
         client,
         &account_id,
         &id,
         &kind.collection_kind(),
         &href,
         &name,
         &color,
         &description,
         &sync_token,
         &i64::from(supports_sync),
         &modseq,
      )
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while loading the collection.
#[inline]
pub async fn get_collection<C>(
   client: &C,
   account_id: &str,
   id: &str,
) -> Result<Option<CollectionRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::get_dav_collection()
      .bind(client, &account_id, &id)
      .opt()
      .await?
      .map(CollectionRow::from))
}

/// # Errors
///
/// Returns the database error encountered while loading the collection.
#[inline]
pub async fn get_collection_by_href<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   href: &str,
) -> Result<Option<CollectionRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::get_dav_collection_by_href()
      .bind(client, &account_id, &kind.collection_kind(), &href)
      .opt()
      .await?
      .map(CollectionRow::from))
}

/// # Errors
///
/// Returns the database error encountered while listing collections.
#[inline]
pub async fn list_collections<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
) -> Result<Vec<CollectionRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::list_dav_collections()
      .bind(client, &account_id, &kind.collection_kind())
      .all()
      .await?
      .into_iter()
      .map(CollectionRow::from)
      .collect())
}

/// # Errors
///
/// Returns the database error encountered while saving the sync token.
#[inline]
pub async fn set_collection_sync_token<C>(
   client: &C,
   account_id: &str,
   id: &str,
   sync_token: Option<&str>,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::set_dav_collection_sync_token()
      .bind(client, &sync_token, &account_id, &id)
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while tombstoning the collection.
#[inline]
pub async fn tombstone_collection<C>(
   client: &C,
   account_id: &str,
   id: &str,
   modseq: i64,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::tombstone_dav_collection()
      .bind(client, &modseq, &account_id, &id)
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while listing changed collections.
#[inline]
pub async fn collections_changed_since<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   modseq: i64,
) -> Result<Vec<CollectionRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::dav_collections_changed_since()
      .bind(client, &account_id, &kind.collection_kind(), &modseq)
      .all()
      .await?
      .into_iter()
      .map(CollectionRow::from)
      .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRow {
   pub account_id:     String,
   pub id:             String,
   pub collection_id:  String,
   pub kind:           String,
   pub href:           String,
   pub etag:           Option<String>,
   pub uid:            String,
   pub raw:            String,
   pub json:           String,
   pub created_modseq: i64,
   pub modseq:         i64,
   pub destroyed:      bool,
}

impl From<dav_queries::DavResourceRow> for ResourceRow {
   #[inline]
   fn from(row: dav_queries::DavResourceRow) -> Self {
      Self {
         account_id:     row.account_id,
         id:             row.id,
         collection_id:  row.collection_id,
         kind:           row.kind,
         href:           row.href,
         etag:           row.etag,
         uid:            row.uid,
         raw:            row.raw,
         json:           row.json,
         created_modseq: row.created_modseq,
         modseq:         row.modseq,
         destroyed:      row.destroyed != 0,
      }
   }
}

/// href/etag pair of a live cached resource, for the ETag-listing diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceEtag {
   pub id:   String,
   pub href: String,
   pub etag: Option<String>,
}

#[expect(
   clippy::too_many_arguments,
   reason = "mirrors the resource row columns one-to-one"
)]
/// # Errors
///
/// Returns the database error encountered while saving the resource.
#[inline]
pub async fn upsert_resource<C>(
   client: &C,
   account_id: &str,
   id: &str,
   collection_id: &str,
   kind: DavKind,
   href: &str,
   etag: Option<&str>,
   uid: &str,
   raw: &str,
   json: &str,
   modseq: i64,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::upsert_dav_resource()
      .bind(
         client,
         &account_id,
         &id,
         &collection_id,
         &kind.resource_kind(),
         &href,
         &etag,
         &uid,
         &raw,
         &json,
         &modseq,
      )
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while loading the resource.
#[inline]
pub async fn get_resource<C>(client: &C, account_id: &str, id: &str) -> Result<Option<ResourceRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::get_dav_resource()
      .bind(client, &account_id, &id)
      .opt()
      .await?
      .map(ResourceRow::from))
}

/// # Errors
///
/// Returns the database error encountered while loading the resource.
#[inline]
pub async fn get_resource_by_href<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   href: &str,
) -> Result<Option<ResourceRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::get_dav_resource_by_href()
      .bind(client, &account_id, &kind.resource_kind(), &href)
      .opt()
      .await?
      .map(ResourceRow::from))
}

/// # Errors
///
/// Returns the database error encountered while listing resources.
#[inline]
pub async fn list_resources<C>(
   client: &C,
   account_id: &str,
   collection_id: &str,
) -> Result<Vec<ResourceRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::list_dav_resources()
      .bind(client, &account_id, &collection_id)
      .all()
      .await?
      .into_iter()
      .map(ResourceRow::from)
      .collect())
}

/// # Errors
///
/// Returns the database error encountered while listing resources.
#[inline]
pub async fn list_resources_by_kind<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
) -> Result<Vec<ResourceRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::list_dav_resources_by_kind()
      .bind(client, &account_id, &kind.resource_kind())
      .all()
      .await?
      .into_iter()
      .map(ResourceRow::from)
      .collect())
}

/// # Errors
///
/// Returns the database error encountered while listing resource etags.
#[inline]
pub async fn list_resource_etags<C>(
   client: &C,
   account_id: &str,
   collection_id: &str,
) -> Result<Vec<ResourceEtag>>
where
   C: GenericClient,
{
   Ok(dav_queries::list_dav_resource_etags()
      .bind(client, &account_id, &collection_id)
      .all()
      .await?
      .into_iter()
      .map(|resource| {
         ResourceEtag {
            id:   resource.id,
            href: resource.href,
            etag: resource.etag,
         }
      })
      .collect())
}

/// # Errors
///
/// Returns the database error encountered while tombstoning the resource.
#[inline]
pub async fn tombstone_resource<C>(
   client: &C,
   account_id: &str,
   id: &str,
   modseq: i64,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::tombstone_dav_resource()
      .bind(client, &modseq, &account_id, &id)
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while tombstoning collection
/// resources.
#[inline]
pub async fn tombstone_resources_in_collection<C>(
   client: &C,
   account_id: &str,
   collection_id: &str,
   modseq: i64,
) -> Result<()>
where
   C: GenericClient,
{
   dav_queries::tombstone_dav_resources_in_collection()
      .bind(client, &modseq, &account_id, &collection_id)
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while listing changed resources.
#[inline]
pub async fn resources_changed_since<C>(
   client: &C,
   account_id: &str,
   kind: DavKind,
   modseq: i64,
) -> Result<Vec<ResourceRow>>
where
   C: GenericClient,
{
   Ok(dav_queries::dav_resources_changed_since()
      .bind(client, &account_id, &kind.resource_kind(), &modseq)
      .all()
      .await?
      .into_iter()
      .map(ResourceRow::from)
      .collect())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySnapshot {
   pub modseq:     i64,
   pub ids:        Vec<String>,
   pub created_at: i64,
   pub expires_at: i64,
}

#[expect(
   clippy::too_many_arguments,
   reason = "mirrors the snapshot row columns one-to-one"
)]
/// # Errors
///
/// Returns the database error encountered while saving the snapshot.
#[inline]
pub async fn save_query_snapshot<C>(
   client: &C,
   account_id: &str,
   kind: &str,
   query_hash: &str,
   modseq: i64,
   ids: &[String],
   created_at: i64,
   expires_at: i64,
) -> Result<()>
where
   C: GenericClient,
{
   let ids_json = serde_json::to_string(ids)
      .map_err(|err| DavError::Other(format!("serialize query ids: {err}")))?;
   dav_queries::upsert_dav_query_snapshot()
      .bind(
         client,
         &account_id,
         &kind,
         &query_hash,
         &modseq,
         &ids_json.as_str(),
         &created_at,
         &expires_at,
      )
      .await?;
   Ok(())
}

/// # Errors
///
/// Returns the database error encountered while loading the snapshot.
#[inline]
pub async fn get_query_snapshot<C>(
   client: &C,
   account_id: &str,
   kind: &str,
   query_hash: &str,
   modseq: i64,
   now: i64,
) -> Result<Option<QuerySnapshot>>
where
   C: GenericClient,
{
   let row = dav_queries::get_dav_query_snapshot()
      .bind(client, &account_id, &kind, &query_hash, &modseq, &now)
      .opt()
      .await?;
   Ok(row.map(|snapshot| {
      QuerySnapshot {
         modseq:     snapshot.modseq,
         ids:        serde_json::from_str(&snapshot.ids_json).unwrap_or_default(),
         created_at: snapshot.created_at,
         expires_at: snapshot.expires_at,
      }
   }))
}

/// # Errors
///
/// Returns the database error encountered while deleting expired snapshots.
#[inline]
pub async fn delete_expired_query_snapshots<C>(client: &C, now: i64) -> Result<u64>
where
   C: GenericClient,
{
   Ok(dav_queries::delete_expired_dav_query_snapshots()
      .bind(client, &now)
      .await?)
}

/// Stable collection id: hex SHA-1 of the DAV protocol namespace and
/// normalized href.
///
/// `CalDAV` and `CardDAV` frequently use identical paths on
/// different endpoints, so the protocol discriminator is part of the id.
#[must_use]
#[inline]
pub fn id_for_href(kind: DavKind, href: &str) -> String {
   opaque_id(kind, &href::normalize_href(href))
}

/// Stable resource id: the protocol namespace plus the JSCalendar/JSContact
/// uid. Unlike an href-derived id, this survives a DAV MOVE.
#[must_use]
#[inline]
pub fn id_for_uid(kind: DavKind, uid: &str) -> String {
   opaque_id(kind, uid)
}

fn opaque_id(kind: DavKind, value: &str) -> String {
   use sha1::{
      Digest as _,
      Sha1,
   };
   let mut hasher = Sha1::new();
   hasher.update(kind.as_str().as_bytes());
   hasher.update(b"\0");
   hasher.update(value.as_bytes());
   hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn ids_are_protocol_scoped_and_resource_ids_survive_moves() {
      assert_ne!(
         id_for_href(DavKind::CalDav, "/dav/default/"),
         id_for_href(DavKind::CardDav, "/dav/default/")
      );
      assert_eq!(
         id_for_uid(DavKind::CalDav, "event-1"),
         id_for_uid(DavKind::CalDav, "event-1")
      );
   }
}
