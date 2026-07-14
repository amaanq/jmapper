// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct UpsertFolderParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
> {
    pub account_id: T1,
    pub imap_name: T2,
    pub uidvalidity: i64,
    pub uidnext: i64,
    pub highestmodseq: i64,
    pub role: Option<T3>,
    pub mailbox_id: T4,
}
#[derive(Debug)]
pub struct FolderSyncStateParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub imap_name: T2,
}
#[derive(Debug)]
pub struct FolderByNameParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub imap_name: T2,
}
#[derive(Clone, Copy, Debug)]
pub struct SetFolderUidfirstParams {
    pub uidfirst: i64,
    pub id: i64,
}
#[derive(Clone, Copy, Debug)]
pub struct SetFolderUidnextParams {
    pub uidnext: i64,
    pub id: i64,
}
#[derive(Debug)]
pub struct RenameFolderParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub imap_name: T1,
    pub id: i64,
    pub account_id: T2,
}
#[derive(Debug)]
pub struct DeleteFolderParams<T1: crate::StringSql> {
    pub id: i64,
    pub account_id: T1,
}
#[derive(Debug)]
pub struct ResetFolderSyncStateParams<T1: crate::StringSql> {
    pub id: i64,
    pub account_id: T1,
}
#[derive(Debug)]
pub struct FolderChildrenParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub prefix: T2,
}
#[derive(Debug, Clone, PartialEq)]
pub struct FolderRow {
    pub id: i64,
    pub account_id: String,
    pub imap_name: String,
    pub uidvalidity: i64,
    pub uidnext: i64,
    pub uidfirst: i64,
    pub highestmodseq: i64,
    pub role: Option<String>,
    pub last_sync_at: Option<i64>,
    pub mailbox_id: String,
}
pub struct FolderRowBorrowed<'a> {
    pub id: i64,
    pub account_id: &'a str,
    pub imap_name: &'a str,
    pub uidvalidity: i64,
    pub uidnext: i64,
    pub uidfirst: i64,
    pub highestmodseq: i64,
    pub role: Option<&'a str>,
    pub last_sync_at: Option<i64>,
    pub mailbox_id: &'a str,
}
impl<'a> From<FolderRowBorrowed<'a>> for FolderRow {
    fn from(
        FolderRowBorrowed {
            id,
            account_id,
            imap_name,
            uidvalidity,
            uidnext,
            uidfirst,
            highestmodseq,
            role,
            last_sync_at,
            mailbox_id,
        }: FolderRowBorrowed<'a>,
    ) -> Self {
        Self {
            id,
            account_id: account_id.into(),
            imap_name: imap_name.into(),
            uidvalidity,
            uidnext,
            uidfirst,
            highestmodseq,
            role: role.map(|v| v.into()),
            last_sync_at,
            mailbox_id: mailbox_id.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FolderSyncStateRow {
    pub uidfirst: i64,
    pub mailbox_id: String,
}
pub struct FolderSyncStateRowBorrowed<'a> {
    pub uidfirst: i64,
    pub mailbox_id: &'a str,
}
impl<'a> From<FolderSyncStateRowBorrowed<'a>> for FolderSyncStateRow {
    fn from(
        FolderSyncStateRowBorrowed {
            uidfirst,
            mailbox_id,
        }: FolderSyncStateRowBorrowed<'a>,
    ) -> Self {
        Self {
            uidfirst,
            mailbox_id: mailbox_id.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FolderByName {
    pub id: i64,
    pub uidvalidity: i64,
    pub uidnext: i64,
    pub mailbox_id: String,
}
pub struct FolderByNameBorrowed<'a> {
    pub id: i64,
    pub uidvalidity: i64,
    pub uidnext: i64,
    pub mailbox_id: &'a str,
}
impl<'a> From<FolderByNameBorrowed<'a>> for FolderByName {
    fn from(
        FolderByNameBorrowed {
            id,
            uidvalidity,
            uidnext,
            mailbox_id,
        }: FolderByNameBorrowed<'a>,
    ) -> Self {
        Self {
            id,
            uidvalidity,
            uidnext,
            mailbox_id: mailbox_id.into(),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct FolderChildRow {
    pub id: i64,
    pub imap_name: String,
    pub mailbox_id: String,
}
pub struct FolderChildRowBorrowed<'a> {
    pub id: i64,
    pub imap_name: &'a str,
    pub mailbox_id: &'a str,
}
impl<'a> From<FolderChildRowBorrowed<'a>> for FolderChildRow {
    fn from(
        FolderChildRowBorrowed {
            id,
            imap_name,
            mailbox_id,
        }: FolderChildRowBorrowed<'a>,
    ) -> Self {
        Self {
            id,
            imap_name: imap_name.into(),
            mailbox_id: mailbox_id.into(),
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
pub struct FolderRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<FolderRowBorrowed, tokio_postgres::Error>,
    mapper: fn(FolderRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FolderRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(FolderRowBorrowed) -> R) -> FolderRowQuery<'c, 'a, 's, C, R, N> {
        FolderRowQuery {
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
pub struct FolderSyncStateRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<FolderSyncStateRowBorrowed, tokio_postgres::Error>,
    mapper: fn(FolderSyncStateRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FolderSyncStateRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(FolderSyncStateRowBorrowed) -> R,
    ) -> FolderSyncStateRowQuery<'c, 'a, 's, C, R, N> {
        FolderSyncStateRowQuery {
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
pub struct FolderByNameQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<FolderByNameBorrowed, tokio_postgres::Error>,
    mapper: fn(FolderByNameBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FolderByNameQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(FolderByNameBorrowed) -> R,
    ) -> FolderByNameQuery<'c, 'a, 's, C, R, N> {
        FolderByNameQuery {
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
pub struct FolderChildRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<FolderChildRowBorrowed, tokio_postgres::Error>,
    mapper: fn(FolderChildRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> FolderChildRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(FolderChildRowBorrowed) -> R,
    ) -> FolderChildRowQuery<'c, 'a, 's, C, R, N> {
        FolderChildRowQuery {
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
pub struct UpsertFolderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_folder() -> UpsertFolderStmt {
    UpsertFolderStmt(
        "INSERT INTO folders ( account_id, imap_name, uidvalidity, uidnext, highestmodseq, role, mailbox_id ) VALUES ( $1, $2, $3, $4, $5, $6, $7 ) ON CONFLICT (account_id, imap_name) DO UPDATE SET uidvalidity = EXCLUDED.uidvalidity, uidnext = EXCLUDED.uidnext, highestmodseq = EXCLUDED.highestmodseq, role = EXCLUDED.role RETURNING id",
        None,
    )
}
impl UpsertFolderStmt {
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
        T4: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        imap_name: &'a T2,
        uidvalidity: &'a i64,
        uidnext: &'a i64,
        highestmodseq: &'a i64,
        role: &'a Option<T3>,
        mailbox_id: &'a T4,
    ) -> I64Query<'c, 'a, 's, C, i64, 7> {
        I64Query {
            client,
            params: [
                account_id,
                imap_name,
                uidvalidity,
                uidnext,
                highestmodseq,
                role,
                mailbox_id,
            ],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
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
    T3: crate::StringSql,
    T4: crate::StringSql,
>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        UpsertFolderParams<T1, T2, T3, T4>,
        I64Query<'c, 'a, 's, C, i64, 7>,
        C,
    > for UpsertFolderStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a UpsertFolderParams<T1, T2, T3, T4>,
    ) -> I64Query<'c, 'a, 's, C, i64, 7> {
        self.bind(
            client,
            &params.account_id,
            &params.imap_name,
            &params.uidvalidity,
            &params.uidnext,
            &params.highestmodseq,
            &params.role,
            &params.mailbox_id,
        )
    }
}
pub struct ListFoldersStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_folders() -> ListFoldersStmt {
    ListFoldersStmt(
        "SELECT id, account_id, imap_name, uidvalidity, uidnext, uidfirst, highestmodseq, role, last_sync_at, mailbox_id FROM folders WHERE account_id = $1 ORDER BY id",
        None,
    )
}
impl ListFoldersStmt {
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
    ) -> FolderRowQuery<'c, 'a, 's, C, FolderRow, 1> {
        FolderRowQuery {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<FolderRowBorrowed, tokio_postgres::Error> {
                    Ok(FolderRowBorrowed {
                        id: row.try_get(0)?,
                        account_id: row.try_get(1)?,
                        imap_name: row.try_get(2)?,
                        uidvalidity: row.try_get(3)?,
                        uidnext: row.try_get(4)?,
                        uidfirst: row.try_get(5)?,
                        highestmodseq: row.try_get(6)?,
                        role: row.try_get(7)?,
                        last_sync_at: row.try_get(8)?,
                        mailbox_id: row.try_get(9)?,
                    })
                },
            mapper: |it| FolderRow::from(it),
        }
    }
}
pub struct FolderSyncStateStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn folder_sync_state() -> FolderSyncStateStmt {
    FolderSyncStateStmt(
        "SELECT uidfirst, mailbox_id FROM folders WHERE account_id = $1 AND imap_name = $2",
        None,
    )
}
impl FolderSyncStateStmt {
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
        imap_name: &'a T2,
    ) -> FolderSyncStateRowQuery<'c, 'a, 's, C, FolderSyncStateRow, 2> {
        FolderSyncStateRowQuery {
            client,
            params: [account_id, imap_name],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<FolderSyncStateRowBorrowed, tokio_postgres::Error> {
                Ok(FolderSyncStateRowBorrowed {
                    uidfirst: row.try_get(0)?,
                    mailbox_id: row.try_get(1)?,
                })
            },
            mapper: |it| FolderSyncStateRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        FolderSyncStateParams<T1, T2>,
        FolderSyncStateRowQuery<'c, 'a, 's, C, FolderSyncStateRow, 2>,
        C,
    > for FolderSyncStateStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a FolderSyncStateParams<T1, T2>,
    ) -> FolderSyncStateRowQuery<'c, 'a, 's, C, FolderSyncStateRow, 2> {
        self.bind(client, &params.account_id, &params.imap_name)
    }
}
pub struct FolderByNameStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn folder_by_name() -> FolderByNameStmt {
    FolderByNameStmt(
        "SELECT id, uidvalidity, uidnext, mailbox_id FROM folders WHERE account_id = $1 AND imap_name = $2",
        None,
    )
}
impl FolderByNameStmt {
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
        imap_name: &'a T2,
    ) -> FolderByNameQuery<'c, 'a, 's, C, FolderByName, 2> {
        FolderByNameQuery {
            client,
            params: [account_id, imap_name],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<FolderByNameBorrowed, tokio_postgres::Error> {
                    Ok(FolderByNameBorrowed {
                        id: row.try_get(0)?,
                        uidvalidity: row.try_get(1)?,
                        uidnext: row.try_get(2)?,
                        mailbox_id: row.try_get(3)?,
                    })
                },
            mapper: |it| FolderByName::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        FolderByNameParams<T1, T2>,
        FolderByNameQuery<'c, 'a, 's, C, FolderByName, 2>,
        C,
    > for FolderByNameStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a FolderByNameParams<T1, T2>,
    ) -> FolderByNameQuery<'c, 'a, 's, C, FolderByName, 2> {
        self.bind(client, &params.account_id, &params.imap_name)
    }
}
pub struct SetFolderUidfirstStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_folder_uidfirst() -> SetFolderUidfirstStmt {
    SetFolderUidfirstStmt("UPDATE folders SET uidfirst = $1 WHERE id = $2", None)
}
impl SetFolderUidfirstStmt {
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
        uidfirst: &'a i64,
        id: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[uidfirst, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        SetFolderUidfirstParams,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetFolderUidfirstStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetFolderUidfirstParams,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.uidfirst, &params.id))
    }
}
pub struct SetFolderUidnextStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_folder_uidnext() -> SetFolderUidnextStmt {
    SetFolderUidnextStmt("UPDATE folders SET uidnext = $1 WHERE id = $2", None)
}
impl SetFolderUidnextStmt {
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
        uidnext: &'a i64,
        id: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[uidnext, id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        SetFolderUidnextParams,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetFolderUidnextStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetFolderUidnextParams,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.uidnext, &params.id))
    }
}
pub struct RenameFolderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn rename_folder() -> RenameFolderStmt {
    RenameFolderStmt(
        "UPDATE folders SET imap_name = $1 WHERE id = $2 AND account_id = $3",
        None,
    )
}
impl RenameFolderStmt {
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
        imap_name: &'a T1,
        id: &'a i64,
        account_id: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[imap_name, id, account_id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        RenameFolderParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for RenameFolderStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a RenameFolderParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.imap_name, &params.id, &params.account_id))
    }
}
pub struct DeleteFolderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_folder() -> DeleteFolderStmt {
    DeleteFolderStmt(
        "DELETE FROM folders WHERE id = $1 AND account_id = $2",
        None,
    )
}
impl DeleteFolderStmt {
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
        id: &'a i64,
        account_id: &'a T1,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[id, account_id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        DeleteFolderParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for DeleteFolderStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a DeleteFolderParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.id, &params.account_id))
    }
}
pub struct ResetFolderSyncStateStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn reset_folder_sync_state() -> ResetFolderSyncStateStmt {
    ResetFolderSyncStateStmt(
        "UPDATE folders SET uidvalidity = 0, uidnext = 0, uidfirst = 0, highestmodseq = 0 WHERE id = $1 AND account_id = $2",
        None,
    )
}
impl ResetFolderSyncStateStmt {
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
        id: &'a i64,
        account_id: &'a T1,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[id, account_id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        ResetFolderSyncStateParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for ResetFolderSyncStateStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a ResetFolderSyncStateParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.id, &params.account_id))
    }
}
pub struct FolderChildrenStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn folder_children() -> FolderChildrenStmt {
    FolderChildrenStmt(
        "SELECT id, imap_name, mailbox_id FROM folders WHERE account_id = $1 AND left(imap_name, length($2)) = $2",
        None,
    )
}
impl FolderChildrenStmt {
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
        prefix: &'a T2,
    ) -> FolderChildRowQuery<'c, 'a, 's, C, FolderChildRow, 2> {
        FolderChildRowQuery {
            client,
            params: [account_id, prefix],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<FolderChildRowBorrowed, tokio_postgres::Error> {
                Ok(FolderChildRowBorrowed {
                    id: row.try_get(0)?,
                    imap_name: row.try_get(1)?,
                    mailbox_id: row.try_get(2)?,
                })
            },
            mapper: |it| FolderChildRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        FolderChildrenParams<T1, T2>,
        FolderChildRowQuery<'c, 'a, 's, C, FolderChildRow, 2>,
        C,
    > for FolderChildrenStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a FolderChildrenParams<T1, T2>,
    ) -> FolderChildRowQuery<'c, 'a, 's, C, FolderChildRow, 2> {
        self.bind(client, &params.account_id, &params.prefix)
    }
}
