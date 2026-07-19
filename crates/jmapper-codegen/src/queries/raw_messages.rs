// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct RawMessageFetchedAtParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct RawMessageProjectionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct RawMessageBytesParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct CachedMessageIdsParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::ArraySql<Item = T2>,
> {
    pub account_id: T1,
    pub msgids: T3,
}
#[derive(Debug)]
pub struct RecentUncachedMessageIdsParams<T1: crate::StringSql> {
    pub account_id: T1,
    pub window: i64,
    pub max_bytes: i64,
    pub limit: i64,
}
#[derive(Debug)]
pub struct UpsertRawMessageParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::BytesSql,
> {
    pub account_id: T1,
    pub msgid: T2,
    pub headers_json: T3,
    pub body_values_json: T4,
    pub attachments_json: T5,
    pub raw_rfc822: T6,
}
#[derive(Debug, Clone, PartialEq)]
pub struct RawBodyRow {
    pub headers_json: String,
    pub body_values_json: String,
    pub attachments_json: String,
}
pub struct RawBodyRowBorrowed<'a> {
    pub headers_json: &'a str,
    pub body_values_json: &'a str,
    pub attachments_json: &'a str,
}
impl<'a> From<RawBodyRowBorrowed<'a>> for RawBodyRow {
    fn from(
        RawBodyRowBorrowed {
            headers_json,
            body_values_json,
            attachments_json,
        }: RawBodyRowBorrowed<'a>,
    ) -> Self {
        Self {
            headers_json: headers_json.into(),
            body_values_json: body_values_json.into(),
            attachments_json: attachments_json.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct RawBytesRow {
    pub raw_rfc822: Vec<u8>,
}
pub struct RawBytesRowBorrowed<'a> {
    pub raw_rfc822: &'a [u8],
}
impl<'a> From<RawBytesRowBorrowed<'a>> for RawBytesRow {
    fn from(RawBytesRowBorrowed { raw_rfc822 }: RawBytesRowBorrowed<'a>) -> Self {
        Self {
            raw_rfc822: raw_rfc822.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct CachedMetadataRepairRow {
    pub msgid: String,
    pub subject: Option<String>,
    pub raw_rfc822: Vec<u8>,
}
pub struct CachedMetadataRepairRowBorrowed<'a> {
    pub msgid: &'a str,
    pub subject: Option<&'a str>,
    pub raw_rfc822: &'a [u8],
}
impl<'a> From<CachedMetadataRepairRowBorrowed<'a>> for CachedMetadataRepairRow {
    fn from(
        CachedMetadataRepairRowBorrowed {
            msgid,
            subject,
            raw_rfc822,
        }: CachedMetadataRepairRowBorrowed<'a>,
    ) -> Self {
        Self {
            msgid: msgid.into(),
            subject: subject.map(|v| v.into()),
            raw_rfc822: raw_rfc822.into(),
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
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
pub struct RawBodyRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<RawBodyRowBorrowed, tokio_postgres::Error>,
    mapper: fn(RawBodyRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> RawBodyRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(RawBodyRowBorrowed) -> R,
    ) -> RawBodyRowQuery<'c, 'a, 's, C, R, N> {
        RawBodyRowQuery {
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
pub struct RawBytesRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<RawBytesRowBorrowed, tokio_postgres::Error>,
    mapper: fn(RawBytesRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> RawBytesRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(RawBytesRowBorrowed) -> R,
    ) -> RawBytesRowQuery<'c, 'a, 's, C, R, N> {
        RawBytesRowQuery {
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
pub struct StringQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<&str, tokio_postgres::Error>,
    mapper: fn(&str) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> StringQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(&str) -> R) -> StringQuery<'c, 'a, 's, C, R, N> {
        StringQuery {
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
pub struct CachedMetadataRepairRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<CachedMetadataRepairRowBorrowed, tokio_postgres::Error>,
    mapper: fn(CachedMetadataRepairRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> CachedMetadataRepairRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(CachedMetadataRepairRowBorrowed) -> R,
    ) -> CachedMetadataRepairRowQuery<'c, 'a, 's, C, R, N> {
        CachedMetadataRepairRowQuery {
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
pub struct RawMessageFetchedAtStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn raw_message_fetched_at() -> RawMessageFetchedAtStmt {
    RawMessageFetchedAtStmt(
        "SELECT fetched_at FROM raw_messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl RawMessageFetchedAtStmt {
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
        msgid: &'a T2,
    ) -> I64Query<'c, 'a, 's, C, i64, 2> {
        I64Query {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        RawMessageFetchedAtParams<T1, T2>,
        I64Query<'c, 'a, 's, C, i64, 2>,
        C,
    > for RawMessageFetchedAtStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a RawMessageFetchedAtParams<T1, T2>,
    ) -> I64Query<'c, 'a, 's, C, i64, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct RawMessageProjectionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn raw_message_projection() -> RawMessageProjectionStmt {
    RawMessageProjectionStmt(
        "SELECT headers_json, body_values_json, attachments_json FROM raw_messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl RawMessageProjectionStmt {
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
        msgid: &'a T2,
    ) -> RawBodyRowQuery<'c, 'a, 's, C, RawBodyRow, 2> {
        RawBodyRowQuery {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<RawBodyRowBorrowed, tokio_postgres::Error> {
                    Ok(RawBodyRowBorrowed {
                        headers_json: row.try_get(0)?,
                        body_values_json: row.try_get(1)?,
                        attachments_json: row.try_get(2)?,
                    })
                },
            mapper: |it| RawBodyRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        RawMessageProjectionParams<T1, T2>,
        RawBodyRowQuery<'c, 'a, 's, C, RawBodyRow, 2>,
        C,
    > for RawMessageProjectionStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a RawMessageProjectionParams<T1, T2>,
    ) -> RawBodyRowQuery<'c, 'a, 's, C, RawBodyRow, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct RawMessageBytesStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn raw_message_bytes() -> RawMessageBytesStmt {
    RawMessageBytesStmt(
        "SELECT raw_rfc822 FROM raw_messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl RawMessageBytesStmt {
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
        msgid: &'a T2,
    ) -> RawBytesRowQuery<'c, 'a, 's, C, RawBytesRow, 2> {
        RawBytesRowQuery {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<RawBytesRowBorrowed, tokio_postgres::Error> {
                    Ok(RawBytesRowBorrowed {
                        raw_rfc822: row.try_get(0)?,
                    })
                },
            mapper: |it| RawBytesRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        RawMessageBytesParams<T1, T2>,
        RawBytesRowQuery<'c, 'a, 's, C, RawBytesRow, 2>,
        C,
    > for RawMessageBytesStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a RawMessageBytesParams<T1, T2>,
    ) -> RawBytesRowQuery<'c, 'a, 's, C, RawBytesRow, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct CachedMessageIdsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn cached_message_ids() -> CachedMessageIdsStmt {
    CachedMessageIdsStmt(
        "SELECT msgid FROM raw_messages WHERE account_id = $1 AND msgid = ANY($2)",
        None,
    )
}
impl CachedMessageIdsStmt {
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
        T3: crate::ArraySql<Item = T2>,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        msgids: &'a T3,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        StringQuery {
            client,
            params: [account_id, msgids],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it.into(),
        }
    }
}
impl<
    'c,
    'a,
    's,
    C: GenericClient,
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::ArraySql<Item = T2>,
>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        CachedMessageIdsParams<T1, T2, T3>,
        StringQuery<'c, 'a, 's, C, String, 2>,
        C,
    > for CachedMessageIdsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a CachedMessageIdsParams<T1, T2, T3>,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        self.bind(client, &params.account_id, &params.msgids)
    }
}
pub struct RecentUncachedMessageIdsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn recent_uncached_message_ids() -> RecentUncachedMessageIdsStmt {
    RecentUncachedMessageIdsStmt(
        "WITH recent AS ( SELECT account_id, msgid, received_at, size FROM messages WHERE account_id = $1 ORDER BY received_at DESC LIMIT $2 ) SELECT m.msgid FROM recent m LEFT JOIN raw_messages r ON r.account_id = m.account_id AND r.msgid = m.msgid WHERE r.msgid IS NULL AND m.size <= $3 ORDER BY m.received_at DESC LIMIT $4",
        None,
    )
}
impl RecentUncachedMessageIdsStmt {
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
        window: &'a i64,
        max_bytes: &'a i64,
        limit: &'a i64,
    ) -> StringQuery<'c, 'a, 's, C, String, 4> {
        StringQuery {
            client,
            params: [account_id, window, max_bytes, limit],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it.into(),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        RecentUncachedMessageIdsParams<T1>,
        StringQuery<'c, 'a, 's, C, String, 4>,
        C,
    > for RecentUncachedMessageIdsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a RecentUncachedMessageIdsParams<T1>,
    ) -> StringQuery<'c, 'a, 's, C, String, 4> {
        self.bind(
            client,
            &params.account_id,
            &params.window,
            &params.max_bytes,
            &params.limit,
        )
    }
}
pub struct UpsertRawMessageStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_raw_message() -> UpsertRawMessageStmt {
    UpsertRawMessageStmt(
        "INSERT INTO raw_messages (account_id, msgid, headers_json, body_values_json, attachments_json, raw_rfc822, fetched_at) VALUES ($1, $2, $3, $4, $5, $6, EXTRACT(EPOCH FROM now())::bigint) ON CONFLICT (account_id, msgid) DO UPDATE SET headers_json = EXCLUDED.headers_json, body_values_json = EXCLUDED.body_values_json, attachments_json = EXCLUDED.attachments_json, raw_rfc822 = EXCLUDED.raw_rfc822, fetched_at = EXCLUDED.fetched_at",
        None,
    )
}
impl UpsertRawMessageStmt {
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
        T6: crate::BytesSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        msgid: &'a T2,
        headers_json: &'a T3,
        body_values_json: &'a T4,
        attachments_json: &'a T5,
        raw_rfc822: &'a T6,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id,
                    msgid,
                    headers_json,
                    body_values_json,
                    attachments_json,
                    raw_rfc822,
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
    T6: crate::BytesSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertRawMessageParams<T1, T2, T3, T4, T5, T6>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertRawMessageStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertRawMessageParams<T1, T2, T3, T4, T5, T6>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.msgid,
            &params.headers_json,
            &params.body_values_json,
            &params.attachments_json,
            &params.raw_rfc822,
        ))
    }
}
pub struct CachedMetadataRepairCandidatesStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn cached_metadata_repair_candidates() -> CachedMetadataRepairCandidatesStmt {
    CachedMetadataRepairCandidatesStmt(
        "SELECT r.msgid, m.subject, r.raw_rfc822 FROM raw_messages r JOIN messages m ON m.account_id = r.account_id AND m.msgid = r.msgid WHERE r.account_id = $1 AND ( m.preview IS NULL OR m.has_attachment <> CASE WHEN r.attachments_json = '[]' THEN 0 ELSE 1 END OR m.subject LIKE '%=?%?=%' )",
        None,
    )
}
impl CachedMetadataRepairCandidatesStmt {
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
    ) -> CachedMetadataRepairRowQuery<'c, 'a, 's, C, CachedMetadataRepairRow, 1> {
        CachedMetadataRepairRowQuery {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<CachedMetadataRepairRowBorrowed, tokio_postgres::Error> {
                Ok(CachedMetadataRepairRowBorrowed {
                    msgid: row.try_get(0)?,
                    subject: row.try_get(1)?,
                    raw_rfc822: row.try_get(2)?,
                })
            },
            mapper: |it| CachedMetadataRepairRow::from(it),
        }
    }
}
