--: MailboxRow(parent_id?, role?)
--: MailboxMetadataRow(parent_id?, role?)

--! upsert_mailbox (parent_id?, role?)
INSERT INTO mailboxes (id, account_id, name, parent_id, role, sort_order, modseq)
VALUES (:id, :account_id, :name, :parent_id, :role, :sort_order, :modseq)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    parent_id = EXCLUDED.parent_id,
    role = EXCLUDED.role,
    sort_order = EXCLUDED.sort_order,
    modseq = EXCLUDED.modseq;

--! mailbox_metadata : MailboxMetadataRow
SELECT name, parent_id, role, sort_order FROM mailboxes
WHERE id = :id AND account_id = :account_id;

--! list_mailboxes : MailboxRow
SELECT id, account_id, name, parent_id, role,
       total_emails, unread_emails, total_threads, unread_threads,
       sort_order, modseq
FROM mailboxes WHERE account_id = :account_id ORDER BY sort_order, name;

--! get_mailboxes_by_ids : MailboxRow
SELECT id, account_id, name, parent_id, role,
       total_emails, unread_emails, total_threads, unread_threads,
       sort_order, modseq
FROM mailboxes WHERE account_id = :account_id AND id = ANY(:ids);

--! set_mailbox_name
UPDATE mailboxes SET name = :name, modseq = :modseq
WHERE id = :id AND account_id = :account_id;

--! delete_mailbox
DELETE FROM mailboxes WHERE id = :id AND account_id = :account_id;

--! recompute_mailbox_counts
WITH counts AS (
    SELECT m.id,
           COUNT(mm.msgid) AS total_emails,
           COUNT(mm.msgid) FILTER (
               WHERE strpos(msg.flags_json, '"$seen"') = 0
           ) AS unread_emails,
           COUNT(DISTINCT msg.thrid) AS total_threads,
           COUNT(DISTINCT msg.thrid) FILTER (
               WHERE strpos(msg.flags_json, '"$seen"') = 0
           ) AS unread_threads
    FROM mailboxes m
    LEFT JOIN message_mailboxes mm
      ON mm.account_id = m.account_id AND mm.mailbox_id = m.id
    LEFT JOIN messages msg
      ON msg.account_id = mm.account_id AND msg.msgid = mm.msgid
    WHERE m.account_id = :account_id
    GROUP BY m.id
), changed AS (
    SELECT counts.* FROM counts
    JOIN mailboxes m ON m.id = counts.id
    WHERE (m.total_emails, m.unread_emails, m.total_threads, m.unread_threads)
       IS DISTINCT FROM
          (counts.total_emails, counts.unread_emails, counts.total_threads, counts.unread_threads)
), new_state AS (
    UPDATE state SET mailbox_modseq = mailbox_modseq + 1
    WHERE account_id = :account_id AND EXISTS (SELECT 1 FROM changed)
    RETURNING mailbox_modseq
)
UPDATE mailboxes m SET
    total_emails = changed.total_emails,
    unread_emails = changed.unread_emails,
    total_threads = changed.total_threads,
    unread_threads = changed.unread_threads,
    modseq = (SELECT mailbox_modseq FROM new_state)
FROM changed
WHERE m.id = changed.id;

--: ResolvedMailboxFolder(role?)

--! resolve_mailbox_folders : ResolvedMailboxFolder
SELECT f.id, f.imap_name, m.id AS mailbox_id, m.role FROM folders f
JOIN mailboxes m ON m.account_id = f.account_id AND m.id = f.mailbox_id
WHERE f.account_id = :account_id AND m.id = ANY(:mailbox_ids);
