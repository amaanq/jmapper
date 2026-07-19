--: FolderRow(role?, last_sync_at?)
--: FolderSyncStateRow()

--! upsert_folder (role?)
INSERT INTO folders (
    account_id, imap_name, uidvalidity, uidnext, highestmodseq, role, mailbox_id
)
VALUES (
    :account_id, :imap_name, :uidvalidity, :uidnext, :highestmodseq, :role, :mailbox_id
)
ON CONFLICT (account_id, imap_name) DO UPDATE SET
    uidvalidity = EXCLUDED.uidvalidity,
    uidnext = EXCLUDED.uidnext,
    highestmodseq = EXCLUDED.highestmodseq,
    role = EXCLUDED.role
RETURNING id;

--! list_folders : FolderRow
SELECT id, account_id, imap_name, uidvalidity, uidnext, uidfirst, highestmodseq,
       role, last_sync_at, mailbox_id
FROM folders WHERE account_id = :account_id ORDER BY id;

--! folder_sync_state : FolderSyncStateRow
SELECT uidfirst, mailbox_id FROM folders
WHERE account_id = :account_id AND imap_name = :imap_name;

--! folder_by_name
SELECT id, uidvalidity, uidnext, uidfirst, mailbox_id FROM folders
WHERE account_id = :account_id AND imap_name = :imap_name;

--! set_folder_uidfirst
UPDATE folders SET uidfirst = :uidfirst WHERE id = :id;

--! set_folder_uidnext
UPDATE folders SET uidnext = :uidnext WHERE id = :id;

--! rename_folder
UPDATE folders SET imap_name = :imap_name WHERE id = :id AND account_id = :account_id;

--! delete_folder
DELETE FROM folders WHERE id = :id AND account_id = :account_id;

--! reset_folder_sync_state
UPDATE folders SET uidvalidity = 0, uidnext = 0, uidfirst = 0, highestmodseq = 0
WHERE id = :id AND account_id = :account_id;

--: FolderChildRow()

--! folder_children : FolderChildRow
SELECT id, imap_name, mailbox_id FROM folders
WHERE account_id = :account_id AND left(imap_name, length(:prefix)) = :prefix;
