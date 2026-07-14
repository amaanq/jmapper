// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct UpsertUploadedBlobParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::BytesSql,
> {
    pub account_id: T1,
    pub blob_id: T2,
    pub content_type: T3,
    pub bytes: T4,
    pub uploaded_at: i64,
    pub expires_at: i64,
}
#[derive(Debug)]
pub struct GetUploadedBlobParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub blob_id: T2,
}
#[derive(Debug, Clone, PartialEq)]
pub struct UploadedBlobRow {
    pub bytes: Vec<u8>,
    pub content_type: String,
}
pub struct UploadedBlobRowBorrowed<'a> {
    pub bytes: &'a [u8],
    pub content_type: &'a str,
}
impl<'a> From<UploadedBlobRowBorrowed<'a>> for UploadedBlobRow {
    fn from(
        UploadedBlobRowBorrowed {
            bytes,
            content_type,
        }: UploadedBlobRowBorrowed<'a>,
    ) -> Self {
        Self {
            bytes: bytes.into(),
            content_type: content_type.into(),
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct UploadedBlobRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<UploadedBlobRowBorrowed, tokio_postgres::Error>,
    mapper: fn(UploadedBlobRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> UploadedBlobRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(UploadedBlobRowBorrowed) -> R,
    ) -> UploadedBlobRowQuery<'c, 'a, 's, C, R, N> {
        UploadedBlobRowQuery {
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
pub struct UpsertUploadedBlobStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_uploaded_blob() -> UpsertUploadedBlobStmt {
    UpsertUploadedBlobStmt(
        "INSERT INTO uploaded_blobs (account_id, blob_id, content_type, bytes, uploaded_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (account_id, blob_id) DO UPDATE SET content_type = EXCLUDED.content_type, bytes = EXCLUDED.bytes, uploaded_at = EXCLUDED.uploaded_at, expires_at = EXCLUDED.expires_at",
        None,
    )
}
impl UpsertUploadedBlobStmt {
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
        T4: crate::BytesSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        blob_id: &'a T2,
        content_type: &'a T3,
        bytes: &'a T4,
        uploaded_at: &'a i64,
        expires_at: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id,
                    blob_id,
                    content_type,
                    bytes,
                    uploaded_at,
                    expires_at,
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
    T4: crate::BytesSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertUploadedBlobParams<T1, T2, T3, T4>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertUploadedBlobStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertUploadedBlobParams<T1, T2, T3, T4>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.blob_id,
            &params.content_type,
            &params.bytes,
            &params.uploaded_at,
            &params.expires_at,
        ))
    }
}
pub struct GetUploadedBlobStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_uploaded_blob() -> GetUploadedBlobStmt {
    GetUploadedBlobStmt(
        "SELECT bytes, content_type FROM uploaded_blobs WHERE account_id = $1 AND blob_id = $2 AND expires_at > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT",
        None,
    )
}
impl GetUploadedBlobStmt {
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
        blob_id: &'a T2,
    ) -> UploadedBlobRowQuery<'c, 'a, 's, C, UploadedBlobRow, 2> {
        UploadedBlobRowQuery {
            client,
            params: [account_id, blob_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<UploadedBlobRowBorrowed, tokio_postgres::Error> {
                Ok(UploadedBlobRowBorrowed {
                    bytes: row.try_get(0)?,
                    content_type: row.try_get(1)?,
                })
            },
            mapper: |it| UploadedBlobRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetUploadedBlobParams<T1, T2>,
        UploadedBlobRowQuery<'c, 'a, 's, C, UploadedBlobRow, 2>,
        C,
    > for GetUploadedBlobStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetUploadedBlobParams<T1, T2>,
    ) -> UploadedBlobRowQuery<'c, 'a, 's, C, UploadedBlobRow, 2> {
        self.bind(client, &params.account_id, &params.blob_id)
    }
}
