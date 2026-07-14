// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct UpsertOauthParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql> {
    pub account_id: T1,
    pub access_token: Option<T2>,
    pub refresh_token: T3,
    pub expires_at: Option<i64>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct OAuthTokenRow {
    pub account_id: String,
    pub access_token: Option<String>,
    pub refresh_token: String,
    pub expires_at: Option<i64>,
}
pub struct OAuthTokenRowBorrowed<'a> {
    pub account_id: &'a str,
    pub access_token: Option<&'a str>,
    pub refresh_token: &'a str,
    pub expires_at: Option<i64>,
}
impl<'a> From<OAuthTokenRowBorrowed<'a>> for OAuthTokenRow {
    fn from(
        OAuthTokenRowBorrowed {
            account_id,
            access_token,
            refresh_token,
            expires_at,
        }: OAuthTokenRowBorrowed<'a>,
    ) -> Self {
        Self {
            account_id: account_id.into(),
            access_token: access_token.map(|v| v.into()),
            refresh_token: refresh_token.into(),
            expires_at,
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct OAuthTokenRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<OAuthTokenRowBorrowed, tokio_postgres::Error>,
    mapper: fn(OAuthTokenRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> OAuthTokenRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(OAuthTokenRowBorrowed) -> R,
    ) -> OAuthTokenRowQuery<'c, 'a, 's, C, R, N> {
        OAuthTokenRowQuery {
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
pub struct UpsertOauthStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_oauth() -> UpsertOauthStmt {
    UpsertOauthStmt(
        "INSERT INTO oauth_tokens (account_id, access_token, refresh_token, expires_at) VALUES ($1, $2, $3, $4) ON CONFLICT (account_id) DO UPDATE SET access_token = EXCLUDED.access_token, refresh_token = EXCLUDED.refresh_token, expires_at = EXCLUDED.expires_at",
        None,
    )
}
impl UpsertOauthStmt {
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
        access_token: &'a Option<T2>,
        refresh_token: &'a T3,
        expires_at: &'a Option<i64>,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[account_id, access_token, refresh_token, expires_at],
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
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertOauthParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertOauthStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertOauthParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.access_token,
            &params.refresh_token,
            &params.expires_at,
        ))
    }
}
pub struct GetOauthStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_oauth() -> GetOauthStmt {
    GetOauthStmt(
        "SELECT account_id, access_token, refresh_token, expires_at FROM oauth_tokens WHERE account_id = $1",
        None,
    )
}
impl GetOauthStmt {
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
    ) -> OAuthTokenRowQuery<'c, 'a, 's, C, OAuthTokenRow, 1> {
        OAuthTokenRowQuery {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<OAuthTokenRowBorrowed, tokio_postgres::Error> {
                    Ok(OAuthTokenRowBorrowed {
                        account_id: row.try_get(0)?,
                        access_token: row.try_get(1)?,
                        refresh_token: row.try_get(2)?,
                        expires_at: row.try_get(3)?,
                    })
                },
            mapper: |it| OAuthTokenRow::from(it),
        }
    }
}
