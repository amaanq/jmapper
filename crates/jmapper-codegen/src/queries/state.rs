// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct StateRow {
    pub email_modseq: i64,
    pub mailbox_modseq: i64,
    pub submission_modseq: i64,
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
pub struct StateRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<StateRow, tokio_postgres::Error>,
    mapper: fn(StateRow) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> StateRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(self, mapper: fn(StateRow) -> R) -> StateRowQuery<'c, 'a, 's, C, R, N> {
        StateRowQuery {
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
pub struct MarkInitialSyncDoneStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn mark_initial_sync_done() -> MarkInitialSyncDoneStmt {
    MarkInitialSyncDoneStmt(
        "UPDATE state SET initial_sync_done = 1 WHERE account_id = $1",
        None,
    )
}
impl MarkInitialSyncDoneStmt {
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
pub struct BumpEmailModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bump_email_modseq() -> BumpEmailModseqStmt {
    BumpEmailModseqStmt(
        "UPDATE state SET email_modseq = email_modseq + 1 WHERE account_id = $1 RETURNING email_modseq",
        None,
    )
}
impl BumpEmailModseqStmt {
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
pub struct BumpMailboxModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bump_mailbox_modseq() -> BumpMailboxModseqStmt {
    BumpMailboxModseqStmt(
        "UPDATE state SET mailbox_modseq = mailbox_modseq + 1 WHERE account_id = $1 RETURNING mailbox_modseq",
        None,
    )
}
impl BumpMailboxModseqStmt {
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
pub struct BumpSubmissionModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn bump_submission_modseq() -> BumpSubmissionModseqStmt {
    BumpSubmissionModseqStmt(
        "UPDATE state SET submission_modseq = submission_modseq + 1 WHERE account_id = $1 RETURNING submission_modseq",
        None,
    )
}
impl BumpSubmissionModseqStmt {
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
pub struct GetStateStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_state() -> GetStateStmt {
    GetStateStmt(
        "SELECT email_modseq, mailbox_modseq, submission_modseq FROM state WHERE account_id = $1",
        None,
    )
}
impl GetStateStmt {
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
    ) -> StateRowQuery<'c, 'a, 's, C, StateRow, 1> {
        StateRowQuery {
            client,
            params: [account_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row: &tokio_postgres::Row| -> Result<StateRow, tokio_postgres::Error> {
                Ok(StateRow {
                    email_modseq: row.try_get(0)?,
                    mailbox_modseq: row.try_get(1)?,
                    submission_modseq: row.try_get(2)?,
                })
            },
            mapper: |it| StateRow::from(it),
        }
    }
}
pub struct CountReadyAccountsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn count_ready_accounts() -> CountReadyAccountsStmt {
    CountReadyAccountsStmt(
        "SELECT COUNT(*) FROM state WHERE initial_sync_done = 1 AND account_id = ANY($1)",
        None,
    )
}
impl CountReadyAccountsStmt {
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
        T2: crate::ArraySql<Item = T1>,
    >(
        &'s self,
        client: &'c C,
        account_ids: &'a T2,
    ) -> I64Query<'c, 'a, 's, C, i64, 1> {
        I64Query {
            client,
            params: [account_ids],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
        }
    }
}
