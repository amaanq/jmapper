-- CTEs share a snapshot, so exclude losers when checking for another folder.
WITH ranked AS (
    SELECT account_id, msgid, folder_id,
           row_number() OVER (
               PARTITION BY folder_id, uid
               ORDER BY (msgid LIKE 'fb\_%'), msgid
           ) AS rn
    FROM message_imap
),
losers AS (
    SELECT account_id, msgid, folder_id FROM ranked WHERE rn > 1
),
dropped AS (
    DELETE FROM message_imap mi
    USING losers l
    WHERE mi.account_id = l.account_id
      AND mi.msgid = l.msgid
      AND mi.folder_id = l.folder_id
    RETURNING mi.account_id, mi.msgid
)
DELETE FROM messages m
USING (SELECT DISTINCT account_id, msgid FROM dropped) d
WHERE m.account_id = d.account_id
  AND m.msgid = d.msgid
  AND NOT EXISTS (
      SELECT 1 FROM message_imap mi
      LEFT JOIN losers l
        ON l.account_id = mi.account_id
       AND l.msgid = mi.msgid
       AND l.folder_id = mi.folder_id
      WHERE mi.account_id = m.account_id
        AND mi.msgid = m.msgid
        AND l.msgid IS NULL
  );

CREATE UNIQUE INDEX IF NOT EXISTS message_imap_folder_uid_key
    ON message_imap (folder_id, uid);
