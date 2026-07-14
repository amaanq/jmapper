// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct BindMessageIdParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql> {
    pub account_id: T1,
    pub message_id: T2,
    pub thrid: T3,
}
#[derive(Debug)]
pub struct ThridFromRefsParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::ArraySql<Item = T2>,
> {
    pub account_id: T1,
    pub refs: T3,
}
#[derive(Debug)]
pub struct RecordSubjectParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql> {
    pub account_id: T1,
    pub norm_subject: T2,
    pub thrid: T3,
    pub last_received_at: i64,
}
#[derive(Debug)]
pub struct ThridFromSubjectParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub norm_subject: T2,
    pub cutoff: i64,
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
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
pub struct BindMessageIdStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bind_message_id() -> BindMessageIdStmt {
    BindMessageIdStmt(
        "INSERT INTO thread_index (account_id, message_id, thrid) VALUES ($1, $2, $3) ON CONFLICT (account_id, message_id) DO NOTHING",
        None,
    )
}
impl BindMessageIdStmt {
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
        account_id: &'a T1,
        message_id: &'a T2,
        thrid: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[account_id, message_id, thrid])
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
        BindMessageIdParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for BindMessageIdStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a BindMessageIdParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.message_id,
            &params.thrid,
        ))
    }
}
pub struct ThridFromRefsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn thrid_from_refs() -> ThridFromRefsStmt {
    ThridFromRefsStmt(
        "SELECT thrid FROM thread_index WHERE account_id = $1 AND message_id = ANY($2) LIMIT 1",
        None,
    )
}
impl ThridFromRefsStmt {
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
        refs: &'a T3,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        StringQuery {
            client,
            params: [account_id, refs],
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
        ThridFromRefsParams<T1, T2, T3>,
        StringQuery<'c, 'a, 's, C, String, 2>,
        C,
    > for ThridFromRefsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ThridFromRefsParams<T1, T2, T3>,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        self.bind(client, &params.account_id, &params.refs)
    }
}
pub struct RecordSubjectStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn record_subject() -> RecordSubjectStmt {
    RecordSubjectStmt(
        "INSERT INTO thread_by_subject (account_id, norm_subject, thrid, last_received_at) VALUES ($1, $2, $3, $4) ON CONFLICT (account_id, norm_subject) DO UPDATE SET thrid = EXCLUDED.thrid, last_received_at = GREATEST(thread_by_subject.last_received_at, EXCLUDED.last_received_at)",
        None,
    )
}
impl RecordSubjectStmt {
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
        account_id: &'a T1,
        norm_subject: &'a T2,
        thrid: &'a T3,
        last_received_at: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[account_id, norm_subject, thrid, last_received_at])
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
        RecordSubjectParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for RecordSubjectStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a RecordSubjectParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.norm_subject,
            &params.thrid,
            &params.last_received_at,
        ))
    }
}
pub struct ThridFromSubjectStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn thrid_from_subject() -> ThridFromSubjectStmt {
    ThridFromSubjectStmt(
        "SELECT thrid FROM thread_by_subject WHERE account_id = $1 AND norm_subject = $2 AND last_received_at >= $3",
        None,
    )
}
impl ThridFromSubjectStmt {
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
        norm_subject: &'a T2,
        cutoff: &'a i64,
    ) -> StringQuery<'c, 'a, 's, C, String, 3> {
        StringQuery {
            client,
            params: [account_id, norm_subject, cutoff],
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
        ThridFromSubjectParams<T1, T2>,
        StringQuery<'c, 'a, 's, C, String, 3>,
        C,
    > for ThridFromSubjectStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ThridFromSubjectParams<T1, T2>,
    ) -> StringQuery<'c, 'a, 's, C, String, 3> {
        self.bind(
            client,
            &params.account_id,
            &params.norm_subject,
            &params.cutoff,
        )
    }
}
