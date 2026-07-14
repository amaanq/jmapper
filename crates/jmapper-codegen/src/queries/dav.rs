// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct UpsertDavAccountParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
> {
    pub account_id: T1,
    pub kind: T2,
    pub base_url: T3,
    pub auth_kind: T4,
    pub auth_user: Option<T5>,
    pub auth_secret: Option<T6>,
}
#[derive(Debug)]
pub struct GetDavAccountParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub kind: T2,
}
#[derive(Debug)]
pub struct SetDavDiscoveryParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
> {
    pub principal_href: T1,
    pub home_href: T2,
    pub account_id: T3,
    pub kind: T4,
}
#[derive(Debug)]
pub struct SetDavSyncOkParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub last_sync_at: i64,
    pub account_id: T1,
    pub kind: T2,
}
#[derive(Debug)]
pub struct SetDavSyncErrorParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql> {
    pub last_sync_error: T1,
    pub account_id: T2,
    pub kind: T3,
}
#[derive(Debug)]
pub struct DeleteDavAccountParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub kind: T2,
}
#[derive(Debug)]
pub struct UpsertDavCollectionParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
    T7: crate::StringSql,
    T8: crate::StringSql,
> {
    pub account_id: T1,
    pub id: T2,
    pub kind: T3,
    pub href: T4,
    pub name: T5,
    pub color: Option<T6>,
    pub description: Option<T7>,
    pub sync_token: Option<T8>,
    pub supports_sync: i64,
    pub modseq: i64,
}
#[derive(Debug)]
pub struct GetDavCollectionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct GetDavCollectionByHrefParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
> {
    pub account_id: T1,
    pub kind: T2,
    pub href: T3,
}
#[derive(Debug)]
pub struct ListDavCollectionsParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub kind: T2,
}
#[derive(Debug)]
pub struct SetDavCollectionSyncTokenParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
> {
    pub sync_token: Option<T1>,
    pub account_id: T2,
    pub id: T3,
}
#[derive(Debug)]
pub struct TombstoneDavCollectionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub modseq: i64,
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct DavCollectionsChangedSinceParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub kind: T2,
    pub modseq: i64,
}
#[derive(Debug)]
pub struct UpsertDavResourceParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
    T7: crate::StringSql,
    T8: crate::StringSql,
    T9: crate::StringSql,
