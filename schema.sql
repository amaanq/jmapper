-- PostgreSQL schema. Timestamps stay BIGINT unix
-- seconds (not timestamptz): every reader converts through chrono at the
-- edges already, and the sync loop compares raw IMAP INTERNALDATE seconds.

CREATE TABLE accounts (
    id                TEXT   PRIMARY KEY,
    email             TEXT   NOT NULL,
    provider          TEXT   NOT NULL,
    display_name      TEXT   NOT NULL,
    bearer_token_hash BYTEA  NOT NULL,
    created_at        BIGINT NOT NULL
);

CREATE TABLE oauth_tokens (
    account_id    TEXT   PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    access_token  TEXT,
    refresh_token TEXT   NOT NULL,
    expires_at    BIGINT
);

CREATE TABLE mailboxes (
    id            TEXT   PRIMARY KEY,
    account_id    TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    name          TEXT   NOT NULL,
    parent_id     TEXT,
    role          TEXT,
    total_emails  BIGINT NOT NULL DEFAULT 0,
    unread_emails BIGINT NOT NULL DEFAULT 0,
    total_threads  BIGINT NOT NULL DEFAULT 0,
    unread_threads BIGINT NOT NULL DEFAULT 0,
    sort_order    BIGINT NOT NULL DEFAULT 0,
    modseq        BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_mailboxes_account ON mailboxes(account_id);
CREATE INDEX idx_mailboxes_modseq  ON mailboxes(account_id, modseq);

CREATE TABLE folders (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    account_id    TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    imap_name     TEXT   NOT NULL,
    uidvalidity   BIGINT NOT NULL DEFAULT 0,
    uidnext       BIGINT NOT NULL DEFAULT 0,
    uidfirst      BIGINT NOT NULL DEFAULT 0,
    highestmodseq BIGINT NOT NULL DEFAULT 0,
    role          TEXT,
    last_sync_at  BIGINT,
    mailbox_id    TEXT   NOT NULL REFERENCES mailboxes(id) ON DELETE CASCADE,
    UNIQUE (account_id, imap_name)
);

CREATE TABLE messages (
    account_id         TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    msgid              TEXT   NOT NULL,
    thrid              TEXT   NOT NULL,
    flags_json         TEXT   NOT NULL DEFAULT '[]',
    received_at        BIGINT NOT NULL,
    sent_at            BIGINT,
    size               BIGINT NOT NULL,
    from_json          TEXT,
    to_json            TEXT,
    cc_json            TEXT,
    bcc_json           TEXT,
    reply_to_json      TEXT,
    subject            TEXT,
    preview            TEXT,
    has_attachment     BIGINT NOT NULL DEFAULT 0,
    message_id_header  TEXT,
    in_reply_to_header TEXT,
    references_header  TEXT,
    modseq             BIGINT NOT NULL,
    PRIMARY KEY (account_id, msgid)
);

CREATE INDEX idx_messages_account_recv ON messages(account_id, received_at DESC);
CREATE INDEX idx_messages_account_mod  ON messages(account_id, modseq);
CREATE INDEX idx_messages_thrid        ON messages(account_id, thrid);

CREATE TABLE message_mailboxes (
    account_id TEXT NOT NULL,
    msgid      TEXT NOT NULL,
    mailbox_id TEXT NOT NULL REFERENCES mailboxes(id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, msgid, mailbox_id),
    FOREIGN KEY (account_id, msgid) REFERENCES messages(account_id, msgid) ON DELETE CASCADE
);

CREATE INDEX idx_mm_mailbox ON message_mailboxes(mailbox_id);

CREATE TABLE message_imap (
    account_id  TEXT   NOT NULL,
    msgid       TEXT   NOT NULL,
    folder_id   BIGINT NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    uid         BIGINT NOT NULL,
    uidvalidity BIGINT NOT NULL,
    PRIMARY KEY (account_id, msgid, folder_id),
    FOREIGN KEY (account_id, msgid) REFERENCES messages(account_id, msgid) ON DELETE CASCADE
);

CREATE INDEX idx_mi_folder_uid ON message_imap(folder_id, uid);

CREATE TABLE raw_messages (
    account_id       TEXT   NOT NULL,
    msgid            TEXT   NOT NULL,
    headers_json     TEXT   NOT NULL,
    body_values_json TEXT   NOT NULL,
    attachments_json TEXT   NOT NULL,
    raw_rfc822       BYTEA  NOT NULL,
    fetched_at       BIGINT NOT NULL,
    PRIMARY KEY (account_id, msgid),
    FOREIGN KEY (account_id, msgid) REFERENCES messages(account_id, msgid) ON DELETE CASCADE
);

CREATE TABLE state (
    account_id        TEXT   PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    email_modseq      BIGINT NOT NULL DEFAULT 0,
    mailbox_modseq    BIGINT NOT NULL DEFAULT 0,
    submission_modseq BIGINT NOT NULL DEFAULT 0,
    initial_sync_done BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE thread_index (
    account_id TEXT NOT NULL,
    message_id TEXT NOT NULL,
    thrid      TEXT NOT NULL,
    PRIMARY KEY (account_id, message_id)
);

CREATE INDEX idx_thread_index_thrid ON thread_index(account_id, thrid);

CREATE TABLE thread_by_subject (
    account_id       TEXT   NOT NULL,
    norm_subject     TEXT   NOT NULL,
    thrid            TEXT   NOT NULL,
    last_received_at BIGINT NOT NULL,
    PRIMARY KEY (account_id, norm_subject)
);

CREATE TABLE uploaded_blobs (
    account_id   TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    blob_id      TEXT   NOT NULL,
    content_type TEXT   NOT NULL,
    bytes        BYTEA  NOT NULL,
    uploaded_at  BIGINT NOT NULL,
    expires_at   BIGINT NOT NULL,
    PRIMARY KEY (account_id, blob_id)
);

CREATE INDEX idx_uploaded_blobs_expiry ON uploaded_blobs(expires_at);

CREATE TABLE email_submissions (
    account_id           TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    id                   TEXT   NOT NULL,
    email_id             TEXT   NOT NULL,
    identity_id          TEXT   NOT NULL,
    thread_id            TEXT,
    envelope_json        TEXT   NOT NULL,
    send_at              BIGINT NOT NULL,
    undo_status          TEXT   NOT NULL DEFAULT 'final',
    attempts             BIGINT NOT NULL DEFAULT 0,
    destroyed            BIGINT NOT NULL DEFAULT 0,
    -- Delayed sends stage their own copy: the standard client flow destroys
    -- the draft via onSuccessDestroyEmail, which CASCADEs the raw_messages
    -- row the scheduler would otherwise read at send time.
    raw_rfc822           BYTEA,
    delivery_status_json TEXT,
    modseq               BIGINT NOT NULL,
    PRIMARY KEY (account_id, id)
);

CREATE INDEX idx_submissions_modseq ON email_submissions(account_id, modseq);
CREATE INDEX idx_submissions_pending
    ON email_submissions(undo_status, send_at) WHERE undo_status = 'pending';

-- ==== CalDAV / CardDAV cache (dav-sync) ====

-- One row per account per protocol endpoint. Discovery results are cached
-- so restarts skip the principal/home-set PROPFINDs; last_sync_* is the
-- per-endpoint health signal surfaced to operators.
CREATE TABLE dav_accounts (
    account_id      TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    kind            TEXT   NOT NULL CHECK (kind IN ('caldav', 'carddav')),
    base_url        TEXT   NOT NULL,
    auth_kind       TEXT   NOT NULL CHECK (auth_kind IN ('basic', 'bearer', 'none')),
    auth_user       TEXT,
    auth_secret     TEXT,
    principal_href  TEXT,
    home_href       TEXT,
    last_sync_at    BIGINT,
    last_sync_error TEXT,
    PRIMARY KEY (account_id, kind)
);

-- Independent JMAP state streams: Calendar, CalendarEvent, AddressBook,
-- ContactCard each advance on their own so one object type churning does
-- not invalidate clients of the others.
CREATE TABLE dav_state (
    account_id            TEXT   PRIMARY KEY REFERENCES accounts(id) ON DELETE CASCADE,
    calendar_modseq       BIGINT NOT NULL DEFAULT 0,
    calendar_event_modseq BIGINT NOT NULL DEFAULT 0,
    addressbook_modseq    BIGINT NOT NULL DEFAULT 0,
    contact_card_modseq   BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE dav_collections (
    account_id    TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    id            TEXT   NOT NULL,
    kind          TEXT   NOT NULL CHECK (kind IN ('calendar', 'addressbook')),
    href          TEXT   NOT NULL,
    name          TEXT   NOT NULL,
    color         TEXT,
    description   TEXT,
    -- RFC 6578 token from the last completed sync; NULL forces a full
    -- (initial or fallback) listing on the next run.
    sync_token    TEXT,
    supports_sync BIGINT NOT NULL DEFAULT 1 CHECK (supports_sync IN (0, 1)),
    created_modseq BIGINT NOT NULL,
    modseq        BIGINT NOT NULL,
    destroyed     BIGINT NOT NULL DEFAULT 0 CHECK (destroyed IN (0, 1)),
    PRIMARY KEY (account_id, id)
);

CREATE UNIQUE INDEX idx_dav_collections_href
    ON dav_collections(account_id, kind, href) WHERE destroyed = 0;
CREATE INDEX idx_dav_collections_modseq ON dav_collections(account_id, kind, modseq);

CREATE TABLE dav_resources (
    account_id    TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    id            TEXT   NOT NULL,
    collection_id TEXT   NOT NULL,
    kind          TEXT   NOT NULL CHECK (kind IN ('event', 'card')),
    href          TEXT   NOT NULL,
    etag          TEXT,
    uid           TEXT   NOT NULL,
    raw           TEXT   NOT NULL, -- iCalendar / vCard payload as fetched
    json          TEXT   NOT NULL, -- normalized JSCalendar / JSContact
    created_modseq BIGINT NOT NULL,
    modseq        BIGINT NOT NULL,
    destroyed     BIGINT NOT NULL DEFAULT 0 CHECK (destroyed IN (0, 1)),
    PRIMARY KEY (account_id, id),
    FOREIGN KEY (account_id, collection_id)
        REFERENCES dav_collections(account_id, id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX idx_dav_resources_href
    ON dav_resources(account_id, kind, href) WHERE destroyed = 0;
CREATE UNIQUE INDEX idx_dav_resources_uid
    ON dav_resources(account_id, kind, uid) WHERE destroyed = 0;
CREATE INDEX idx_dav_resources_collection
    ON dav_resources(account_id, collection_id) WHERE destroyed = 0;
CREATE INDEX idx_dav_resources_modseq ON dav_resources(account_id, kind, modseq);

-- Query snapshots for future queryChanges: the id list a query returned at
-- a given state, so later calls can diff instead of recompute.
CREATE TABLE dav_query_snapshots (
    account_id TEXT   NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    kind       TEXT   NOT NULL,
    query_hash TEXT   NOT NULL,
    modseq     BIGINT NOT NULL,
    ids_json   TEXT   NOT NULL,
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    PRIMARY KEY (account_id, kind, query_hash, modseq)
);

CREATE INDEX idx_dav_query_snapshots_expiry ON dav_query_snapshots(expires_at);
