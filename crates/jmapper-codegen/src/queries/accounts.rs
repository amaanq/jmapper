// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct UpsertAccountParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::BytesSql,
> {
    pub id: T1,
    pub email: T2,
    pub provider: T3,
    pub display_name: T4,
    pub bearer_token_hash: T5,
}
#[derive(Debug, Clone, PartialEq)]
pub struct AccountRow {
    pub id: String,
    pub email: String,
    pub provider: String,
    pub display_name: String,
    pub bearer_token_hash: Vec<u8>,
    pub created_at: i64,
}
pub struct AccountRowBorrowed<'a> {
    pub id: &'a str,
    pub email: &'a str,
    pub provider: &'a str,
    pub display_name: &'a str,
    pub bearer_token_hash: &'a [u8],
    pub created_at: i64,
}
impl<'a> From<AccountRowBorrowed<'a>> for AccountRow {
    fn from(
        AccountRowBorrowed {
            id,
            email,
            provider,
            display_name,
            bearer_token_hash,
            created_at,
        }: AccountRowBorrowed<'a>,
    ) -> Self {
        Self {
            id: id.into(),
            email: email.into(),
            provider: provider.into(),
            display_name: display_name.into(),
            bearer_token_hash: bearer_token_hash.into(),
            created_at,
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct AccountRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<AccountRowBorrowed, tokio_postgres::Error>,
    mapper: fn(AccountRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> AccountRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(AccountRowBorrowed) -> R,
    ) -> AccountRowQuery<'c, 'a, 's, C, R, N> {
        AccountRowQuery {
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
pub struct UpsertAccountStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_account() -> UpsertAccountStmt {
    UpsertAccountStmt(
        "INSERT INTO accounts (id, email, provider, display_name, bearer_token_hash, created_at) VALUES ($1, $2, $3, $4, $5, EXTRACT(EPOCH FROM now())::bigint) ON CONFLICT (id) DO UPDATE SET email = EXCLUDED.email, provider = EXCLUDED.provider, display_name = EXCLUDED.display_name, bearer_token_hash = EXCLUDED.bearer_token_hash",
        None,
    )
}
impl UpsertAccountStmt {
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
        T5: crate::BytesSql,
    >(
        &'s self,
        client: &'c C,
        id: &'a T1,
        email: &'a T2,
        provider: &'a T3,
        display_name: &'a T4,
        bearer_token_hash: &'a T5,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[id, email, provider, display_name, bearer_token_hash],
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
    T5: crate::BytesSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertAccountParams<T1, T2, T3, T4, T5>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertAccountStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertAccountParams<T1, T2, T3, T4, T5>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.id,
            &params.email,
            &params.provider,
            &params.display_name,
            &params.bearer_token_hash,
        ))
    }
}
pub struct EnsureStateRowStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn ensure_state_row() -> EnsureStateRowStmt {
    EnsureStateRowStmt(
        "INSERT INTO state (account_id) VALUES ($1) ON CONFLICT (account_id) DO NOTHING",
        None,
    )
}
impl EnsureStateRowStmt {
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
pub struct GetAccountStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_account() -> GetAccountStmt {
    GetAccountStmt(
        "SELECT id, email, provider, display_name, bearer_token_hash, created_at FROM accounts WHERE id = $1",
        None,
    )
}
impl GetAccountStmt {
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
        id: &'a T1,
    ) -> AccountRowQuery<'c, 'a, 's, C, AccountRow, 1> {
        AccountRowQuery {
            client,
            params: [id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<AccountRowBorrowed, tokio_postgres::Error> {
                    Ok(AccountRowBorrowed {
                        id: row.try_get(0)?,
                        email: row.try_get(1)?,
                        provider: row.try_get(2)?,
                        display_name: row.try_get(3)?,
                        bearer_token_hash: row.try_get(4)?,
                        created_at: row.try_get(5)?,
                    })
                },
            mapper: |it| AccountRow::from(it),
        }
    }
}
pub struct ListAccountsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_accounts() -> ListAccountsStmt {
    ListAccountsStmt(
        "SELECT id, email, provider, display_name, bearer_token_hash, created_at FROM accounts ORDER BY id",
        None,
    )
}
impl ListAccountsStmt {
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
    ) -> AccountRowQuery<'c, 'a, 's, C, AccountRow, 0> {
        AccountRowQuery {
            client,
            params: [],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<AccountRowBorrowed, tokio_postgres::Error> {
                    Ok(AccountRowBorrowed {
                        id: row.try_get(0)?,
                        email: row.try_get(1)?,
                        provider: row.try_get(2)?,
                        display_name: row.try_get(3)?,
                        bearer_token_hash: row.try_get(4)?,
                        created_at: row.try_get(5)?,
                    })
                },
            mapper: |it| AccountRow::from(it),
        }
    }
}