> {
    pub account_id: T1,
    pub id: T2,
    pub collection_id: T3,
    pub kind: T4,
    pub href: T5,
    pub etag: Option<T6>,
    pub uid: T7,
    pub raw: T8,
    pub json: T9,
    pub modseq: i64,
}
#[derive(Debug)]
pub struct GetDavResourceParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct GetDavResourceByHrefParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
> {
    pub account_id: T1,
    pub kind: T2,
    pub href: T3,
}
#[derive(Debug)]
pub struct ListDavResourcesParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub collection_id: T2,
}
#[derive(Debug)]
pub struct ListDavResourcesByKindParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub kind: T2,
}
#[derive(Debug)]
pub struct ListDavResourceEtagsParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub collection_id: T2,
}
#[derive(Debug)]
pub struct TombstoneDavResourceParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub modseq: i64,
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct TombstoneDavResourcesInCollectionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub modseq: i64,
    pub account_id: T1,
    pub collection_id: T2,
}
#[derive(Debug)]
pub struct DavResourcesChangedSinceParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub kind: T2,
    pub modseq: i64,
}
#[derive(Debug)]
pub struct UpsertDavQuerySnapshotParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
> {
    pub account_id: T1,
    pub kind: T2,
    pub query_hash: T3,
    pub modseq: i64,
    pub ids_json: T4,
    pub created_at: i64,
    pub expires_at: i64,
}
#[derive(Debug)]
pub struct GetDavQuerySnapshotParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
> {
    pub account_id: T1,
    pub kind: T2,
    pub query_hash: T3,
    pub modseq: i64,
    pub now: i64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct DavAccountRow {
    pub account_id: String,
    pub kind: String,
    pub base_url: String,
    pub auth_kind: String,
    pub auth_user: Option<String>,
    pub auth_secret: Option<String>,
    pub principal_href: Option<String>,
    pub home_href: Option<String>,
    pub last_sync_at: Option<i64>,
    pub last_sync_error: Option<String>,
}
pub struct DavAccountRowBorrowed<'a> {
    pub account_id: &'a str,
    pub kind: &'a str,
    pub base_url: &'a str,
    pub auth_kind: &'a str,
    pub auth_user: Option<&'a str>,
    pub auth_secret: Option<&'a str>,
    pub principal_href: Option<&'a str>,
    pub home_href: Option<&'a str>,
    pub last_sync_at: Option<i64>,
    pub last_sync_error: Option<&'a str>,
}
impl<'a> From<DavAccountRowBorrowed<'a>> for DavAccountRow {
    fn from(
        DavAccountRowBorrowed {
            account_id,
            kind,
            base_url,
            auth_kind,
            auth_user,
            auth_secret,
            principal_href,
            home_href,
            last_sync_at,
            last_sync_error,
        }: DavAccountRowBorrowed<'a>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            kind: kind.into(),
            base_url: base_url.into(),
            auth_kind: auth_kind.into(),
            auth_user: auth_user.map(|v| v.into()),
            auth_secret: auth_secret.map(|v| v.into()),
            principal_href: principal_href.map(|v| v.into()),
            home_href: home_href.map(|v| v.into()),
            last_sync_at,
            last_sync_error: last_sync_error.map(|v| v.into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct DavStateRow {
    pub calendar_modseq: i64,
    pub calendar_event_modseq: i64,
    pub addressbook_modseq: i64,
    pub contact_card_modseq: i64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct DavCollectionRow {
    pub account_id: String,
    pub id: String,
    pub kind: String,
    pub href: String,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub sync_token: Option<String>,
    pub supports_sync: i64,
    pub created_modseq: i64,
    pub modseq: i64,
    pub destroyed: i64,
}
pub struct DavCollectionRowBorrowed<'a> {
    pub account_id: &'a str,
    pub id: &'a str,
    pub kind: &'a str,
    pub href: &'a str,
    pub name: &'a str,
    pub color: Option<&'a str>,
    pub description: Option<&'a str>,
    pub sync_token: Option<&'a str>,
    pub supports_sync: i64,
    pub created_modseq: i64,
    pub modseq: i64,
    pub destroyed: i64,
}
impl<'a> From<DavCollectionRowBorrowed<'a>> for DavCollectionRow {
    fn from(
        DavCollectionRowBorrowed {
            account_id,
            id,
            kind,
            href,
            name,
            color,
            description,
            sync_token,
            supports_sync,
            created_modseq,
            modseq,
            destroyed,
        }: DavCollectionRowBorrowed<'a>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            id: id.into(),
            kind: kind.into(),
            href: href.into(),
            name: name.into(),
            color: color.map(|v| v.into()),
            description: description.map(|v| v.into()),
            sync_token: sync_token.map(|v| v.into()),
            supports_sync,
            created_modseq,
            modseq,
            destroyed,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DavResourceRow {
    pub account_id: String,
    pub id: String,
    pub collection_id: String,
    pub kind: String,
    pub href: String,
    pub etag: Option<String>,
    pub uid: String,
    pub raw: String,
    pub json: String,
    pub created_modseq: i64,
    pub modseq: i64,
    pub destroyed: i64,
}
pub struct DavResourceRowBorrowed<'a> {
    pub account_id: &'a str,
    pub id: &'a str,
    pub collection_id: &'a str,
    pub kind: &'a str,
    pub href: &'a str,
    pub etag: Option<&'a str>,
    pub uid: &'a str,
    pub raw: &'a str,
    pub json: &'a str,
    pub created_modseq: i64,
    pub modseq: i64,
    pub destroyed: i64,
}
impl<'a> From<DavResourceRowBorrowed<'a>> for DavResourceRow {
    fn from(
        DavResourceRowBorrowed {
            account_id,
            id,
            collection_id,
            kind,
            href,
            etag,
            uid,
            raw,
            json,
            created_modseq,
            modseq,
            destroyed,
        }: DavResourceRowBorrowed<'a>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            id: id.into(),
            collection_id: collection_id.into(),
            kind: kind.into(),
            href: href.into(),
            etag: etag.map(|v| v.into()),
            uid: uid.into(),
            raw: raw.into(),
            json: json.into(),
            created_modseq,
            modseq,
            destroyed,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DavResourceEtagRow {
    pub id: String,
    pub href: String,
    pub etag: Option<String>,
}
pub struct DavResourceEtagRowBorrowed<'a> {
    pub id: &'a str,
    pub href: &'a str,
    pub etag: Option<&'a str>,
}
impl<'a> From<DavResourceEtagRowBorrowed<'a>> for DavResourceEtagRow {
    fn from(DavResourceEtagRowBorrowed { id, href, etag }: DavResourceEtagRowBorrowed<'a>) -> Self {
        Self {
            id: id.into(),
            href: href.into(),
            etag: etag.map(|v| v.into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DavQuerySnapshotRow {
    pub modseq: i64,
    pub ids_json: String,
    pub created_at: i64,
    pub expires_at: i64,
}
pub struct DavQuerySnapshotRowBorrowed<'a> {
    pub modseq: i64,
    pub ids_json: &'a str,
    pub created_at: i64,
    pub expires_at: i64,
}
impl<'a> From<DavQuerySnapshotRowBorrowed<'a>> for DavQuerySnapshotRow {
    fn from(
        DavQuerySnapshotRowBorrowed {
            modseq,
            ids_json,
            created_at,
            expires_at,
        }: DavQuerySnapshotRowBorrowed<'a>,
    ) -> Self {
        Self {
            modseq,
            ids_json: ids_json.into(),
            created_at,
            expires_at,
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct DavAccountRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<DavAccountRowBorrowed, tokio_postgres::Error>,
    mapper: fn(DavAccountRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> DavAccountRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(DavAccountRowBorrowed) -> R,
    ) -> DavAccountRowQuery<'c, 'a, 's, C, R, N> {
        DavAccountRowQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct DavStateRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<DavStateRow, tokio_postgres::Error>,
    mapper: fn(DavStateRow) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> DavStateRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(DavStateRow) -> R) -> DavStateRowQuery<'c, 'a, 's, C, R, N> {
        DavStateRowQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct I64Query<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<i64, tokio_postgres::Error>,
    mapper: fn(i64) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> I64Query<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(i64) -> R) -> I64Query<'c, 'a, 's, C, R, N> {
        I64Query {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct DavCollectionRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<DavCollectionRowBorrowed, tokio_postgres::Error>,
    mapper: fn(DavCollectionRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> DavCollectionRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(DavCollectionRowBorrowed) -> R,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, R, N> {
        DavCollectionRowQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct DavResourceRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<DavResourceRowBorrowed, tokio_postgres::Error>,
    mapper: fn(DavResourceRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> DavResourceRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(DavResourceRowBorrowed) -> R,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, R, N> {
        DavResourceRowQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct DavResourceEtagRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<DavResourceEtagRowBorrowed, tokio_postgres::Error>,
    mapper: fn(DavResourceEtagRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> DavResourceEtagRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(DavResourceEtagRowBorrowed) -> R,
    ) -> DavResourceEtagRowQuery<'c, 'a, 's, C, R, N> {
        DavResourceEtagRowQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct DavQuerySnapshotRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<DavQuerySnapshotRowBorrowed, tokio_postgres::Error>,
    mapper: fn(DavQuerySnapshotRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> DavQuerySnapshotRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(DavQuerySnapshotRowBorrowed) -> R,
    ) -> DavQuerySnapshotRowQuery<'c, 'a, 's, C, R, N> {
        DavQuerySnapshotRowQuery {
            client: self.client,
            params: self.params,
            query: self.query,
            cached: self.cached,
            extractor: self.extractor,
            mapper,
        }
    }
    pub async fn one(self) -> Result<T, tokio_postgres::Error> {
        let row =
            crate::client::async_::one(self.client, self.query, &self.params, self.cached).await?;
        Ok((self.mapper)((self.extractor)(&row)?))
    }
    pub async fn all(self) -> Result<Vec<T>, tokio_postgres::Error> {
        self.iter().await?.try_collect().await
    }
    pub async fn opt(self) -> Result<Option<T>, tokio_postgres::Error> {
        let opt_row =
            crate::client::async_::opt(self.client, self.query, &self.params, self.cached).await?;
        Ok(opt_row
            .map(|row| {
                let extracted = (self.extractor)(&row)?;
                Ok((self.mapper)(extracted))
            })
            .transpose()?)
    }
    pub async fn iter(
        self,
    ) -> Result<
        impl futures::Stream<Item = Result<T, tokio_postgres::Error>> + 'c,
        tokio_postgres::Error,
    > {
        let stream = crate::client::async_::raw(
            self.client,
            self.query,
            crate::slice_iter(&self.params),
            self.cached,
        )
        .await?;
        let mapped = stream
            .map(move |res| {
                res.and_then(|row| {
                    let extracted = (self.extractor)(&row)?;
                    Ok((self.mapper)(extracted))
                })
            })
            .into_stream();
        Ok(mapped)
    }
}
pub struct UpsertDavAccountStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_dav_account() -> UpsertDavAccountStmt {
    UpsertDavAccountStmt(
        "INSERT INTO dav_accounts (account_id, kind, base_url, auth_kind, auth_user, auth_secret) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (account_id, kind) DO UPDATE SET base_url = EXCLUDED.base_url, auth_kind = EXCLUDED.auth_kind, auth_user = EXCLUDED.auth_user, auth_secret = EXCLUDED.auth_secret, principal_href = CASE WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL ELSE dav_accounts.principal_href END, home_href = CASE WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL ELSE dav_accounts.home_href END, last_sync_at = CASE WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL ELSE dav_accounts.last_sync_at END, last_sync_error = CASE WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL ELSE dav_accounts.last_sync_error END",
        None,
    )
}
impl UpsertDavAccountStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
        T4: crate::StringSql,
        T5: crate::StringSql,
        T6: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
        base_url: &'a T3,
        auth_kind: &'a T4,
        auth_user: &'a Option<T5>,
        auth_secret: &'a Option<T6>,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id,
                    kind,
                    base_url,
                    auth_kind,
                    auth_user,
                    auth_secret,
                ],
            )
            .await
    }
}
impl<
    'a,
    C: GenericClient + Send + Sync,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertDavAccountParams<T1, T2, T3, T4, T5, T6>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertDavAccountStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertDavAccountParams<T1, T2, T3, T4, T5, T6>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.kind,
            &params.base_url,
            &params.auth_kind,
            &params.auth_user,
            &params.auth_secret,
        ))
    }
}
pub struct GetDavAccountStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_dav_account() -> GetDavAccountStmt {
    GetDavAccountStmt(
        "SELECT account_id, kind, base_url, auth_kind, auth_user, auth_secret, principal_href, home_href, last_sync_at, last_sync_error FROM dav_accounts WHERE account_id = $1 AND kind = $2",
        None,
    )
}
impl GetDavAccountStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
    ) -> DavAccountRowQuery<'c, 'a, 's, C, DavAccountRow, 2> {
        DavAccountRowQuery {
            client,
            params: [account_id, kind],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<DavAccountRowBorrowed, tokio_postgres::Error> {
                    Ok(DavAccountRowBorrowed {
                        account_id: row.try_get(0)?,
                        kind: row.try_get(1)?,
                        base_url: row.try_get(2)?,
                        auth_kind: row.try_get(3)?,
                        auth_user: row.try_get(4)?,
                        auth_secret: row.try_get(5)?,
                        principal_href: row.try_get(6)?,
                        home_href: row.try_get(7)?,
                        last_sync_at: row.try_get(8)?,
                        last_sync_error: row.try_get(9)?,
                    })
                },
            mapper: |it| DavAccountRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetDavAccountParams<T1, T2>,
        DavAccountRowQuery<'c, 'a, 's, C, DavAccountRow, 2>,
        C,
    > for GetDavAccountStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetDavAccountParams<T1, T2>,
    ) -> DavAccountRowQuery<'c, 'a, 's, C, DavAccountRow, 2> {
        self.bind(client, &params.account_id, &params.kind)
    }
}
pub struct ListDavAccountsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_dav_accounts() -> ListDavAccountsStmt {
    ListDavAccountsStmt(
        "SELECT account_id, kind, base_url, auth_kind, auth_user, auth_secret, principal_href, home_href, last_sync_at, last_sync_error FROM dav_accounts ORDER BY account_id, kind",
        None,
    )
}
impl ListDavAccountsStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
    ) -> DavAccountRowQuery<'c, 'a, 's, C, DavAccountRow, 0> {
        DavAccountRowQuery {
            client,
            params: [],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<DavAccountRowBorrowed, tokio_postgres::Error> {
                    Ok(DavAccountRowBorrowed {
                        account_id: row.try_get(0)?,
                        kind: row.try_get(1)?,
                        base_url: row.try_get(2)?,
                        auth_kind: row.try_get(3)?,
                        auth_user: row.try_get(4)?,
                        auth_secret: row.try_get(5)?,
                        principal_href: row.try_get(6)?,
                        home_href: row.try_get(7)?,
                        last_sync_at: row.try_get(8)?,
                        last_sync_error: row.try_get(9)?,
                    })
                },
            mapper: |it| DavAccountRow::from(it),
        }
    }
}
pub struct SetDavDiscoveryStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_dav_discovery() -> SetDavDiscoveryStmt {
    SetDavDiscoveryStmt(
        "UPDATE dav_accounts SET principal_href = $1, home_href = $2 WHERE account_id = $3 AND kind = $4",
        None,
    )
}
impl SetDavDiscoveryStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
        T4: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        principal_href: &'a T1,
        home_href: &'a T2,
        account_id: &'a T3,
        kind: &'a T4,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[principal_href, home_href, account_id, kind])
            .await
    }
}
impl<
    'a,
    C: GenericClient + Send + Sync,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        SetDavDiscoveryParams<T1, T2, T3, T4>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetDavDiscoveryStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetDavDiscoveryParams<T1, T2, T3, T4>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.principal_href,
            &params.home_href,
            &params.account_id,
            &params.kind,
        ))
    }
}
pub struct SetDavSyncOkStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_dav_sync_ok() -> SetDavSyncOkStmt {
    SetDavSyncOkStmt(
        "UPDATE dav_accounts SET last_sync_at = $1, last_sync_error = NULL WHERE account_id = $2 AND kind = $3",
        None,
    )
}
impl SetDavSyncOkStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        last_sync_at: &'a i64,
        account_id: &'a T1,
        kind: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[last_sync_at, account_id, kind])
            .await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        SetDavSyncOkParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetDavSyncOkStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetDavSyncOkParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.last_sync_at,
            &params.account_id,
            &params.kind,
        ))
    }
}
pub struct SetDavSyncErrorStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_dav_sync_error() -> SetDavSyncErrorStmt {
    SetDavSyncErrorStmt(
        "UPDATE dav_accounts SET last_sync_error = $1 WHERE account_id = $2 AND kind = $3",
        None,
    )
}
impl SetDavSyncErrorStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        last_sync_error: &'a T1,
        account_id: &'a T2,
        kind: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[last_sync_error, account_id, kind])
            .await
    }
}
impl<
    'a,
    C: GenericClient + Send + Sync,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        SetDavSyncErrorParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetDavSyncErrorStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetDavSyncErrorParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.last_sync_error,
            &params.account_id,
            &params.kind,
        ))
    }
}
pub struct DeleteDavAccountStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_dav_account() -> DeleteDavAccountStmt {
    DeleteDavAccountStmt(
        "DELETE FROM dav_accounts WHERE account_id = $1 AND kind = $2",
        None,
    )
}
impl DeleteDavAccountStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id, kind]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        DeleteDavAccountParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for DeleteDavAccountStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a DeleteDavAccountParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.account_id, &params.kind))
    }
}
pub struct EnsureDavStateStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn ensure_dav_state() -> EnsureDavStateStmt {
    EnsureDavStateStmt(
        "INSERT INTO dav_state (account_id) VALUES ($1) ON CONFLICT (account_id) DO NOTHING",
        None,
    )
}
impl EnsureDavStateStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id]).await
    }
}
pub struct GetDavStateStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_dav_state() -> GetDavStateStmt {
    GetDavStateStmt(
        "SELECT calendar_modseq, calendar_event_modseq, addressbook_modseq, contact_card_modseq FROM dav_state WHERE account_id = $1",
        None,
    )
}
impl GetDavStateStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
    ) -> DavStateRowQuery<'c, 'a, 's, C, DavStateRow, 1> {
        DavStateRowQuery {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<DavStateRow, tokio_postgres::Error> {
                Ok(DavStateRow {
                    calendar_modseq: row.try_get(0)?,
                    calendar_event_modseq: row.try_get(1)?,
                    addressbook_modseq: row.try_get(2)?,
                    contact_card_modseq: row.try_get(3)?,
                })
            },
            mapper: |it| DavStateRow::from(it),
        }
    }
}
pub struct BumpCalendarModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bump_calendar_modseq() -> BumpCalendarModseqStmt {
    BumpCalendarModseqStmt(
        "UPDATE dav_state SET calendar_modseq = calendar_modseq + 1 WHERE account_id = $1 RETURNING calendar_modseq",
        None,
    )
}
impl BumpCalendarModseqStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
    ) -> I64Query<'c, 'a, 's, C, i64, 1> {
        I64Query {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
        }
    }
}
pub struct BumpCalendarEventModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bump_calendar_event_modseq() -> BumpCalendarEventModseqStmt {
    BumpCalendarEventModseqStmt(
        "UPDATE dav_state SET calendar_event_modseq = calendar_event_modseq + 1 WHERE account_id = $1 RETURNING calendar_event_modseq",
        None,
    )
}
impl BumpCalendarEventModseqStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
    ) -> I64Query<'c, 'a, 's, C, i64, 1> {
        I64Query {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
        }
    }
}
pub struct BumpAddressbookModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bump_addressbook_modseq() -> BumpAddressbookModseqStmt {
    BumpAddressbookModseqStmt(
        "UPDATE dav_state SET addressbook_modseq = addressbook_modseq + 1 WHERE account_id = $1 RETURNING addressbook_modseq",
        None,
    )
}
impl BumpAddressbookModseqStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
    ) -> I64Query<'c, 'a, 's, C, i64, 1> {
        I64Query {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
        }
    }
}
pub struct BumpContactCardModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bump_contact_card_modseq() -> BumpContactCardModseqStmt {
    BumpContactCardModseqStmt(
        "UPDATE dav_state SET contact_card_modseq = contact_card_modseq + 1 WHERE account_id = $1 RETURNING contact_card_modseq",
        None,
    )
}
impl BumpContactCardModseqStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
    ) -> I64Query<'c, 'a, 's, C, i64, 1> {
        I64Query {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
        }
    }
}
pub struct UpsertDavCollectionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_dav_collection() -> UpsertDavCollectionStmt {
    UpsertDavCollectionStmt(
        "INSERT INTO dav_collections (account_id, id, kind, href, name, color, description, sync_token, supports_sync, created_modseq, modseq) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10) ON CONFLICT (account_id, id) DO UPDATE SET href = EXCLUDED.href, name = EXCLUDED.name, color = EXCLUDED.color, description = EXCLUDED.description, sync_token = EXCLUDED.sync_token, supports_sync = EXCLUDED.supports_sync, created_modseq = CASE WHEN dav_collections.destroyed <> 0 THEN EXCLUDED.created_modseq ELSE dav_collections.created_modseq END, modseq = EXCLUDED.modseq, destroyed = 0",
        None,
    )
}
impl UpsertDavCollectionStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
        T4: crate::StringSql,
        T5: crate::StringSql,
        T6: crate::StringSql,
        T7: crate::StringSql,
        T8: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        id: &'a T2,
        kind: &'a T3,
        href: &'a T4,
        name: &'a T5,
        color: &'a Option<T6>,
        description: &'a Option<T7>,
        sync_token: &'a Option<T8>,
        supports_sync: &'a i64,
        modseq: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id,
                    id,
                    kind,
                    href,
                    name,
                    color,
                    description,
                    sync_token,
                    supports_sync,
                    modseq,
                ],
            )
            .await
    }
}
impl<
    'a,
    C: GenericClient + Send + Sync,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
    T7: crate::StringSql,
    T8: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertDavCollectionParams<T1, T2, T3, T4, T5, T6, T7, T8>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertDavCollectionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertDavCollectionParams<T1, T2, T3, T4, T5, T6, T7, T8>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.id,
            &params.kind,
            &params.href,
            &params.name,
            &params.color,
            &params.description,
            &params.sync_token,
            &params.supports_sync,
            &params.modseq,
        ))
    }
}
pub struct GetDavCollectionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_dav_collection() -> GetDavCollectionStmt {
    GetDavCollectionStmt(
        "SELECT account_id, id, kind, href, name, color, description, sync_token, supports_sync, created_modseq, modseq, destroyed FROM dav_collections WHERE account_id = $1 AND id = $2",
        None,
    )
}
impl GetDavCollectionStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        id: &'a T2,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 2> {
        DavCollectionRowQuery {
            client,
            params: [account_id, id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavCollectionRowBorrowed, tokio_postgres::Error> {
                Ok(DavCollectionRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    kind: row.try_get(2)?,
                    href: row.try_get(3)?,
                    name: row.try_get(4)?,
                    color: row.try_get(5)?,
                    description: row.try_get(6)?,
                    sync_token: row.try_get(7)?,
                    supports_sync: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavCollectionRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetDavCollectionParams<T1, T2>,
        DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 2>,
        C,
    > for GetDavCollectionStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetDavCollectionParams<T1, T2>,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 2> {
        self.bind(client, &params.account_id, &params.id)
    }
}
pub struct GetDavCollectionByHrefStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_dav_collection_by_href() -> GetDavCollectionByHrefStmt {
    GetDavCollectionByHrefStmt(
        "SELECT account_id, id, kind, href, name, color, description, sync_token, supports_sync, created_modseq, modseq, destroyed FROM dav_collections WHERE account_id = $1 AND kind = $2 AND href = $3 AND destroyed = 0",
        None,
    )
}
impl GetDavCollectionByHrefStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
        href: &'a T3,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 3> {
        DavCollectionRowQuery {
            client,
            params: [account_id, kind, href],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavCollectionRowBorrowed, tokio_postgres::Error> {
                Ok(DavCollectionRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    kind: row.try_get(2)?,
                    href: row.try_get(3)?,
                    name: row.try_get(4)?,
                    color: row.try_get(5)?,
                    description: row.try_get(6)?,
                    sync_token: row.try_get(7)?,
                    supports_sync: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavCollectionRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetDavCollectionByHrefParams<T1, T2, T3>,
        DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 3>,
        C,
    > for GetDavCollectionByHrefStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetDavCollectionByHrefParams<T1, T2, T3>,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 3> {
        self.bind(client, &params.account_id, &params.kind, &params.href)
    }
}
pub struct ListDavCollectionsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_dav_collections() -> ListDavCollectionsStmt {
    ListDavCollectionsStmt(
        "SELECT account_id, id, kind, href, name, color, description, sync_token, supports_sync, created_modseq, modseq, destroyed FROM dav_collections WHERE account_id = $1 AND kind = $2 AND destroyed = 0 ORDER BY id",
        None,
    )
}
impl ListDavCollectionsStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 2> {
        DavCollectionRowQuery {
            client,
            params: [account_id, kind],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavCollectionRowBorrowed, tokio_postgres::Error> {
                Ok(DavCollectionRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    kind: row.try_get(2)?,
                    href: row.try_get(3)?,
                    name: row.try_get(4)?,
                    color: row.try_get(5)?,
                    description: row.try_get(6)?,
                    sync_token: row.try_get(7)?,
                    supports_sync: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavCollectionRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        ListDavCollectionsParams<T1, T2>,
        DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 2>,
        C,
    > for ListDavCollectionsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ListDavCollectionsParams<T1, T2>,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 2> {
        self.bind(client, &params.account_id, &params.kind)
    }
}
pub struct SetDavCollectionSyncTokenStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_dav_collection_sync_token() -> SetDavCollectionSyncTokenStmt {
    SetDavCollectionSyncTokenStmt(
        "UPDATE dav_collections SET sync_token = $1 WHERE account_id = $2 AND id = $3",
        None,
    )
}
impl SetDavCollectionSyncTokenStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        sync_token: &'a Option<T1>,
        account_id: &'a T2,
        id: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[sync_token, account_id, id]).await
    }
}
impl<
    'a,
    C: GenericClient + Send + Sync,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        SetDavCollectionSyncTokenParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetDavCollectionSyncTokenStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetDavCollectionSyncTokenParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.sync_token, &params.account_id, &params.id))
    }
}
pub struct TombstoneDavCollectionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn tombstone_dav_collection() -> TombstoneDavCollectionStmt {
    TombstoneDavCollectionStmt(
        "UPDATE dav_collections SET destroyed = 1, modseq = $1 WHERE account_id = $2 AND id = $3 AND destroyed = 0",
        None,
    )
}
impl TombstoneDavCollectionStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        modseq: &'a i64,
        account_id: &'a T1,
        id: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[modseq, account_id, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        TombstoneDavCollectionParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for TombstoneDavCollectionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a TombstoneDavCollectionParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.modseq, &params.account_id, &params.id))
    }
}
pub struct DavCollectionsChangedSinceStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn dav_collections_changed_since() -> DavCollectionsChangedSinceStmt {
    DavCollectionsChangedSinceStmt(
        "SELECT account_id, id, kind, href, name, color, description, sync_token, supports_sync, created_modseq, modseq, destroyed FROM dav_collections WHERE account_id = $1 AND kind = $2 AND modseq > $3 ORDER BY modseq",
        None,
    )
}
impl DavCollectionsChangedSinceStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
        modseq: &'a i64,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 3> {
        DavCollectionRowQuery {
            client,
            params: [account_id, kind, modseq],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavCollectionRowBorrowed, tokio_postgres::Error> {
                Ok(DavCollectionRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    kind: row.try_get(2)?,
                    href: row.try_get(3)?,
                    name: row.try_get(4)?,
                    color: row.try_get(5)?,
                    description: row.try_get(6)?,
                    sync_token: row.try_get(7)?,
                    supports_sync: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavCollectionRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        DavCollectionsChangedSinceParams<T1, T2>,
        DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 3>,
        C,
    > for DavCollectionsChangedSinceStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a DavCollectionsChangedSinceParams<T1, T2>,
    ) -> DavCollectionRowQuery<'c, 'a, 's, C, DavCollectionRow, 3> {
        self.bind(client, &params.account_id, &params.kind, &params.modseq)
    }
}
pub struct UpsertDavResourceStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_dav_resource() -> UpsertDavResourceStmt {
    UpsertDavResourceStmt(
        "INSERT INTO dav_resources (account_id, id, collection_id, kind, href, etag, uid, raw, json, created_modseq, modseq) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $10) ON CONFLICT (account_id, id) DO UPDATE SET collection_id = EXCLUDED.collection_id, href = EXCLUDED.href, etag = EXCLUDED.etag, uid = EXCLUDED.uid, raw = EXCLUDED.raw, json = EXCLUDED.json, created_modseq = CASE WHEN dav_resources.destroyed <> 0 THEN EXCLUDED.created_modseq ELSE dav_resources.created_modseq END, modseq = EXCLUDED.modseq, destroyed = 0",
        None,
    )
}
impl UpsertDavResourceStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
        T4: crate::StringSql,
        T5: crate::StringSql,
        T6: crate::StringSql,
        T7: crate::StringSql,
        T8: crate::StringSql,
        T9: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        id: &'a T2,
        collection_id: &'a T3,
        kind: &'a T4,
        href: &'a T5,
        etag: &'a Option<T6>,
        uid: &'a T7,
        raw: &'a T8,
        json: &'a T9,
        modseq: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id,
                    id,
                    collection_id,
                    kind,
                    href,
                    etag,
                    uid,
                    raw,
                    json,
                    modseq,
                ],
            )
            .await
    }
}
impl<
    'a,
    C: GenericClient + Send + Sync,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
    T7: crate::StringSql,
    T8: crate::StringSql,
    T9: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertDavResourceParams<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertDavResourceStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertDavResourceParams<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.id,
            &params.collection_id,
            &params.kind,
            &params.href,
            &params.etag,
            &params.uid,
            &params.raw,
            &params.json,
            &params.modseq,
        ))
    }
}
pub struct GetDavResourceStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_dav_resource() -> GetDavResourceStmt {
    GetDavResourceStmt(
        "SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json, created_modseq, modseq, destroyed FROM dav_resources WHERE account_id = $1 AND id = $2",
        None,
    )
}
impl GetDavResourceStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        id: &'a T2,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2> {
        DavResourceRowQuery {
            client,
            params: [account_id, id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavResourceRowBorrowed, tokio_postgres::Error> {
                Ok(DavResourceRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    collection_id: row.try_get(2)?,
                    kind: row.try_get(3)?,
                    href: row.try_get(4)?,
                    etag: row.try_get(5)?,
                    uid: row.try_get(6)?,
                    raw: row.try_get(7)?,
                    json: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavResourceRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetDavResourceParams<T1, T2>,
        DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2>,
        C,
    > for GetDavResourceStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetDavResourceParams<T1, T2>,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2> {
        self.bind(client, &params.account_id, &params.id)
    }
}
pub struct GetDavResourceByHrefStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_dav_resource_by_href() -> GetDavResourceByHrefStmt {
    GetDavResourceByHrefStmt(
        "SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json, created_modseq, modseq, destroyed FROM dav_resources WHERE account_id = $1 AND kind = $2 AND href = $3 AND destroyed = 0",
        None,
    )
}
impl GetDavResourceByHrefStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
        href: &'a T3,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 3> {
        DavResourceRowQuery {
            client,
            params: [account_id, kind, href],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavResourceRowBorrowed, tokio_postgres::Error> {
                Ok(DavResourceRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    collection_id: row.try_get(2)?,
                    kind: row.try_get(3)?,
                    href: row.try_get(4)?,
                    etag: row.try_get(5)?,
                    uid: row.try_get(6)?,
                    raw: row.try_get(7)?,
                    json: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavResourceRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetDavResourceByHrefParams<T1, T2, T3>,
        DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 3>,
        C,
    > for GetDavResourceByHrefStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetDavResourceByHrefParams<T1, T2, T3>,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 3> {
        self.bind(client, &params.account_id, &params.kind, &params.href)
    }
}
pub struct ListDavResourcesStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_dav_resources() -> ListDavResourcesStmt {
    ListDavResourcesStmt(
        "SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json, created_modseq, modseq, destroyed FROM dav_resources WHERE account_id = $1 AND collection_id = $2 AND destroyed = 0 ORDER BY id",
        None,
    )
}
impl ListDavResourcesStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        collection_id: &'a T2,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2> {
        DavResourceRowQuery {
            client,
            params: [account_id, collection_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavResourceRowBorrowed, tokio_postgres::Error> {
                Ok(DavResourceRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    collection_id: row.try_get(2)?,
                    kind: row.try_get(3)?,
                    href: row.try_get(4)?,
                    etag: row.try_get(5)?,
                    uid: row.try_get(6)?,
                    raw: row.try_get(7)?,
                    json: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavResourceRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        ListDavResourcesParams<T1, T2>,
        DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2>,
        C,
    > for ListDavResourcesStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ListDavResourcesParams<T1, T2>,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2> {
        self.bind(client, &params.account_id, &params.collection_id)
    }
}
pub struct ListDavResourcesByKindStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_dav_resources_by_kind() -> ListDavResourcesByKindStmt {
    ListDavResourcesByKindStmt(
        "SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json, created_modseq, modseq, destroyed FROM dav_resources WHERE account_id = $1 AND kind = $2 AND destroyed = 0 ORDER BY id",
        None,
    )
}
impl ListDavResourcesByKindStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2> {
        DavResourceRowQuery {
            client,
            params: [account_id, kind],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavResourceRowBorrowed, tokio_postgres::Error> {
                Ok(DavResourceRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    collection_id: row.try_get(2)?,
                    kind: row.try_get(3)?,
                    href: row.try_get(4)?,
                    etag: row.try_get(5)?,
                    uid: row.try_get(6)?,
                    raw: row.try_get(7)?,
                    json: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavResourceRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        ListDavResourcesByKindParams<T1, T2>,
        DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2>,
        C,
    > for ListDavResourcesByKindStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ListDavResourcesByKindParams<T1, T2>,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 2> {
        self.bind(client, &params.account_id, &params.kind)
    }
}
pub struct ListDavResourceEtagsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_dav_resource_etags() -> ListDavResourceEtagsStmt {
    ListDavResourceEtagsStmt(
        "SELECT id, href, etag FROM dav_resources WHERE account_id = $1 AND collection_id = $2 AND destroyed = 0 ORDER BY href",
        None,
    )
}
impl ListDavResourceEtagsStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        collection_id: &'a T2,
    ) -> DavResourceEtagRowQuery<'c, 'a, 's, C, DavResourceEtagRow, 2> {
        DavResourceEtagRowQuery {
            client,
            params: [account_id, collection_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavResourceEtagRowBorrowed, tokio_postgres::Error> {
                Ok(DavResourceEtagRowBorrowed {
                    id: row.try_get(0)?,
                    href: row.try_get(1)?,
                    etag: row.try_get(2)?,
                })
            },
            mapper: |it| DavResourceEtagRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        ListDavResourceEtagsParams<T1, T2>,
        DavResourceEtagRowQuery<'c, 'a, 's, C, DavResourceEtagRow, 2>,
        C,
    > for ListDavResourceEtagsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ListDavResourceEtagsParams<T1, T2>,
    ) -> DavResourceEtagRowQuery<'c, 'a, 's, C, DavResourceEtagRow, 2> {
        self.bind(client, &params.account_id, &params.collection_id)
    }
}
pub struct TombstoneDavResourceStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn tombstone_dav_resource() -> TombstoneDavResourceStmt {
    TombstoneDavResourceStmt(
        "UPDATE dav_resources SET destroyed = 1, modseq = $1 WHERE account_id = $2 AND id = $3 AND destroyed = 0",
        None,
    )
}
impl TombstoneDavResourceStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        modseq: &'a i64,
        account_id: &'a T1,
        id: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[modseq, account_id, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        TombstoneDavResourceParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for TombstoneDavResourceStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a TombstoneDavResourceParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.modseq, &params.account_id, &params.id))
    }
}
pub struct TombstoneDavResourcesInCollectionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn tombstone_dav_resources_in_collection() -> TombstoneDavResourcesInCollectionStmt {
    TombstoneDavResourcesInCollectionStmt(
        "UPDATE dav_resources SET destroyed = 1, modseq = $1 WHERE account_id = $2 AND collection_id = $3 AND destroyed = 0",
        None,
    )
}
impl TombstoneDavResourcesInCollectionStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        modseq: &'a i64,
        account_id: &'a T1,
        collection_id: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[modseq, account_id, collection_id])
            .await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        TombstoneDavResourcesInCollectionParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for TombstoneDavResourcesInCollectionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a TombstoneDavResourcesInCollectionParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.modseq,
            &params.account_id,
            &params.collection_id,
        ))
    }
}
pub struct DavResourcesChangedSinceStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn dav_resources_changed_since() -> DavResourcesChangedSinceStmt {
    DavResourcesChangedSinceStmt(
        "SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json, created_modseq, modseq, destroyed FROM dav_resources WHERE account_id = $1 AND kind = $2 AND modseq > $3 ORDER BY modseq",
        None,
    )
}
impl DavResourcesChangedSinceStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
        modseq: &'a i64,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 3> {
        DavResourceRowQuery {
            client,
            params: [account_id, kind, modseq],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavResourceRowBorrowed, tokio_postgres::Error> {
                Ok(DavResourceRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    collection_id: row.try_get(2)?,
                    kind: row.try_get(3)?,
                    href: row.try_get(4)?,
                    etag: row.try_get(5)?,
                    uid: row.try_get(6)?,
                    raw: row.try_get(7)?,
                    json: row.try_get(8)?,
                    created_modseq: row.try_get(9)?,
                    modseq: row.try_get(10)?,
                    destroyed: row.try_get(11)?,
                })
            },
            mapper: |it| DavResourceRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        DavResourcesChangedSinceParams<T1, T2>,
        DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 3>,
        C,
    > for DavResourcesChangedSinceStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a DavResourcesChangedSinceParams<T1, T2>,
    ) -> DavResourceRowQuery<'c, 'a, 's, C, DavResourceRow, 3> {
        self.bind(client, &params.account_id, &params.kind, &params.modseq)
    }
}
pub struct UpsertDavQuerySnapshotStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_dav_query_snapshot() -> UpsertDavQuerySnapshotStmt {
    UpsertDavQuerySnapshotStmt(
        "INSERT INTO dav_query_snapshots (account_id, kind, query_hash, modseq, ids_json, created_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (account_id, kind, query_hash, modseq) DO UPDATE SET ids_json = EXCLUDED.ids_json, created_at = EXCLUDED.created_at, expires_at = EXCLUDED.expires_at",
        None,
    )
}
impl UpsertDavQuerySnapshotStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
        T4: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
        query_hash: &'a T3,
        modseq: &'a i64,
        ids_json: &'a T4,
        created_at: &'a i64,
        expires_at: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id, kind, query_hash, modseq, ids_json, created_at, expires_at,
                ],
            )
            .await
    }
}
impl<
    'a,
    C: GenericClient + Send + Sync,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertDavQuerySnapshotParams<T1, T2, T3, T4>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertDavQuerySnapshotStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertDavQuerySnapshotParams<T1, T2, T3, T4>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.kind,
            &params.query_hash,
            &params.modseq,
            &params.ids_json,
            &params.created_at,
            &params.expires_at,
        ))
    }
}
pub struct GetDavQuerySnapshotStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_dav_query_snapshot() -> GetDavQuerySnapshotStmt {
    GetDavQuerySnapshotStmt(
        "SELECT modseq, ids_json, created_at, expires_at FROM dav_query_snapshots WHERE account_id = $1 AND kind = $2 AND query_hash = $3 AND modseq = $4 AND expires_at > $5",
        None,
    )
}
impl GetDavQuerySnapshotStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub fn bind<
        'c,
        'a,
        's,
        C: GenericClient,
        T1: crate::StringSql,
        T2: crate::StringSql,
        T3: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        kind: &'a T2,
        query_hash: &'a T3,
        modseq: &'a i64,
        now: &'a i64,
    ) -> DavQuerySnapshotRowQuery<'c, 'a, 's, C, DavQuerySnapshotRow, 5> {
        DavQuerySnapshotRowQuery {
            client,
            params: [account_id, kind, query_hash, modseq, now],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DavQuerySnapshotRowBorrowed, tokio_postgres::Error> {
                Ok(DavQuerySnapshotRowBorrowed {
                    modseq: row.try_get(0)?,
                    ids_json: row.try_get(1)?,
                    created_at: row.try_get(2)?,
                    expires_at: row.try_get(3)?,
                })
            },
            mapper: |it| DavQuerySnapshotRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetDavQuerySnapshotParams<T1, T2, T3>,
        DavQuerySnapshotRowQuery<'c, 'a, 's, C, DavQuerySnapshotRow, 5>,
        C,
    > for GetDavQuerySnapshotStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetDavQuerySnapshotParams<T1, T2, T3>,
    ) -> DavQuerySnapshotRowQuery<'c, 'a, 's, C, DavQuerySnapshotRow, 5> {
        self.bind(
            client,
            &params.account_id,
            &params.kind,
            &params.query_hash,
            &params.modseq,
            &params.now,
        )
    }
}
pub struct DeleteExpiredDavQuerySnapshotsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_expired_dav_query_snapshots() -> DeleteExpiredDavQuerySnapshotsStmt {
    DeleteExpiredDavQuerySnapshotsStmt(
        "DELETE FROM dav_query_snapshots WHERE expires_at <= $1",
        None,
    )
}
impl DeleteExpiredDavQuerySnapshotsStmt {
    pub async fn prepare<'a, C: GenericClient>(
        mut self,
        client: &'a C,
    ) -> Result<Self, tokio_postgres::Error> {
        self.1 = Some(client.prepare(self.0).await?);
        Ok(self)
    }
    pub async fn bind<'c, 'a, 's, C: GenericClient>(
        &'s self,
        client: &'c C,
        now: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[now]).await
    }
}
