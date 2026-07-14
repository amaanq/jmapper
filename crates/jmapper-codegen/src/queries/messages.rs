// This file was generated with `cornucopia`. Do not modify.

#[derive(Debug)]
pub struct GetEnvelopeForCompareParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct UpsertMessageParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
    T5: crate::StringSql,
    T6: crate::StringSql,
    T7: crate::StringSql,
    T8: crate::StringSql,
    T9: crate::StringSql,
    T10: crate::StringSql,
    T11: crate::StringSql,
    T12: crate::StringSql,
    T13: crate::StringSql,
    T14: crate::StringSql,
> {
    pub account_id: T1,
    pub msgid: T2,
    pub thrid: T3,
    pub flags_json: T4,
    pub received_at: i64,
    pub sent_at: Option<i64>,
    pub size: i64,
    pub from_json: Option<T5>,
    pub to_json: Option<T6>,
    pub cc_json: Option<T7>,
    pub bcc_json: Option<T8>,
    pub reply_to_json: Option<T9>,
    pub subject: Option<T10>,
    pub preview: Option<T11>,
    pub has_attachment: i64,
    pub message_id_header: Option<T12>,
    pub in_reply_to_header: Option<T13>,
    pub references_header: Option<T14>,
    pub modseq: i64,
}
#[derive(Debug)]
pub struct DeleteMessageParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct MessageFlagsJsonParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct SetMessageFlagsParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql> {
    pub flags_json: T1,
    pub modseq: i64,
    pub account_id: T2,
    pub msgid: T3,
}
#[derive(Debug)]
pub struct SetMessageModseqParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub modseq: i64,
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct MessageBodyMetadataParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct UpdateMessageBodyCacheParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
> {
    pub preview: T1,
    pub has_attachment: i64,
    pub modseq: i64,
    pub account_id: T2,
    pub msgid: T3,
}
#[derive(Debug)]
pub struct RepairMessageBodyMetadataParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
    T4: crate::StringSql,
> {
    pub subject: Option<T1>,
    pub preview: T2,
    pub has_attachment: i64,
    pub modseq: i64,
    pub account_id: T3,
    pub msgid: T4,
}
#[derive(Debug)]
pub struct MessageAddressesParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct AddMessageMailboxParams<T1: crate::StringSql, T2: crate::StringSql, T3: crate::StringSql>
{
    pub account_id: T1,
    pub msgid: T2,
    pub mailbox_id: T3,
}
#[derive(Debug)]
pub struct RemoveMessageMailboxParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::StringSql,
> {
    pub account_id: T1,
    pub msgid: T2,
    pub mailbox_id: T3,
}
#[derive(Debug)]
pub struct ClearMessageMailboxesParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct MessageMailboxIdsParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct UpsertMessageImapParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
    pub folder_id: i64,
    pub uid: i64,
    pub uidvalidity: i64,
}
#[derive(Debug)]
pub struct GetMessageImapInFolderParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
    pub folder_id: i64,
}
#[derive(Debug)]
pub struct GetMessageImapAnyParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct MessageLocationsParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug)]
pub struct PreferredMessageLocationsParams<
    T1: crate::StringSql,
    T2: crate::StringSql,
    T3: crate::ArraySql<Item = T2>,
