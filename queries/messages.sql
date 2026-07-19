--: EnvelopeCmpRow(sent_at?, from_json?, to_json?, cc_json?, bcc_json?, reply_to_json?, subject?, preview?, message_id_header?, in_reply_to_header?, references_header?)

--! get_envelope_for_compare : EnvelopeCmpRow
SELECT thrid, flags_json, received_at, sent_at, size,
       from_json, to_json, cc_json, bcc_json, reply_to_json,
       subject, preview, has_attachment,
       message_id_header, in_reply_to_header, references_header
FROM messages WHERE account_id = :account_id AND msgid = :msgid;

--! upsert_message (sent_at?, from_json?, to_json?, cc_json?, bcc_json?, reply_to_json?, subject?, preview?, message_id_header?, in_reply_to_header?, references_header?)
INSERT INTO messages (
    account_id, msgid, thrid, flags_json, received_at, sent_at, size,
    from_json, to_json, cc_json, bcc_json, reply_to_json,
    subject, preview, has_attachment,
    message_id_header, in_reply_to_header, references_header,
    modseq
) VALUES (
    :account_id, :msgid, :thrid, :flags_json, :received_at, :sent_at, :size,
    :from_json, :to_json, :cc_json, :bcc_json, :reply_to_json,
    :subject, :preview, :has_attachment,
    :message_id_header, :in_reply_to_header, :references_header,
    :modseq
)
ON CONFLICT (account_id, msgid) DO UPDATE SET
    thrid = EXCLUDED.thrid,
    flags_json = EXCLUDED.flags_json,
    received_at = EXCLUDED.received_at,
    sent_at = EXCLUDED.sent_at,
    size = EXCLUDED.size,
    from_json = EXCLUDED.from_json,
    to_json = EXCLUDED.to_json,
    cc_json = EXCLUDED.cc_json,
    bcc_json = EXCLUDED.bcc_json,
    reply_to_json = EXCLUDED.reply_to_json,
    subject = EXCLUDED.subject,
    preview = EXCLUDED.preview,
    has_attachment = EXCLUDED.has_attachment,
    message_id_header = EXCLUDED.message_id_header,
    in_reply_to_header = EXCLUDED.in_reply_to_header,
    references_header = EXCLUDED.references_header,
    modseq = EXCLUDED.modseq;

--! delete_message
DELETE FROM messages WHERE account_id = :account_id AND msgid = :msgid;

--! message_flags_json
SELECT flags_json FROM messages WHERE account_id = :account_id AND msgid = :msgid;

--! set_message_flags
UPDATE messages SET flags_json = :flags_json, modseq = :modseq
WHERE account_id = :account_id AND msgid = :msgid;

--! set_message_modseq
UPDATE messages SET modseq = :modseq WHERE account_id = :account_id AND msgid = :msgid;

--: MessageBodyMetadataRow(subject?, preview?)

--! message_body_metadata : MessageBodyMetadataRow
SELECT subject, preview, has_attachment
FROM messages WHERE account_id = :account_id AND msgid = :msgid;

--! update_message_body_cache
UPDATE messages
SET preview = :preview, has_attachment = :has_attachment, modseq = :modseq
WHERE account_id = :account_id AND msgid = :msgid;

--! repair_message_body_metadata (subject?)
UPDATE messages
SET subject = :subject, preview = :preview, has_attachment = :has_attachment, modseq = :modseq
WHERE account_id = :account_id AND msgid = :msgid;

--: MessageAddressesRow(to_json?, cc_json?, bcc_json?)

--! message_addresses : MessageAddressesRow
SELECT thrid AS thread_id, to_json, cc_json, bcc_json
FROM messages WHERE account_id = :account_id AND msgid = :msgid;

--! add_message_mailbox
INSERT INTO message_mailboxes (account_id, msgid, mailbox_id)
VALUES (:account_id, :msgid, :mailbox_id)
ON CONFLICT (account_id, msgid, mailbox_id) DO NOTHING;

--! remove_message_mailbox
DELETE FROM message_mailboxes
WHERE account_id = :account_id AND msgid = :msgid AND mailbox_id = :mailbox_id;

--! clear_message_mailboxes
DELETE FROM message_mailboxes WHERE account_id = :account_id AND msgid = :msgid;

--! message_mailbox_ids
SELECT mailbox_id FROM message_mailboxes WHERE account_id = :account_id AND msgid = :msgid;

--! upsert_message_imap
INSERT INTO message_imap (account_id, msgid, folder_id, uid, uidvalidity)
VALUES (:account_id, :msgid, :folder_id, :uid, :uidvalidity)
ON CONFLICT (account_id, msgid, folder_id) DO UPDATE SET
    uid = EXCLUDED.uid,
    uidvalidity = EXCLUDED.uidvalidity;

--: MessageImapRow()

--! get_message_imap_in_folder : MessageImapRow
SELECT folder_id, uid, uidvalidity FROM message_imap
WHERE account_id = :account_id AND msgid = :msgid AND folder_id = :folder_id;

--! get_message_imap_any : MessageImapRow
SELECT folder_id, uid, uidvalidity FROM message_imap
WHERE account_id = :account_id AND msgid = :msgid LIMIT 1;

--: MessageLocationRow()

--! message_locations : MessageLocationRow
SELECT f.id AS folder_id, f.imap_name, mi.uid, mi.uidvalidity
FROM message_imap mi JOIN folders f ON f.id = mi.folder_id
WHERE mi.account_id = :account_id AND mi.msgid = :msgid;

--: PreferredMessageLocationRow()

--! preferred_message_locations : PreferredMessageLocationRow
SELECT DISTINCT ON (mi.msgid) mi.msgid, f.imap_name, mi.uid, mi.uidvalidity
FROM message_imap mi
JOIN folders f ON f.id = mi.folder_id
WHERE mi.account_id = :account_id AND mi.msgid = ANY(:msgids)
ORDER BY mi.msgid, CASE f.role WHEN 'all' THEN 0 WHEN 'inbox' THEN 1 ELSE 2 END;

--: ImportedMessageRow()

--! imported_message_by_header : ImportedMessageRow
SELECT m.msgid, m.thrid
FROM messages m
JOIN message_imap mi ON mi.account_id = m.account_id AND mi.msgid = m.msgid
WHERE m.account_id = :account_id
  AND mi.folder_id = :folder_id
  AND m.message_id_header = :message_id_header
ORDER BY mi.uid DESC
LIMIT 1;

--! msgids_in_folder
SELECT msgid FROM message_imap WHERE account_id = :account_id AND folder_id = :folder_id;

--! uids_in_folder
SELECT uid FROM message_imap
WHERE account_id = :account_id AND folder_id = :folder_id ORDER BY uid;

--! msgid_for_folder_uid
SELECT msgid FROM message_imap
WHERE account_id = :account_id AND folder_id = :folder_id AND uid = :uid;

--! delete_message_imap_by_uid
DELETE FROM message_imap
WHERE account_id = :account_id AND folder_id = :folder_id AND uid = :uid;

--! delete_message_imap_for_folder
DELETE FROM message_imap WHERE account_id = :account_id AND folder_id = :folder_id;

--! count_message_imap
SELECT COUNT(*) FROM message_imap WHERE account_id = :account_id AND msgid = :msgid;
