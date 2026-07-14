--: DavAccountRow(auth_user?, auth_secret?, principal_href?, home_href?, last_sync_at?, last_sync_error?)

--! upsert_dav_account (auth_user?, auth_secret?)
INSERT INTO dav_accounts (account_id, kind, base_url, auth_kind, auth_user, auth_secret)
VALUES (:account_id, :kind, :base_url, :auth_kind, :auth_user, :auth_secret)
ON CONFLICT (account_id, kind) DO UPDATE SET
    base_url = EXCLUDED.base_url,
    auth_kind = EXCLUDED.auth_kind,
    auth_user = EXCLUDED.auth_user,
    auth_secret = EXCLUDED.auth_secret,
    principal_href = CASE
        WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL
        ELSE dav_accounts.principal_href
    END,
    home_href = CASE
        WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL
        ELSE dav_accounts.home_href
    END,
    last_sync_at = CASE
        WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL
        ELSE dav_accounts.last_sync_at
    END,
    last_sync_error = CASE
        WHEN dav_accounts.base_url IS DISTINCT FROM EXCLUDED.base_url THEN NULL
        ELSE dav_accounts.last_sync_error
    END;

--! get_dav_account : DavAccountRow
SELECT account_id, kind, base_url, auth_kind, auth_user, auth_secret,
       principal_href, home_href, last_sync_at, last_sync_error
FROM dav_accounts WHERE account_id = :account_id AND kind = :kind;

--! list_dav_accounts : DavAccountRow
SELECT account_id, kind, base_url, auth_kind, auth_user, auth_secret,
       principal_href, home_href, last_sync_at, last_sync_error
FROM dav_accounts ORDER BY account_id, kind;

--! set_dav_discovery
UPDATE dav_accounts SET principal_href = :principal_href, home_href = :home_href
WHERE account_id = :account_id AND kind = :kind;

--! set_dav_sync_ok
UPDATE dav_accounts SET last_sync_at = :last_sync_at, last_sync_error = NULL
WHERE account_id = :account_id AND kind = :kind;

--! set_dav_sync_error
UPDATE dav_accounts SET last_sync_error = :last_sync_error
WHERE account_id = :account_id AND kind = :kind;

--! delete_dav_account
DELETE FROM dav_accounts WHERE account_id = :account_id AND kind = :kind;

--! ensure_dav_state
INSERT INTO dav_state (account_id) VALUES (:account_id)
ON CONFLICT (account_id) DO NOTHING;

--: DavStateRow()

--! get_dav_state : DavStateRow
SELECT calendar_modseq, calendar_event_modseq, addressbook_modseq, contact_card_modseq
FROM dav_state WHERE account_id = :account_id;

--! bump_calendar_modseq
UPDATE dav_state SET calendar_modseq = calendar_modseq + 1
WHERE account_id = :account_id RETURNING calendar_modseq;

--! bump_calendar_event_modseq
UPDATE dav_state SET calendar_event_modseq = calendar_event_modseq + 1
WHERE account_id = :account_id RETURNING calendar_event_modseq;

--! bump_addressbook_modseq
UPDATE dav_state SET addressbook_modseq = addressbook_modseq + 1
WHERE account_id = :account_id RETURNING addressbook_modseq;

--! bump_contact_card_modseq
UPDATE dav_state SET contact_card_modseq = contact_card_modseq + 1
WHERE account_id = :account_id RETURNING contact_card_modseq;

--: DavCollectionRow(color?, description?, sync_token?)

--! upsert_dav_collection (color?, description?, sync_token?)
INSERT INTO dav_collections
    (account_id, id, kind, href, name, color, description, sync_token,
     supports_sync, created_modseq, modseq)
VALUES
    (:account_id, :id, :kind, :href, :name, :color, :description, :sync_token,
     :supports_sync, :modseq, :modseq)
ON CONFLICT (account_id, id) DO UPDATE SET
    href = EXCLUDED.href,
    name = EXCLUDED.name,
    color = EXCLUDED.color,
    description = EXCLUDED.description,
    sync_token = EXCLUDED.sync_token,
    supports_sync = EXCLUDED.supports_sync,
    created_modseq = CASE
        WHEN dav_collections.destroyed <> 0 THEN EXCLUDED.created_modseq
        ELSE dav_collections.created_modseq
    END,
    modseq = EXCLUDED.modseq,
    destroyed = 0;

--! get_dav_collection : DavCollectionRow
SELECT account_id, id, kind, href, name, color, description, sync_token,
       supports_sync, created_modseq, modseq, destroyed
FROM dav_collections WHERE account_id = :account_id AND id = :id;

--! get_dav_collection_by_href : DavCollectionRow
SELECT account_id, id, kind, href, name, color, description, sync_token,
       supports_sync, created_modseq, modseq, destroyed