> {
    pub account_id: T1,
    pub msgids: T3,
}
#[derive(Debug)]
pub struct ImportedMessageByHeaderParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub folder_id: i64,
    pub message_id_header: T2,
}
#[derive(Debug)]
pub struct MsgidsInFolderParams<T1: crate::StringSql> {
    pub account_id: T1,
    pub folder_id: i64,
}
#[derive(Debug)]
pub struct UidsInFolderParams<T1: crate::StringSql> {
    pub account_id: T1,
    pub folder_id: i64,
}
#[derive(Debug)]
pub struct MsgidForFolderUidParams<T1: crate::StringSql> {
    pub account_id: T1,
    pub folder_id: i64,
    pub uid: i64,
}
#[derive(Debug)]
pub struct DeleteMessageImapByUidParams<T1: crate::StringSql> {
    pub account_id: T1,
    pub folder_id: i64,
    pub uid: i64,
}
#[derive(Debug)]
pub struct DeleteMessageImapForFolderParams<T1: crate::StringSql> {
    pub account_id: T1,
    pub folder_id: i64,
}
#[derive(Debug)]
pub struct CountMessageImapParams<T1: crate::StringSql, T2: crate::StringSql> {
    pub account_id: T1,
    pub msgid: T2,
}
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeCmpRow {
    pub thrid: String,
    pub flags_json: String,
    pub received_at: i64,
    pub sent_at: Option<i64>,
    pub size: i64,
    pub from_json: Option<String>,
    pub to_json: Option<String>,
    pub cc_json: Option<String>,
    pub bcc_json: Option<String>,
    pub reply_to_json: Option<String>,
    pub subject: Option<String>,
    pub preview: Option<String>,
    pub has_attachment: i64,
    pub message_id_header: Option<String>,
    pub in_reply_to_header: Option<String>,
    pub references_header: Option<String>,
}
pub struct EnvelopeCmpRowBorrowed<'a> {
    pub thrid: &'a str,
    pub flags_json: &'a str,
    pub received_at: i64,
    pub sent_at: Option<i64>,
    pub size: i64,
    pub from_json: Option<&'a str>,
    pub to_json: Option<&'a str>,
    pub cc_json: Option<&'a str>,
    pub bcc_json: Option<&'a str>,
    pub reply_to_json: Option<&'a str>,
    pub subject: Option<&'a str>,
    pub preview: Option<&'a str>,
    pub has_attachment: i64,
    pub message_id_header: Option<&'a str>,
    pub in_reply_to_header: Option<&'a str>,
    pub references_header: Option<&'a str>,
}
impl<'a> From<EnvelopeCmpRowBorrowed<'a>> for EnvelopeCmpRow {
    fn from(
        EnvelopeCmpRowBorrowed {
            thrid,
            flags_json,
            received_at,
            sent_at,
            size,
            from_json,
            to_json,
            cc_json,
            bcc_json,
            reply_to_json,
            subject,
            preview,
            has_attachment,
            message_id_header,
            in_reply_to_header,
            references_header,
        }: EnvelopeCmpRowBorrowed<'a>,
    ) -> Self {
        Self {
            thrid: thrid.into(),
            flags_json: flags_json.into(),
            received_at,
            sent_at,
            size,
            from_json: from_json.map(|v| v.into()),
            to_json: to_json.map(|v| v.into()),
            cc_json: cc_json.map(|v| v.into()),
            bcc_json: bcc_json.map(|v| v.into()),
            reply_to_json: reply_to_json.map(|v| v.into()),
            subject: subject.map(|v| v.into()),
            preview: preview.map(|v| v.into()),
            has_attachment,
            message_id_header: message_id_header.map(|v| v.into()),
            in_reply_to_header: in_reply_to_header.map(|v| v.into()),
            references_header: references_header.map(|v| v.into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct MessageBodyMetadataRow {
    pub subject: Option<String>,
    pub preview: Option<String>,
    pub has_attachment: i64,
}
pub struct MessageBodyMetadataRowBorrowed<'a> {
    pub subject: Option<&'a str>,
    pub preview: Option<&'a str>,
    pub has_attachment: i64,
}
impl<'a> From<MessageBodyMetadataRowBorrowed<'a>> for MessageBodyMetadataRow {
    fn from(
        MessageBodyMetadataRowBorrowed {
            subject,
            preview,
            has_attachment,
        }: MessageBodyMetadataRowBorrowed<'a>,
    ) -> Self {
        Self {
            subject: subject.map(|v| v.into()),
            preview: preview.map(|v| v.into()),
            has_attachment,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct MessageAddressesRow {
    pub thread_id: Option<String>,
    pub to_json: Option<String>,
    pub cc_json: Option<String>,
    pub bcc_json: Option<String>,
}
pub struct MessageAddressesRowBorrowed<'a> {
    pub thread_id: Option<&'a str>,
    pub to_json: Option<&'a str>,
    pub cc_json: Option<&'a str>,
    pub bcc_json: Option<&'a str>,
}
impl<'a> From<MessageAddressesRowBorrowed<'a>> for MessageAddressesRow {
    fn from(
        MessageAddressesRowBorrowed {
            thread_id,
            to_json,
            cc_json,
            bcc_json,
        }: MessageAddressesRowBorrowed<'a>,
    ) -> Self {
        Self {
            thread_id: thread_id.map(|v| v.into()),
            to_json: to_json.map(|v| v.into()),
            cc_json: cc_json.map(|v| v.into()),
            bcc_json: bcc_json.map(|v| v.into()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct MessageImapRow {
    pub folder_id: i64,
    pub uid: i64,
    pub uidvalidity: i64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MessageLocationRow {
    pub folder_id: i64,
    pub imap_name: String,
    pub uid: i64,
    pub uidvalidity: i64,
}
pub struct MessageLocationRowBorrowed<'a> {
    pub folder_id: i64,
    pub imap_name: &'a str,
    pub uid: i64,
    pub uidvalidity: i64,
}
impl<'a> From<MessageLocationRowBorrowed<'a>> for MessageLocationRow {
    fn from(
        MessageLocationRowBorrowed {
            folder_id,
            imap_name,
            uid,
            uidvalidity,
        }: MessageLocationRowBorrowed<'a>,
    ) -> Self {
        Self {
            folder_id,
            imap_name: imap_name.into(),
            uid,
            uidvalidity,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct PreferredMessageLocationRow {
    pub msgid: String,
    pub imap_name: String,
    pub uid: i64,
    pub uidvalidity: i64,
}
pub struct PreferredMessageLocationRowBorrowed<'a> {
    pub msgid: &'a str,
    pub imap_name: &'a str,
    pub uid: i64,
    pub uidvalidity: i64,
}
impl<'a> From<PreferredMessageLocationRowBorrowed<'a>> for PreferredMessageLocationRow {
    fn from(
        PreferredMessageLocationRowBorrowed {
            msgid,
            imap_name,
            uid,
            uidvalidity,
        }: PreferredMessageLocationRowBorrowed<'a>,
    ) -> Self {
        Self {
            msgid: msgid.into(),
            imap_name: imap_name.into(),
            uid,
            uidvalidity,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedMessageRow {
    pub msgid: String,
    pub thrid: String,
}
pub struct ImportedMessageRowBorrowed<'a> {
    pub msgid: &'a str,
    pub thrid: &'a str,
}
impl<'a> From<ImportedMessageRowBorrowed<'a>> for ImportedMessageRow {
    fn from(ImportedMessageRowBorrowed { msgid, thrid }: ImportedMessageRowBorrowed<'a>) -> Self {
        Self {
            msgid: msgid.into(),
            thrid: thrid.into(),
        }
    }
}
use crate::client::async_::GenericClient;
use futures::{self, StreamExt, TryStreamExt};
pub struct EnvelopeCmpRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<EnvelopeCmpRowBorrowed, tokio_postgres::Error>,
    mapper: fn(EnvelopeCmpRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> EnvelopeCmpRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(EnvelopeCmpRowBorrowed) -> R,
    ) -> EnvelopeCmpRowQuery<'c, 'a, 's, C, R, N> {
        EnvelopeCmpRowQuery {
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
pub struct MessageBodyMetadataRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<MessageBodyMetadataRowBorrowed, tokio_postgres::Error>,
    mapper: fn(MessageBodyMetadataRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> MessageBodyMetadataRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(MessageBodyMetadataRowBorrowed) -> R,
    ) -> MessageBodyMetadataRowQuery<'c, 'a, 's, C, R, N> {
        MessageBodyMetadataRowQuery {
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
pub struct MessageAddressesRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<MessageAddressesRowBorrowed, tokio_postgres::Error>,
    mapper: fn(MessageAddressesRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> MessageAddressesRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(MessageAddressesRowBorrowed) -> R,
    ) -> MessageAddressesRowQuery<'c, 'a, 's, C, R, N> {
        MessageAddressesRowQuery {
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
pub struct MessageImapRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(&tokio_postgres::Row) -> Result<MessageImapRow, tokio_postgres::Error>,
    mapper: fn(MessageImapRow) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> MessageImapRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(MessageImapRow) -> R,
    ) -> MessageImapRowQuery<'c, 'a, 's, C, R, N> {
        MessageImapRowQuery {
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
pub struct MessageLocationRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<MessageLocationRowBorrowed, tokio_postgres::Error>,
    mapper: fn(MessageLocationRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> MessageLocationRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(MessageLocationRowBorrowed) -> R,
    ) -> MessageLocationRowQuery<'c, 'a, 's, C, R, N> {
        MessageLocationRowQuery {
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
pub struct PreferredMessageLocationRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor: fn(
        &tokio_postgres::Row,
    ) -> Result<PreferredMessageLocationRowBorrowed, tokio_postgres::Error>,
    mapper: fn(PreferredMessageLocationRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> PreferredMessageLocationRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(PreferredMessageLocationRowBorrowed) -> R,
    ) -> PreferredMessageLocationRowQuery<'c, 'a, 's, C, R, N> {
        PreferredMessageLocationRowQuery {
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
pub struct ImportedMessageRowQuery<'c, 'a, 's, C: GenericClient, T, const N: usize> {
    client: &'c C,
    params: [&'a (dyn postgres_types::ToSql + Sync); N],
    query: &'static str,
    cached: Option<&'s tokio_postgres::Statement>,
    extractor:
        fn(&tokio_postgres::Row) -> Result<ImportedMessageRowBorrowed, tokio_postgres::Error>,
    mapper: fn(ImportedMessageRowBorrowed) -> T,
}
impl<'c, 'a, 's, C, T: 'c, const N: usize> ImportedMessageRowQuery<'c, 'a, 's, C, T, N>
where
    C: GenericClient,
{
    pub fn map<R>(
        self,
        mapper: fn(ImportedMessageRowBorrowed) -> R,
    ) -> ImportedMessageRowQuery<'c, 'a, 's, C, R, N> {
        ImportedMessageRowQuery {
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
pub struct GetEnvelopeForCompareStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_envelope_for_compare() -> GetEnvelopeForCompareStmt {
    GetEnvelopeForCompareStmt(
        "SELECT thrid, flags_json, received_at, sent_at, size, from_json, to_json, cc_json, bcc_json, reply_to_json, subject, preview, has_attachment, message_id_header, in_reply_to_header, references_header FROM messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl GetEnvelopeForCompareStmt {
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
    ) -> EnvelopeCmpRowQuery<'c, 'a, 's, C, EnvelopeCmpRow, 2> {
        EnvelopeCmpRowQuery {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<EnvelopeCmpRowBorrowed, tokio_postgres::Error> {
                Ok(EnvelopeCmpRowBorrowed {
                    thrid: row.try_get(0)?,
                    flags_json: row.try_get(1)?,
                    received_at: row.try_get(2)?,
                    sent_at: row.try_get(3)?,
                    size: row.try_get(4)?,
                    from_json: row.try_get(5)?,
                    to_json: row.try_get(6)?,
                    cc_json: row.try_get(7)?,
                    bcc_json: row.try_get(8)?,
                    reply_to_json: row.try_get(9)?,
                    subject: row.try_get(10)?,
                    preview: row.try_get(11)?,
                    has_attachment: row.try_get(12)?,
                    message_id_header: row.try_get(13)?,
                    in_reply_to_header: row.try_get(14)?,
                    references_header: row.try_get(15)?,
                })
            },
            mapper: |it| EnvelopeCmpRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetEnvelopeForCompareParams<T1, T2>,
        EnvelopeCmpRowQuery<'c, 'a, 's, C, EnvelopeCmpRow, 2>,
        C,
    > for GetEnvelopeForCompareStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetEnvelopeForCompareParams<T1, T2>,
    ) -> EnvelopeCmpRowQuery<'c, 'a, 's, C, EnvelopeCmpRow, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct UpsertMessageStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_message() -> UpsertMessageStmt {
    UpsertMessageStmt(
        "INSERT INTO messages ( account_id, msgid, thrid, flags_json, received_at, sent_at, size, from_json, to_json, cc_json, bcc_json, reply_to_json, subject, preview, has_attachment, message_id_header, in_reply_to_header, references_header, modseq ) VALUES ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19 ) ON CONFLICT (account_id, msgid) DO UPDATE SET thrid = EXCLUDED.thrid, flags_json = EXCLUDED.flags_json, received_at = EXCLUDED.received_at, sent_at = EXCLUDED.sent_at, size = EXCLUDED.size, from_json = EXCLUDED.from_json, to_json = EXCLUDED.to_json, cc_json = EXCLUDED.cc_json, bcc_json = EXCLUDED.bcc_json, reply_to_json = EXCLUDED.reply_to_json, subject = EXCLUDED.subject, preview = EXCLUDED.preview, has_attachment = EXCLUDED.has_attachment, message_id_header = EXCLUDED.message_id_header, in_reply_to_header = EXCLUDED.in_reply_to_header, references_header = EXCLUDED.references_header, modseq = EXCLUDED.modseq",
        None,
    )
}
impl UpsertMessageStmt {
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
        T10: crate::StringSql,
        T11: crate::StringSql,
        T12: crate::StringSql,
        T13: crate::StringSql,
        T14: crate::StringSql,
    >(
        &'s self,
        client: &'c C,
        account_id: &'a T1,
        msgid: &'a T2,
        thrid: &'a T3,
        flags_json: &'a T4,
        received_at: &'a i64,
        sent_at: &'a Option<i64>,
        size: &'a i64,
        from_json: &'a Option<T5>,
        to_json: &'a Option<T6>,
        cc_json: &'a Option<T7>,
        bcc_json: &'a Option<T8>,
        reply_to_json: &'a Option<T9>,
        subject: &'a Option<T10>,
        preview: &'a Option<T11>,
        has_attachment: &'a i64,
        message_id_header: &'a Option<T12>,
        in_reply_to_header: &'a Option<T13>,
        references_header: &'a Option<T14>,
        modseq: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[
                    account_id,
                    msgid,
                    thrid,
                    flags_json,
                    received_at,
                    sent_at,
                    size,
                    from_json,
                    to_json,
                    cc_json,
                    bcc_json,
                    reply_to_json,
                    subject,
                    preview,
                    has_attachment,
                    message_id_header,
                    in_reply_to_header,
                    references_header,
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
    T10: crate::StringSql,
    T11: crate::StringSql,
    T12: crate::StringSql,
    T13: crate::StringSql,
    T14: crate::StringSql,
>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertMessageParams<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertMessageStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertMessageParams<
            T1,
            T2,
            T3,
            T4,
            T5,
            T6,
            T7,
            T8,
            T9,
            T10,
            T11,
            T12,
            T13,
            T14,
        >,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.msgid,
            &params.thrid,
            &params.flags_json,
            &params.received_at,
            &params.sent_at,
            &params.size,
            &params.from_json,
            &params.to_json,
            &params.cc_json,
            &params.bcc_json,
            &params.reply_to_json,
            &params.subject,
            &params.preview,
            &params.has_attachment,
            &params.message_id_header,
            &params.in_reply_to_header,
            &params.references_header,
            &params.modseq,
        ))
    }
}
pub struct DeleteMessageStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_message() -> DeleteMessageStmt {
    DeleteMessageStmt(
        "DELETE FROM messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl DeleteMessageStmt {
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
        msgid: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id, msgid]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        DeleteMessageParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for DeleteMessageStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a DeleteMessageParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.account_id, &params.msgid))
    }
}
pub struct MessageFlagsJsonStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn message_flags_json() -> MessageFlagsJsonStmt {
    MessageFlagsJsonStmt(
        "SELECT flags_json FROM messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl MessageFlagsJsonStmt {
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
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        StringQuery {
            client,
            params: [account_id, msgid],
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
        MessageFlagsJsonParams<T1, T2>,
        StringQuery<'c, 'a, 's, C, String, 2>,
        C,
    > for MessageFlagsJsonStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MessageFlagsJsonParams<T1, T2>,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct SetMessageFlagsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_message_flags() -> SetMessageFlagsStmt {
    SetMessageFlagsStmt(
        "UPDATE messages SET flags_json = $1, modseq = $2 WHERE account_id = $3 AND msgid = $4",
        None,
    )
}
impl SetMessageFlagsStmt {
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
        flags_json: &'a T1,
        modseq: &'a i64,
        account_id: &'a T2,
        msgid: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[flags_json, modseq, account_id, msgid])
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
        SetMessageFlagsParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetMessageFlagsStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetMessageFlagsParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.flags_json,
            &params.modseq,
            &params.account_id,
            &params.msgid,
        ))
    }
}
pub struct SetMessageModseqStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn set_message_modseq() -> SetMessageModseqStmt {
    SetMessageModseqStmt(
        "UPDATE messages SET modseq = $1 WHERE account_id = $2 AND msgid = $3",
        None,
    )
}
impl SetMessageModseqStmt {
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
        msgid: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[modseq, account_id, msgid]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        SetMessageModseqParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for SetMessageModseqStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a SetMessageModseqParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.modseq, &params.account_id, &params.msgid))
    }
}
pub struct MessageBodyMetadataStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn message_body_metadata() -> MessageBodyMetadataStmt {
    MessageBodyMetadataStmt(
        "SELECT subject, preview, has_attachment FROM messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl MessageBodyMetadataStmt {
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
    ) -> MessageBodyMetadataRowQuery<'c, 'a, 's, C, MessageBodyMetadataRow, 2> {
        MessageBodyMetadataRowQuery {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<MessageBodyMetadataRowBorrowed, tokio_postgres::Error> {
                Ok(MessageBodyMetadataRowBorrowed {
                    subject: row.try_get(0)?,
                    preview: row.try_get(1)?,
                    has_attachment: row.try_get(2)?,
                })
            },
            mapper: |it| MessageBodyMetadataRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        MessageBodyMetadataParams<T1, T2>,
        MessageBodyMetadataRowQuery<'c, 'a, 's, C, MessageBodyMetadataRow, 2>,
        C,
    > for MessageBodyMetadataStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MessageBodyMetadataParams<T1, T2>,
    ) -> MessageBodyMetadataRowQuery<'c, 'a, 's, C, MessageBodyMetadataRow, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct UpdateMessageBodyCacheStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn update_message_body_cache() -> UpdateMessageBodyCacheStmt {
    UpdateMessageBodyCacheStmt(
        "UPDATE messages SET preview = $1, has_attachment = $2, modseq = $3 WHERE account_id = $4 AND msgid = $5",
        None,
    )
}
impl UpdateMessageBodyCacheStmt {
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
        preview: &'a T1,
        has_attachment: &'a i64,
        modseq: &'a i64,
        account_id: &'a T2,
        msgid: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[preview, has_attachment, modseq, account_id, msgid],
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
        UpdateMessageBodyCacheParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpdateMessageBodyCacheStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpdateMessageBodyCacheParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.preview,
            &params.has_attachment,
            &params.modseq,
            &params.account_id,
            &params.msgid,
        ))
    }
}
pub struct RepairMessageBodyMetadataStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn repair_message_body_metadata() -> RepairMessageBodyMetadataStmt {
    RepairMessageBodyMetadataStmt(
        "UPDATE messages SET subject = $1, preview = $2, has_attachment = $3, modseq = $4 WHERE account_id = $5 AND msgid = $6",
        None,
    )
}
impl RepairMessageBodyMetadataStmt {
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
        subject: &'a Option<T1>,
        preview: &'a T2,
        has_attachment: &'a i64,
        modseq: &'a i64,
        account_id: &'a T3,
        msgid: &'a T4,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(
                self.0,
                &[subject, preview, has_attachment, modseq, account_id, msgid],
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
        RepairMessageBodyMetadataParams<T1, T2, T3, T4>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for RepairMessageBodyMetadataStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a RepairMessageBodyMetadataParams<T1, T2, T3, T4>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.subject,
            &params.preview,
            &params.has_attachment,
            &params.modseq,
            &params.account_id,
            &params.msgid,
        ))
    }
}
pub struct MessageAddressesStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn message_addresses() -> MessageAddressesStmt {
    MessageAddressesStmt(
        "SELECT thrid AS thread_id, to_json, cc_json, bcc_json FROM messages WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl MessageAddressesStmt {
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
    ) -> MessageAddressesRowQuery<'c, 'a, 's, C, MessageAddressesRow, 2> {
        MessageAddressesRowQuery {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<MessageAddressesRowBorrowed, tokio_postgres::Error> {
                Ok(MessageAddressesRowBorrowed {
                    thread_id: row.try_get(0)?,
                    to_json: row.try_get(1)?,
                    cc_json: row.try_get(2)?,
                    bcc_json: row.try_get(3)?,
                })
            },
            mapper: |it| MessageAddressesRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        MessageAddressesParams<T1, T2>,
        MessageAddressesRowQuery<'c, 'a, 's, C, MessageAddressesRow, 2>,
        C,
    > for MessageAddressesStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MessageAddressesParams<T1, T2>,
    ) -> MessageAddressesRowQuery<'c, 'a, 's, C, MessageAddressesRow, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct AddMessageMailboxStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn add_message_mailbox() -> AddMessageMailboxStmt {
    AddMessageMailboxStmt(
        "INSERT INTO message_mailboxes (account_id, msgid, mailbox_id) VALUES ($1, $2, $3) ON CONFLICT (account_id, msgid, mailbox_id) DO NOTHING",
        None,
    )
}
impl AddMessageMailboxStmt {
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
        msgid: &'a T2,
        mailbox_id: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[account_id, msgid, mailbox_id])
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
        AddMessageMailboxParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for AddMessageMailboxStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a AddMessageMailboxParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.msgid,
            &params.mailbox_id,
        ))
    }
}
pub struct RemoveMessageMailboxStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn remove_message_mailbox() -> RemoveMessageMailboxStmt {
    RemoveMessageMailboxStmt(
        "DELETE FROM message_mailboxes WHERE account_id = $1 AND msgid = $2 AND mailbox_id = $3",
        None,
    )
}
impl RemoveMessageMailboxStmt {
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
        msgid: &'a T2,
        mailbox_id: &'a T3,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[account_id, msgid, mailbox_id])
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
        RemoveMessageMailboxParams<T1, T2, T3>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for RemoveMessageMailboxStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a RemoveMessageMailboxParams<T1, T2, T3>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.msgid,
            &params.mailbox_id,
        ))
    }
}
pub struct ClearMessageMailboxesStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn clear_message_mailboxes() -> ClearMessageMailboxesStmt {
    ClearMessageMailboxesStmt(
        "DELETE FROM message_mailboxes WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl ClearMessageMailboxesStmt {
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
        msgid: &'a T2,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id, msgid]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        ClearMessageMailboxesParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for ClearMessageMailboxesStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a ClearMessageMailboxesParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.account_id, &params.msgid))
    }
}
pub struct MessageMailboxIdsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn message_mailbox_ids() -> MessageMailboxIdsStmt {
    MessageMailboxIdsStmt(
        "SELECT mailbox_id FROM message_mailboxes WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl MessageMailboxIdsStmt {
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
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        StringQuery {
            client,
            params: [account_id, msgid],
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
        MessageMailboxIdsParams<T1, T2>,
        StringQuery<'c, 'a, 's, C, String, 2>,
        C,
    > for MessageMailboxIdsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MessageMailboxIdsParams<T1, T2>,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct UpsertMessageImapStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn upsert_message_imap() -> UpsertMessageImapStmt {
    UpsertMessageImapStmt(
        "INSERT INTO message_imap (account_id, msgid, folder_id, uid, uidvalidity) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (account_id, msgid, folder_id) DO UPDATE SET uid = EXCLUDED.uid, uidvalidity = EXCLUDED.uidvalidity",
        None,
    )
}
impl UpsertMessageImapStmt {
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
        msgid: &'a T2,
        folder_id: &'a i64,
        uid: &'a i64,
        uidvalidity: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client
            .execute(self.0, &[account_id, msgid, folder_id, uid, uidvalidity])
            .await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        UpsertMessageImapParams<T1, T2>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for UpsertMessageImapStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a UpsertMessageImapParams<T1, T2>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(
            client,
            &params.account_id,
            &params.msgid,
            &params.folder_id,
            &params.uid,
            &params.uidvalidity,
        ))
    }
}
pub struct GetMessageImapInFolderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_message_imap_in_folder() -> GetMessageImapInFolderStmt {
    GetMessageImapInFolderStmt(
        "SELECT folder_id, uid, uidvalidity FROM message_imap WHERE account_id = $1 AND msgid = $2 AND folder_id = $3",
        None,
    )
}
impl GetMessageImapInFolderStmt {
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
        folder_id: &'a i64,
    ) -> MessageImapRowQuery<'c, 'a, 's, C, MessageImapRow, 3> {
        MessageImapRowQuery {
            client,
            params: [account_id, msgid, folder_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<MessageImapRow, tokio_postgres::Error> {
                    Ok(MessageImapRow {
                        folder_id: row.try_get(0)?,
                        uid: row.try_get(1)?,
                        uidvalidity: row.try_get(2)?,
                    })
                },
            mapper: |it| MessageImapRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetMessageImapInFolderParams<T1, T2>,
        MessageImapRowQuery<'c, 'a, 's, C, MessageImapRow, 3>,
        C,
    > for GetMessageImapInFolderStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetMessageImapInFolderParams<T1, T2>,
    ) -> MessageImapRowQuery<'c, 'a, 's, C, MessageImapRow, 3> {
        self.bind(client, &params.account_id, &params.msgid, &params.folder_id)
    }
}
pub struct GetMessageImapAnyStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn get_message_imap_any() -> GetMessageImapAnyStmt {
    GetMessageImapAnyStmt(
        "SELECT folder_id, uid, uidvalidity FROM message_imap WHERE account_id = $1 AND msgid = $2 LIMIT 1",
        None,
    )
}
impl GetMessageImapAnyStmt {
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
    ) -> MessageImapRowQuery<'c, 'a, 's, C, MessageImapRow, 2> {
        MessageImapRowQuery {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor:
                |row: &tokio_postgres::Row| -> Result<MessageImapRow, tokio_postgres::Error> {
                    Ok(MessageImapRow {
                        folder_id: row.try_get(0)?,
                        uid: row.try_get(1)?,
                        uidvalidity: row.try_get(2)?,
                    })
                },
            mapper: |it| MessageImapRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        GetMessageImapAnyParams<T1, T2>,
        MessageImapRowQuery<'c, 'a, 's, C, MessageImapRow, 2>,
        C,
    > for GetMessageImapAnyStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a GetMessageImapAnyParams<T1, T2>,
    ) -> MessageImapRowQuery<'c, 'a, 's, C, MessageImapRow, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct MessageLocationsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn message_locations() -> MessageLocationsStmt {
    MessageLocationsStmt(
        "SELECT f.id AS folder_id, f.imap_name, mi.uid, mi.uidvalidity FROM message_imap mi JOIN folders f ON f.id = mi.folder_id WHERE mi.account_id = $1 AND mi.msgid = $2",
        None,
    )
}
impl MessageLocationsStmt {
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
    ) -> MessageLocationRowQuery<'c, 'a, 's, C, MessageLocationRow, 2> {
        MessageLocationRowQuery {
            client,
            params: [account_id, msgid],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<MessageLocationRowBorrowed, tokio_postgres::Error> {
                Ok(MessageLocationRowBorrowed {
                    folder_id: row.try_get(0)?,
                    imap_name: row.try_get(1)?,
                    uid: row.try_get(2)?,
                    uidvalidity: row.try_get(3)?,
                })
            },
            mapper: |it| MessageLocationRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        MessageLocationsParams<T1, T2>,
        MessageLocationRowQuery<'c, 'a, 's, C, MessageLocationRow, 2>,
        C,
    > for MessageLocationsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MessageLocationsParams<T1, T2>,
    ) -> MessageLocationRowQuery<'c, 'a, 's, C, MessageLocationRow, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
