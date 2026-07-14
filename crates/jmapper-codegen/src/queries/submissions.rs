// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct InsertSubmissionParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
    T7: crate::StringSql,
    T8: crate::BytesSql,
    T9: crate::StringSql,
> {
    pub account_id: T1,
    pub id: T2,
    pub email_id: T3,
    pub identity_id: T4,
    pub thread_id: Option<T5>,
    pub envelope_json: T6,
    pub send_at: i64,
    pub undo_status: T7,
    pub raw_rfc822: Option<T8>,
    pub delivery_status_json: Option<T9>,
    pub modseq: i64,
}
#[derive(Debug)]
pub struct GetSubmissionsByIdsParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::ArraySql<Item = T2>,
> {
    pub account_id: T1,
    pub ids: T3,
}
#[derive(Debug)]
pub struct SubmissionsChangedSinceParams<T1: crate::StringSql> {
    pub account_id: T1,
    pub since: i64,
}
#[derive(Debug)]
pub struct SubmissionUndoStatusParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct CancelPendingSubmissionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub modseq: i64,
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct TombstoneSubmissionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub modseq: i64,
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct ClaimSubmissionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug)]
pub struct FinishSubmissionParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
> {
    pub undo_status: T1,
    pub delivery_status_json: Option<T2>,
    pub modseq: i64,
    pub account_id: T3,
    pub id: T4,
}
#[derive(Debug)]
pub struct RetrySubmissionParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub id: T2,
}
#[derive(Debug, Clone, PartialEq)]
pub struct SubmissionRow {
    pub id: String,
    pub email_id: String,
    pub identity_id: String,
    pub thread_id: Option<String>,
    pub envelope_json: String,
    pub send_at: i64,
    pub undo_status: String,
    pub delivery_status_json: Option<String>,
}
pub struct SubmissionRowBorrowed<'a> {
    pub id: &'a str,
    pub email_id: &'a str,
    pub identity_id: &'a str,
    pub thread_id: Option<&'a str>,
    pub envelope_json: &'a str,
    pub send_at: i64,
    pub undo_status: &'a str,
    pub delivery_status_json: Option<&'a str>,
}
impl<'a> From<SubmissionRowBorrowed<'a>> for SubmissionRow {
    fn from(
        SubmissionRowBorrowed {
            id,
            email_id,
            identity_id,
            thread_id,
            envelope_json,
            send_at,
            undo_status,
            delivery_status_json,
        }: SubmissionRowBorrowed<'a>,
    ) -> Self {
        Self {
            id: id.into(),
            email_id: email_id.into(),
            identity_id: identity_id.into(),
            thread_id: thread_id.map(|v| v.into()),
            envelope_json: envelope_json.into(),
            send_at,
            undo_status: undo_status.into(),
            delivery_status_json: delivery_status_json.map(|v| v.into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct SubmissionChangeRow {
    pub id: String,
    pub destroyed: i64,
}
pub struct SubmissionChangeRowBorrowed<'a> {
    pub id: &'a str,
    pub destroyed: i64,
}
impl<'a> From<SubmissionChangeRowBorrowed<'a>> for SubmissionChangeRow {
    fn from(
        SubmissionChangeRowBorrowed { id, destroyed }: SubmissionChangeRowBorrowed<'a>,
    ) -> Self {
        Self {
            id: id.into(),
            destroyed,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DueSubmissionRow {
    pub account_id: String,
    pub id: String,
    pub envelope_json: String,
    pub raw_rfc822: Option<Vec<u8>>,
    pub attempts: i64,
}
pub struct DueSubmissionRowBorrowed<'a> {
    pub account_id: &'a str,
    pub id: &'a str,
    pub envelope_json: &'a str,
    pub raw_rfc822: Option<&'a [u8]>,
    pub attempts: i64,
}
impl<'a> From<DueSubmissionRowBorrowed<'a>> for DueSubmissionRow {
    fn from(
        DueSubmissionRowBorrowed {
            account_id,
            id,
            envelope_json,
            raw_rfc822,
            attempts,
        }: DueSubmissionRowBorrowed<'a>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            id: id.into(),
            envelope_json: envelope_json.into(),
            raw_rfc822: raw_rfc822.map(|v| v.into()),
            attempts,
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct SubmissionRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<SubmissionRowBorrowed, tokio_postgres::Error>,
    mapper: fn(SubmissionRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> SubmissionRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(SubmissionRowBorrowed) -> R,
    ) -> SubmissionRowQuery<'c, 'a, 's, C, R, N> {
        SubmissionRowQuery {
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
pub struct SubmissionChangeRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<SubmissionChangeRowBorrowed, tokio_postgres::Error>,
    mapper: fn(SubmissionChangeRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> SubmissionChangeRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(SubmissionChangeRowBorrowed) -> R,
    ) -> SubmissionChangeRowQuery<'c, 'a, 's, C, R, N> {
        SubmissionChangeRowQuery {
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
pub struct DueSubmissionRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<DueSubmissionRowBorrowed, tokio_postgres::Error>,
    mapper: fn(DueSubmissionRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> DueSubmissionRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(DueSubmissionRowBorrowed) -> R,
    ) -> DueSubmissionRowQuery<'c, 'a, 's, C, R, N> {
        DueSubmissionRowQuery {
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
pub struct InsertSubmissionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn insert_submission() -> InsertSubmissionStmt {
    InsertSubmissionStmt(
        "INSERT INTO email_submissions (account_id, id, email_id, identity_id, thread_id, envelope_json, send_at, undo_status, raw_rfc822, delivery_status_json, modseq) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        None,
    )
}
impl InsertSubmissionStmt {
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
        T8: crate::BytesSql,
        T9: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        id: &'a T2,
        email_id: &'a T3,
        identity_id: &'a T4,
        thread_id: &'a Option<T5>,
        envelope_json: &'a T6,
        send_at: &'a i64,
        undo_status: &'a T7,
        raw_rfc822: &'a Option<T8>,
        delivery_status_json: &'a Option<T9>,
        modseq: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id,
                    id,
                    email_id,
                    identity_id,
                    thread_id,
                    envelope_json,
                    send_at,
                    undo_status,
                    raw_rfc822,
                    delivery_status_json,
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
    T8: crate::BytesSql,
    T9: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        InsertSubmissionParams<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for InsertSubmissionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a InsertSubmissionParams<T1, T2, T3, T4, T5, T6, T7, T8, T9>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.id,
            &params.email_id,
            &params.identity_id,
            &params.thread_id,
            &params.envelope_json,
            &params.send_at,
            &params.undo_status,
            &params.raw_rfc822,
            &params.delivery_status_json,
            &params.modseq,
        ))
    }
}
pub struct GetSubmissionsByIdsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_submissions_by_ids() -> GetSubmissionsByIdsStmt {
    GetSubmissionsByIdsStmt(
        "SELECT id, email_id, identity_id, thread_id, envelope_json, send_at, undo_status, delivery_status_json FROM email_submissions WHERE account_id = $1 AND destroyed = 0 AND id = ANY($2)",
        None,
    )
}
impl GetSubmissionsByIdsStmt {
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
        ids: &'a T3,
    ) -> SubmissionRowQuery<'c, 'a, 's, C, SubmissionRow, 2> {
        SubmissionRowQuery {
            client,
            params: [account_id, ids],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<SubmissionRowBorrowed, tokio_postgres::Error> {
                    Ok(SubmissionRowBorrowed {
                        id: row.try_get(0)?,
                        email_id: row.try_get(1)?,
                        identity_id: row.try_get(2)?,
                        thread_id: row.try_get(3)?,
                        envelope_json: row.try_get(4)?,
                        send_at: row.try_get(5)?,
                        undo_status: row.try_get(6)?,
                        delivery_status_json: row.try_get(7)?,
                    })
                },
            mapper: |it| SubmissionRow::from(it),
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
        GetSubmissionsByIdsParams<T1, T2, T3>,
        SubmissionRowQuery<'c, 'a, 's, C, SubmissionRow, 2>,
        C,
    > for GetSubmissionsByIdsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetSubmissionsByIdsParams<T1, T2, T3>,
    ) -> SubmissionRowQuery<'c, 'a, 's, C, SubmissionRow, 2> {
        self.bind(client, &params.account_id, &params.ids)
    }
}
pub struct ListSubmissionsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_submissions() -> ListSubmissionsStmt {
    ListSubmissionsStmt(
        "SELECT id, email_id, identity_id, thread_id, envelope_json, send_at, undo_status, delivery_status_json FROM email_submissions WHERE account_id = $1 AND destroyed = 0 ORDER BY send_at DESC LIMIT 500",
        None,
    )
}
impl ListSubmissionsStmt {
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
    ) -> SubmissionRowQuery<'c, 'a, 's, C, SubmissionRow, 1> {
        SubmissionRowQuery {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<SubmissionRowBorrowed, tokio_postgres::Error> {
                    Ok(SubmissionRowBorrowed {
                        id: row.try_get(0)?,
                        email_id: row.try_get(1)?,
                        identity_id: row.try_get(2)?,
                        thread_id: row.try_get(3)?,
                        envelope_json: row.try_get(4)?,
                        send_at: row.try_get(5)?,
                        undo_status: row.try_get(6)?,
                        delivery_status_json: row.try_get(7)?,
                    })
                },
            mapper: |it| SubmissionRow::from(it),
        }
    }
}
pub struct SubmissionsChangedSinceStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn submissions_changed_since() -> SubmissionsChangedSinceStmt {
    SubmissionsChangedSinceStmt(
        "SELECT id, destroyed FROM email_submissions WHERE account_id = $1 AND modseq > $2 ORDER BY modseq LIMIT 500",
        None,
    )
}
impl SubmissionsChangedSinceStmt {
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
        since: &'a i64,
    ) -> SubmissionChangeRowQuery<'c, 'a, 's, C, SubmissionChangeRow, 2> {
        SubmissionChangeRowQuery {
            client,
            params: [account_id, since],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<SubmissionChangeRowBorrowed, tokio_postgres::Error> {
                Ok(SubmissionChangeRowBorrowed {
                    id: row.try_get(0)?,
                    destroyed: row.try_get(1)?,
                })
            },
            mapper: |it| SubmissionChangeRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        SubmissionsChangedSinceParams<T1>,
        SubmissionChangeRowQuery<'c, 'a, 's, C, SubmissionChangeRow, 2>,
        C,
    > for SubmissionsChangedSinceStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a SubmissionsChangedSinceParams<T1>,
    ) -> SubmissionChangeRowQuery<'c, 'a, 's, C, SubmissionChangeRow, 2> {
        self.bind(client, &params.account_id, &params.since)
    }
}
pub struct SubmissionUndoStatusStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn submission_undo_status() -> SubmissionUndoStatusStmt {
    SubmissionUndoStatusStmt(
        "SELECT undo_status FROM email_submissions WHERE account_id = $1 AND id = $2 AND destroyed = 0",
        None,
    )
}
impl SubmissionUndoStatusStmt {
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
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        StringQuery {
            client,
            params: [account_id, id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it.into(),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        SubmissionUndoStatusParams<T1, T2>,
        StringQuery<'c, 'a, 's, C, String, 2>,
        C,
    > for SubmissionUndoStatusStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a SubmissionUndoStatusParams<T1, T2>,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        self.bind(client, &params.account_id, &params.id)
    }
}
pub struct CancelPendingSubmissionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn cancel_pending_submission() -> CancelPendingSubmissionStmt {
    CancelPendingSubmissionStmt(
        "UPDATE email_submissions SET undo_status = 'canceled', modseq = $1 WHERE account_id = $2 AND id = $3 AND destroyed = 0 AND undo_status = 'pending'",
        None,
    )
}
impl CancelPendingSubmissionStmt {
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
        CancelPendingSubmissionParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for CancelPendingSubmissionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a CancelPendingSubmissionParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.modseq, &params.account_id, &params.id))
    }
}
pub struct TombstoneSubmissionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn tombstone_submission() -> TombstoneSubmissionStmt {
    TombstoneSubmissionStmt(
        "UPDATE email_submissions SET destroyed = 1, raw_rfc822 = NULL, modseq = $1 WHERE account_id = $2 AND id = $3 AND destroyed = 0 AND undo_status IN ('final', 'canceled')",
        None,
    )
}
impl TombstoneSubmissionStmt {
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
        TombstoneSubmissionParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for TombstoneSubmissionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a TombstoneSubmissionParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.modseq, &params.account_id, &params.id))
    }
}
pub struct RecoverStrandedSubmissionsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn recover_stranded_submissions() -> RecoverStrandedSubmissionsStmt {
    RecoverStrandedSubmissionsStmt(
        "UPDATE email_submissions SET undo_status = 'pending' WHERE undo_status = 'sending'",
        None,
    )
}
impl RecoverStrandedSubmissionsStmt {
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
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[]).await
    }
}
pub struct DueSubmissionsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn due_submissions() -> DueSubmissionsStmt {
    DueSubmissionsStmt(
        "SELECT account_id, id, envelope_json, raw_rfc822, attempts FROM email_submissions WHERE undo_status = 'pending' AND destroyed = 0 AND send_at <= $1 LIMIT 50",
        None,
    )
}
impl DueSubmissionsStmt {
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
        now: &'a i64,
    ) -> DueSubmissionRowQuery<'c, 'a, 's, C, DueSubmissionRow, 1> {
        DueSubmissionRowQuery {
            client,
            params: [now],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<DueSubmissionRowBorrowed, tokio_postgres::Error> {
                Ok(DueSubmissionRowBorrowed {
                    account_id: row.try_get(0)?,
                    id: row.try_get(1)?,
                    envelope_json: row.try_get(2)?,
                    raw_rfc822: row.try_get(3)?,
                    attempts: row.try_get(4)?,
                })
            },
            mapper: |it| DueSubmissionRow::from(it),
        }
    }
}
pub struct ClaimSubmissionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn claim_submission() -> ClaimSubmissionStmt {
    ClaimSubmissionStmt(
        "UPDATE email_submissions SET undo_status = 'sending' WHERE account_id = $1 AND id = $2 AND undo_status = 'pending'",
        None,
    )
}
impl ClaimSubmissionStmt {
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
        id: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        ClaimSubmissionParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for ClaimSubmissionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a ClaimSubmissionParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.account_id, &params.id))
    }
}
pub struct FinishSubmissionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn finish_submission() -> FinishSubmissionStmt {
    FinishSubmissionStmt(
        "UPDATE email_submissions SET undo_status = $1, raw_rfc822 = NULL, delivery_status_json = COALESCE($2, delivery_status_json), modseq = $3 WHERE account_id = $4 AND id = $5",
        None,
    )
}
impl FinishSubmissionStmt {
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
        undo_status: &'a T1,
        delivery_status_json: &'a Option<T2>,
        modseq: &'a i64,
        account_id: &'a T3,
        id: &'a T4,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[undo_status, delivery_status_json, modseq, account_id, id],
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
        FinishSubmissionParams<T1, T2, T3, T4>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for FinishSubmissionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a FinishSubmissionParams<T1, T2, T3, T4>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.undo_status,
            &params.delivery_status_json,
            &params.modseq,
            &params.account_id,
            &params.id,
        ))
    }
}
pub struct RetrySubmissionStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn retry_submission() -> RetrySubmissionStmt {
    RetrySubmissionStmt(
        "UPDATE email_submissions SET undo_status = 'pending', attempts = attempts + 1 WHERE account_id = $1 AND id = $2 AND undo_status = 'sending'",
        None,
    )
}
impl RetrySubmissionStmt {
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
        id: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        RetrySubmissionParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for RetrySubmissionStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a RetrySubmissionParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.account_id, &params.id))
    }
}