FROM dav_collections
WHERE account_id = :account_id AND kind = :kind AND href = :href AND destroyed = 0;

--! list_dav_collections : DavCollectionRow
SELECT account_id, id, kind, href, name, color, description, sync_token,
       supports_sync, created_modseq, modseq, destroyed
FROM dav_collections
WHERE account_id = :account_id AND kind = :kind AND destroyed = 0
ORDER BY id;

--! set_dav_collection_sync_token (sync_token?)
UPDATE dav_collections SET sync_token = :sync_token
WHERE account_id = :account_id AND id = :id;

--! tombstone_dav_collection
UPDATE dav_collections SET destroyed = 1, modseq = :modseq
WHERE account_id = :account_id AND id = :id AND destroyed = 0;

--! dav_collections_changed_since : DavCollectionRow
SELECT account_id, id, kind, href, name, color, description, sync_token,
       supports_sync, created_modseq, modseq, destroyed
FROM dav_collections
WHERE account_id = :account_id AND kind = :kind AND modseq > :modseq
ORDER BY modseq;

--: DavResourceRow(etag?)

--! upsert_dav_resource (etag?)
INSERT INTO dav_resources
    (account_id, id, collection_id, kind, href, etag, uid, raw, json,
     created_modseq, modseq)
VALUES
    (:account_id, :id, :collection_id, :kind, :href, :etag, :uid, :raw, :json,
     :modseq, :modseq)
ON CONFLICT (account_id, id) DO UPDATE SET
    collection_id = EXCLUDED.collection_id,
    href = EXCLUDED.href,
    etag = EXCLUDED.etag,
    uid = EXCLUDED.uid,
    raw = EXCLUDED.raw,
    json = EXCLUDED.json,
    created_modseq = CASE
        WHEN dav_resources.destroyed <> 0 THEN EXCLUDED.created_modseq
        ELSE dav_resources.created_modseq
    END,
    modseq = EXCLUDED.modseq,
    destroyed = 0;

--! get_dav_resource : DavResourceRow
SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json,
       created_modseq, modseq, destroyed
FROM dav_resources WHERE account_id = :account_id AND id = :id;

--! get_dav_resource_by_href : DavResourceRow
SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json,
       created_modseq, modseq, destroyed
FROM dav_resources
WHERE account_id = :account_id AND kind = :kind AND href = :href AND destroyed = 0;

--! list_dav_resources : DavResourceRow
SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json,
       created_modseq, modseq, destroyed
FROM dav_resources
WHERE account_id = :account_id AND collection_id = :collection_id AND destroyed = 0
ORDER BY id;

--! list_dav_resources_by_kind : DavResourceRow
SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json,
       created_modseq, modseq, destroyed
FROM dav_resources
WHERE account_id = :account_id AND kind = :kind AND destroyed = 0
ORDER BY id;

--: DavResourceEtagRow(etag?)

--! list_dav_resource_etags : DavResourceEtagRow
SELECT id, href, etag
FROM dav_resources
WHERE account_id = :account_id AND collection_id = :collection_id AND destroyed = 0
ORDER BY href;

--! tombstone_dav_resource
UPDATE dav_resources SET destroyed = 1, modseq = :modseq
WHERE account_id = :account_id AND id = :id AND destroyed = 0;

--! tombstone_dav_resources_in_collection
UPDATE dav_resources SET destroyed = 1, modseq = :modseq
WHERE account_id = :account_id AND collection_id = :collection_id AND destroyed = 0;

--! dav_resources_changed_since : DavResourceRow
SELECT account_id, id, collection_id, kind, href, etag, uid, raw, json,
       created_modseq, modseq, destroyed
FROM dav_resources
WHERE account_id = :account_id AND kind = :kind AND modseq > :modseq
ORDER BY modseq;

--! upsert_dav_query_snapshot
INSERT INTO dav_query_snapshots
    (account_id, kind, query_hash, modseq, ids_json, created_at, expires_at)
VALUES
    (:account_id, :kind, :query_hash, :modseq, :ids_json, :created_at, :expires_at)
ON CONFLICT (account_id, kind, query_hash, modseq) DO UPDATE SET
    ids_json = EXCLUDED.ids_json,
    created_at = EXCLUDED.created_at,
    expires_at = EXCLUDED.expires_at;

--: DavQuerySnapshotRow()

--! get_dav_query_snapshot : DavQuerySnapshotRow
SELECT modseq, ids_json, created_at, expires_at
FROM dav_query_snapshots
WHERE account_id = :account_id AND kind = :kind AND query_hash = :query_hash
  AND modseq = :modseq
  AND expires_at > :now;

--! delete_expired_dav_query_snapshots
DELETE FROM dav_query_snapshots WHERE expires_at <= :now;