pub struct PreferredMessageLocationsStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn preferred_message_locations() -> PreferredMessageLocationsStmt {
    PreferredMessageLocationsStmt(
        "SELECT DISTINCT ON (mi.msgid) mi.msgid, f.imap_name, mi.uid, mi.uidvalidity FROM message_imap mi JOIN folders f ON f.id = mi.folder_id WHERE mi.account_id = $1 AND mi.msgid = ANY($2) ORDER BY mi.msgid, CASE f.role WHEN 'all' THEN 0 WHEN 'inbox' THEN 1 ELSE 2 END",
        None,
    )
}
impl PreferredMessageLocationsStmt {
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
    ) -> PreferredMessageLocationRowQuery<'c, 'a, 's, C, PreferredMessageLocationRow, 2> {
        PreferredMessageLocationRowQuery {
            client,
            params: [account_id, msgids],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<PreferredMessageLocationRowBorrowed, tokio_postgres::Error> {
                Ok(PreferredMessageLocationRowBorrowed {
                    msgid: row.try_get(0)?,
                    imap_name: row.try_get(1)?,
                    uid: row.try_get(2)?,
                    uidvalidity: row.try_get(3)?,
                })
            },
            mapper: |it| PreferredMessageLocationRow::from(it),
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
        PreferredMessageLocationsParams<T1, T2, T3>,
        PreferredMessageLocationRowQuery<'c, 'a, 's, C, PreferredMessageLocationRow, 2>,
        C,
    > for PreferredMessageLocationsStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a PreferredMessageLocationsParams<T1, T2, T3>,
    ) -> PreferredMessageLocationRowQuery<'c, 'a, 's, C, PreferredMessageLocationRow, 2> {
        self.bind(client, &params.account_id, &params.msgids)
    }
}
pub struct ImportedMessageByHeaderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn imported_message_by_header() -> ImportedMessageByHeaderStmt {
    ImportedMessageByHeaderStmt(
        "SELECT m.msgid, m.thrid FROM messages m JOIN message_imap mi ON mi.account_id = m.account_id AND mi.msgid = m.msgid WHERE m.account_id = $1 AND mi.folder_id = $2 AND m.message_id_header = $3 ORDER BY mi.uid DESC LIMIT 1",
        None,
    )
}
impl ImportedMessageByHeaderStmt {
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
        folder_id: &'a i64,
        message_id_header: &'a T2,
    ) -> ImportedMessageRowQuery<'c, 'a, 's, C, ImportedMessageRow, 3> {
        ImportedMessageRowQuery {
            client,
            params: [account_id, folder_id, message_id_header],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |
                row: &tokio_postgres::Row,
            | -> Result<ImportedMessageRowBorrowed, tokio_postgres::Error> {
                Ok(ImportedMessageRowBorrowed {
                    msgid: row.try_get(0)?,
                    thrid: row.try_get(1)?,
                })
            },
            mapper: |it| ImportedMessageRow::from(it),
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql, T2: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        ImportedMessageByHeaderParams<T1, T2>,
        ImportedMessageRowQuery<'c, 'a, 's, C, ImportedMessageRow, 3>,
        C,
    > for ImportedMessageByHeaderStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a ImportedMessageByHeaderParams<T1, T2>,
    ) -> ImportedMessageRowQuery<'c, 'a, 's, C, ImportedMessageRow, 3> {
        self.bind(
            client,
            &params.account_id,
            &params.folder_id,
            &params.message_id_header,
        )
    }
}
pub struct MsgidsInFolderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn msgids_in_folder() -> MsgidsInFolderStmt {
    MsgidsInFolderStmt(
        "SELECT msgid FROM message_imap WHERE account_id = $1 AND folder_id = $2",
        None,
    )
}
impl MsgidsInFolderStmt {
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
        folder_id: &'a i64,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        StringQuery {
            client,
            params: [account_id, folder_id],
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
        MsgidsInFolderParams<T1>,
        StringQuery<'c, 'a, 's, C, String, 2>,
        C,
    > for MsgidsInFolderStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MsgidsInFolderParams<T1>,
    ) -> StringQuery<'c, 'a, 's, C, String, 2> {
        self.bind(client, &params.account_id, &params.folder_id)
    }
}
pub struct UidsInFolderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn uids_in_folder() -> UidsInFolderStmt {
    UidsInFolderStmt(
        "SELECT uid FROM message_imap WHERE account_id = $1 AND folder_id = $2 ORDER BY uid",
        None,
    )
}
impl UidsInFolderStmt {
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
        folder_id: &'a i64,
    ) -> I64Query<'c, 'a, 's, C, i64, 2> {
        I64Query {
            client,
            params: [account_id, folder_id],
            query: self.0,
            cached: self.1.as_ref(),
            extractor: |row| Ok(row.try_get(0)?),
            mapper: |it| it,
        }
    }
}
impl<'c, 'a, 's, C: GenericClient, T1: crate::StringSql>
    crate::client::async_::Params<
        'c,
        'a,
        's,
        UidsInFolderParams<T1>,
        I64Query<'c, 'a, 's, C, i64, 2>,
        C,
    > for UidsInFolderStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a UidsInFolderParams<T1>,
    ) -> I64Query<'c, 'a, 's, C, i64, 2> {
        self.bind(client, &params.account_id, &params.folder_id)
    }
}
pub struct MsgidForFolderUidStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn msgid_for_folder_uid() -> MsgidForFolderUidStmt {
    MsgidForFolderUidStmt(
        "SELECT msgid FROM message_imap WHERE account_id = $1 AND folder_id = $2 AND uid = $3",
        None,
    )
}
impl MsgidForFolderUidStmt {
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
        folder_id: &'a i64,
        uid: &'a i64,
    ) -> StringQuery<'c, 'a, 's, C, String, 3> {
        StringQuery {
            client,
            params: [account_id, folder_id, uid],
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
        MsgidForFolderUidParams<T1>,
        StringQuery<'c, 'a, 's, C, String, 3>,
        C,
    > for MsgidForFolderUidStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a MsgidForFolderUidParams<T1>,
    ) -> StringQuery<'c, 'a, 's, C, String, 3> {
        self.bind(client, &params.account_id, &params.folder_id, &params.uid)
    }
}
pub struct DeleteMessageImapByUidStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_message_imap_by_uid() -> DeleteMessageImapByUidStmt {
    DeleteMessageImapByUidStmt(
        "DELETE FROM message_imap WHERE account_id = $1 AND folder_id = $2 AND uid = $3",
        None,
    )
}
impl DeleteMessageImapByUidStmt {
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
        folder_id: &'a i64,
        uid: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id, folder_id, uid]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        DeleteMessageImapByUidParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for DeleteMessageImapByUidStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a DeleteMessageImapByUidParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.account_id, &params.folder_id, &params.uid))
    }
}
pub struct DeleteMessageImapForFolderStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn delete_message_imap_for_folder() -> DeleteMessageImapForFolderStmt {
    DeleteMessageImapForFolderStmt(
        "DELETE FROM message_imap WHERE account_id = $1 AND folder_id = $2",
        None,
    )
}
impl DeleteMessageImapForFolderStmt {
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
        folder_id: &'a i64,
    ) -> Result<u64, tokio_postgres::Error> {
        client.execute(self.0, &[account_id, folder_id]).await
    }
}
impl<'a, C: GenericClient + Send + Sync, T1: crate::StringSql>
    crate::client::async_::Params<
        'a,
        'a,
        'a,
        DeleteMessageImapForFolderParams<T1>,
        std::pin::Pin<
            Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
        >,
        C,
    > for DeleteMessageImapForFolderStmt
{
    fn params(
        &'a self,
        client: &'a C,
        params: &'a DeleteMessageImapForFolderParams<T1>,
    ) -> std::pin::Pin<
        Box<dyn futures::Future<Output = Result<u64, tokio_postgres::Error>> + Send + 'a>,
    > {
        Box::pin(self.bind(client, &params.account_id, &params.folder_id))
    }
}
pub struct CountMessageImapStmt(&'static str, Option<tokio_postgres::Statement>);
pub fn count_message_imap() -> CountMessageImapStmt {
    CountMessageImapStmt(
        "SELECT COUNT(*) FROM message_imap WHERE account_id = $1 AND msgid = $2",
        None,
    )
}
impl CountMessageImapStmt {
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
        CountMessageImapParams<T1, T2>,
        I64Query<'c, 'a, 's, C, i64, 2>,
        C,
    > for CountMessageImapStmt
{
    fn params(
        &'s self,
        client: &'c C,
        params: &'a CountMessageImapParams<T1, T2>,
    ) -> I64Query<'c, 'a, 's, C, i64, 2> {
        self.bind(client, &params.account_id, &params.msgid)
    }
}
