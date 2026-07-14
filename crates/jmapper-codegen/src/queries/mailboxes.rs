// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct UpsertMailboxParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
> {
    pub id: T1,
    pub account_id: T2,
    pub name: T3,
    pub parent_id: Option<T4>,
    pub role: Option<T5>,
    pub sort_order: i64,
    pub modseq: i64,
}
#[derive(Debug)]
pub struct MailboxMetadataParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub id: T1,
    pub account_id: T2,
}
#[derive(Debug)]
pub struct GetMailboxesByIdsParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::ArraySql<Item = T2>,
> {
    pub account_id: T1,
    pub ids: T3,
}
#[derive(Debug)]
pub struct SetMailboxNameParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql> {
    pub name: T1,
    pub modseq: i64,
    pub id: T2,
    pub account_id: T3,
}
#[derive(Debug)]
pub struct DeleteMailboxParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub id: T1,
    pub account_id: T2,
}
#[derive(Debug)]
pub struct ResolveMailboxFoldersParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::ArraySql<Item = T2>,
> {
    pub account_id: T1,
    pub mailbox_ids: T3,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MailboxMetadataRow {
    pub name: String,
    pub parent_id: Option<String>,
    pub role: Option<String>,
    pub sort_order: i64,
}
pub struct MailboxMetadataRowBorrowed<'a> {
    pub name: &'a str,
    pub parent_id: Option<&'a str>,
    pub role: Option<&'a str>,
    pub sort_order: i64,
}
impl<'a> From<MailboxMetadataRowBorrowed<'a>> for MailboxMetadataRow {
    fn from(
        MailboxMetadataRowBorrowed {
            name,
            parent_id,
            role,
            sort_order,
        }: MailboxMetadataRowBorrowed<'a>,
    ) -> Self {
        Self {
            name: name.into(),
            parent_id: parent_id.map(|v| v.into()),
            role: role.map(|v| v.into()),
            sort_order,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct MailboxRow {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub role: Option<String>,
    pub total_emails: i64,
    pub unread_emails: i64,
    pub total_threads: i64,
    pub unread_threads: i64,
    pub sort_order: i64,
    pub modseq: i64,
}
pub struct MailboxRowBorrowed<'a> {
    pub id: &'a str,
    pub account_id: &'a str,
    pub name: &'a str,
    pub parent_id: Option<&'a str>,
    pub role: Option<&'a str>,
    pub total_emails: i64,
    pub unread_emails: i64,
    pub total_threads: i64,
    pub unread_threads: i64,
    pub sort_order: i64,
    pub modseq: i64,
}
impl<'a> From<MailboxRowBorrowed<'a>> for MailboxRow {
    fn from(
        MailboxRowBorrowed {
            id,
            account_id,
            name,
            parent_id,
            role,
            total_emails,
            unread_emails,
            total_threads,
            unread_threads,
            sort_order,
            modseq,
        }: MailboxRowBorrowed<'a>,
    ) -> Self {
        Self {
            id: id.into(),
            account_id: account_id.into(),
            name: name.into(),
            parent_id: parent_id.map(|v| v.into()),
            role: role.map(|v| v.into()),
            total_emails,
            unread_emails,
            total_threads,
            unread_threads,
            sort_order,
            modseq,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ResolveMailboxFolders {
    pub id: i64,
    pub imap_name: String,
    pub mailbox_id: String,
}
pub struct ResolveMailboxFoldersBorrowed<'a> {
    pub id: i64,
    pub imap_name: &'a str,
    pub mailbox_id: &'a str,
}
impl<'a> From<ResolveMailboxFoldersBorrowed<'a>> for ResolveMailboxFolders {
    fn from(
        ResolveMailboxFoldersBorrowed {
            id,
            imap_name,
            mailbox_id,
        }: ResolveMailboxFoldersBorrowed<'a>,
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
pub struct MailboxMetadataRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<MailboxMetadataRowBorrowed, tokio_postgres::Error>,
    mapper: fn(MailboxMetadataRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> MailboxMetadataRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(MailboxMetadataRowBorrowed) -> R,
    ) -> MailboxMetadataRowQuery<'c, 'a, 's, C, R, N> {
        MailboxMetadataRowQuery {
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
pub struct MailboxRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<MailboxRowBorrowed, tokio_postgres::Error>,
    mapper: fn(MailboxRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> MailboxRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(MailboxRowBorrowed) -> R,
    ) -> MailboxRowQuery<'c, 'a, 's, C, R, N> {
        MailboxRowQuery {
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
pub struct ResolveMailboxFoldersQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<ResolveMailboxFoldersBorrowed, tokio_postgres::Error>,
    mapper: fn(ResolveMailboxFoldersBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> ResolveMailboxFoldersQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(ResolveMailboxFoldersBorrowed) -> R,
    ) -> ResolveMailboxFoldersQuery<'c, 'a, 's, C, R, N> {
        ResolveMailboxFoldersQuery {
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
pub struct UpsertMailboxStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_mailbox() -> UpsertMailboxStmt {
    UpsertMailboxStmt(
        "INSERT INTO mailboxes (id, account_id, name, parent_id, role, sort_order, modseq) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name, parent_id = EXCLUDED.parent_id, role = EXCLUDED.role, sort_order = EXCLUDED.sort_order, modseq = EXCLUDED.modseq",
        None,
    )
}
impl UpsertMailboxStmt {
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
    >(
        &'s self,
        client: &'c C,
        id: &'a T1,
        account_id: &'a T2,
        name: &'a T3,
        parent_id: &'a Option<T4>,
        role: &'a Option<T5>,
        sort_order: &'a i64,
        modseq: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[id, account_id, name, parent_id, role, sort_order, modseq],
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
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertMailboxParams<T1, T2, T3, T4, T5>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertMailboxStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertMailboxParams<T1, T2, T3, T4, T5>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.id,
            &params.account_id,
            &params.name,
            &params.parent_id,
            &params.role,
            &params.sort_order,
            &params.modseq,
        ))
    }
}
pub struct MailboxMetadataStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn mailbox_metadata() -> MailboxMetadataStmt {
    MailboxMetadataStmt(
        "SELECT name, parent_id, role, sort_order FROM mailboxes WHERE id = $1 AND account_id = $2",
        None,
    )
}
impl MailboxMetadataStmt {
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
        id: &'a T1,
        account_id: &'a T2,
    ) -> MailboxMetadataRowQuery<'c, 'a, 's, C, MailboxMetadataRow, 2> {
        MailboxMetadataRowQuery {
            client,
            params: [id, account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<MailboxMetadataRowBorrowed, tokio_postgres::Error> {
                Ok(MailboxMetadataRowBorrowed {
                    name: row.try_get(0)?,
                    parent_id: row.try_get(1)?,
                    role: row.try_get(2)?,
                    sort_order: row.try_get(3)?,
                })
            },
            mapper: |it| MailboxMetadataRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        MailboxMetadataParams<T1, T2>,
        MailboxMetadataRowQuery<'c, 'a, 's, C, MailboxMetadataRow, 2>,
        C,
    > for MailboxMetadataStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MailboxMetadataParams<T1, T2>,
    ) -> MailboxMetadataRowQuery<'c, 'a, 's, C, MailboxMetadataRow, 2> {
        self.bind(client, &params.id, &params.account_id)
    }
}
pub struct ListMailboxesStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn list_mailboxes() -> ListMailboxesStmt {
    ListMailboxesStmt(
        "SELECT id, account_id, name, parent_id, role, total_emails, unread_emails, total_threads, unread_threads, sort_order, modseq FROM mailboxes WHERE account_id = $1 ORDER BY sort_order, name",
        None,
    )
}
impl ListMailboxesStmt {
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
    ) -> MailboxRowQuery<'c, 'a, 's, C, MailboxRow, 1> {
        MailboxRowQuery {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<MailboxRowBorrowed, tokio_postgres::Error> {
                    Ok(MailboxRowBorrowed {
                        id: row.try_get(0)?,
                        account_id: row.try_get(1)?,
                        name: row.try_get(2)?,
                        parent_id: row.try_get(3)?,
                        role: row.try_get(4)?,
                        total_emails: row.try_get(5)?,
                        unread_emails: row.try_get(6)?,
                        total_threads: row.try_get(7)?,
                        unread_threads: row.try_get(8)?,
                        sort_order: row.try_get(9)?,
                        modseq: row.try_get(10)?,
                    })
                },
            mapper: |it| MailboxRow::from(it),
        }
    }
}
pub struct GetMailboxesByIdsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_mailboxes_by_ids() -> GetMailboxesByIdsStmt {
    GetMailboxesByIdsStmt(
        "SELECT id, account_id, name, parent_id, role, total_emails, unread_emails, total_threads, unread_threads, sort_order, modseq FROM mailboxes WHERE account_id = $1 AND id = ANY($2)",
        None,
    )
}
impl GetMailboxesByIdsStmt {
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
    ) -> MailboxRowQuery<'c, 'a, 's, C, MailboxRow, 2> {
        MailboxRowQuery {
            client,
            params: [account_id, ids],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<MailboxRowBorrowed, tokio_postgres::Error> {
                    Ok(MailboxRowBorrowed {
                        id: row.try_get(0)?,
                        account_id: row.try_get(1)?,
                        name: row.try_get(2)?,
                        parent_id: row.try_get(3)?,
                        role: row.try_get(4)?,
                        total_emails: row.try_get(5)?,
                        unread_emails: row.try_get(6)?,
                        total_threads: row.try_get(7)?,
                        unread_threads: row.try_get(8)?,
                        sort_order: row.try_get(9)?,
                        modseq: row.try_get(10)?,
                    })
                },
            mapper: |it| MailboxRow::from(it),
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
        GetMailboxesByIdsParams<T1, T2, T3>,
        MailboxRowQuery<'c, 'a, 's, C, MailboxRow, 2>,
        C,
    > for GetMailboxesByIdsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetMailboxesByIdsParams<T1, T2, T3>,
    ) -> MailboxRowQuery<'c, 'a, 's, C, MailboxRow, 2> {
        self.bind(client, &params.account_id, &params.ids)
    }
}
pub struct SetMailboxNameStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_mailbox_name() -> SetMailboxNameStmt {
    SetMailboxNameStmt(
        "UPDATE mailboxes SET name = $1, modseq = $2 WHERE id = $3 AND account_id = $4",
        None,
    )
}
impl SetMailboxNameStmt {
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
        name: &'a T1,
        modseq: &'a i64,
        id: &'a T2,
        account_id: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[name, modseq, id, account_id])
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
        SetMailboxNameParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetMailboxNameStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetMailboxNameParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.name,
            &params.modseq,
            &params.id,
            &params.account_id,
        ))
    }
}
pub struct DeleteMailboxStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_mailbox() -> DeleteMailboxStmt {
    DeleteMailboxStmt(
        "DELETE FROM mailboxes WHERE id = $1 AND account_id = $2",
        None,
    )
}
impl DeleteMailboxStmt {
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
        id: &'a T1,
        account_id: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[id, account_id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        DeleteMailboxParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for DeleteMailboxStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a DeleteMailboxParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.id, &params.account_id))
    }
}
pub struct RecomputeMailboxCountsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn recompute_mailbox_counts() -> RecomputeMailboxCountsStmt {
    RecomputeMailboxCountsStmt(
        "WITH counts AS ( SELECT m.id, COUNT(mm.msgid) AS total_emails, COUNT(mm.msgid) FILTER ( WHERE strpos(msg.flags_json, '\"$seen\"') = 0 ) AS unread_emails, COUNT(DISTINCT msg.thrid) AS total_threads, COUNT(DISTINCT msg.thrid) FILTER ( WHERE strpos(msg.flags_json, '\"$seen\"') = 0 ) AS unread_threads FROM mailboxes m LEFT JOIN message_mailboxes mm ON mm.account_id = m.account_id AND mm.mailbox_id = m.id LEFT JOIN messages msg ON msg.account_id = mm.account_id AND msg.msgid = mm.msgid WHERE m.account_id = $1 GROUP BY m.id ), changed AS ( SELECT counts.* FROM counts JOIN mailboxes m ON m.id = counts.id WHERE (m.total_emails, m.unread_emails, m.total_threads, m.unread_threads) IS DISTINCT FROM (counts.total_emails, counts.unread_emails, counts.total_threads, counts.unread_threads) ), new_state AS ( UPDATE state SET mailbox_modseq = mailbox_modseq + 1 WHERE account_id = $1 AND EXISTS (SELECT 1 FROM changed) RETURNING mailbox_modseq ) UPDATE mailboxes m SET total_emails = changed.total_emails, unread_emails = changed.unread_emails, total_threads = changed.total_threads, unread_threads = changed.unread_threads, modseq = (SELECT mailbox_modseq FROM new_state) FROM changed WHERE m.id = changed.id",
        None,
    )
}
impl RecomputeMailboxCountsStmt {
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
pub struct ResolveMailboxFoldersStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn resolve_mailbox_folders() -> ResolveMailboxFoldersStmt {
    ResolveMailboxFoldersStmt(
        "SELECT f.id, f.imap_name, m.id AS mailbox_id FROM folders f JOIN mailboxes m ON m.account_id = f.account_id AND m.id = f.mailbox_id WHERE f.account_id = $1 AND m.id = ANY($2)",
        None,
    )
}
impl ResolveMailboxFoldersStmt {
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
        mailbox_ids: &'a T3,
    ) -> ResolveMailboxFoldersQuery<'c, 'a, 's, C, ResolveMailboxFolders, 2> {
        ResolveMailboxFoldersQuery {
            client,
            params: [account_id, mailbox_ids],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<ResolveMailboxFoldersBorrowed, tokio_postgres::Error> {
                Ok(ResolveMailboxFoldersBorrowed {
                    id: row.try_get(0)?,
                    imap_name: row.try_get(1)?,
                    mailbox_id: row.try_get(2)?,
                })
            },
            mapper: |it| ResolveMailboxFolders::from(it),
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
        ResolveMailboxFoldersParams<T1, T2, T3>,
        ResolveMailboxFoldersQuery<'c, 'a, 's, C, ResolveMailboxFolders, 2>,
        C,
    > for ResolveMailboxFoldersStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ResolveMailboxFoldersParams<T1, T2, T3>,
    ) -> ResolveMailboxFoldersQuery<'c, 'a, 's, C, ResolveMailboxFolders, 2> {
        self.bind(client, &params.account_id, &params.mailbox_ids)
    }
}
