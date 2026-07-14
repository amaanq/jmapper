--! raw_message_fetched_at
SELECT fetched_at FROM raw_messages WHERE account_id = :account_id AND msgid = :msgid;

--: RawBodyRow()

--! raw_message_projection : RawBodyRow
SELECT headers_json, body_values_json, attachments_json
FROM raw_messages WHERE account_id = :account_id AND msgid = :msgid;

--: RawBytesRow()

--! raw_message_bytes : RawBytesRow
SELECT raw_rfc822 FROM raw_messages WHERE account_id = :account_id AND msgid = :msgid;

--! cached_message_ids
SELECT msgid FROM raw_messages
WHERE account_id = :account_id AND msgid = ANY(:msgids);

--! recent_uncached_message_ids
SELECT m.msgid
FROM messages m
LEFT JOIN raw_messages r
  ON r.account_id = m.account_id AND r.msgid = m.msgid
WHERE m.account_id = :account_id
  AND r.msgid IS NULL
  AND m.size <= :max_bytes
ORDER BY m.received_at DESC
LIMIT :limit;

--! upsert_raw_message
INSERT INTO raw_messages
    (account_id, msgid, headers_json, body_values_json, attachments_json, raw_rfc822, fetched_at)
VALUES (:account_id, :msgid, :headers_json, :body_values_json, :attachments_json, :raw_rfc822,
        EXTRACT(EPOCH FROM now())::bigint)
ON CONFLICT (account_id, msgid) DO UPDATE SET
    headers_json = EXCLUDED.headers_json,
    body_values_json = EXCLUDED.body_values_json,
    attachments_json = EXCLUDED.attachments_json,
    raw_rfc822 = EXCLUDED.raw_rfc822,
    fetched_at = EXCLUDED.fetched_at;

--: CachedMetadataRepairRow(subject?)

--! cached_metadata_repair_candidates : CachedMetadataRepairRow
SELECT r.msgid, m.subject, r.raw_rfc822
FROM raw_messages r
JOIN messages m ON m.account_id = r.account_id AND m.msgid = r.msgid
WHERE r.account_id = :account_id
  AND (
    m.preview IS NULL
    OR m.has_attachment <> CASE WHEN r.attachments_json = '[]' THEN 0 ELSE 1 END
    OR m.subject LIKE '%=?%?=%'
  );
