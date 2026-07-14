--! mark_initial_sync_done
UPDATE state SET initial_sync_done = 1 WHERE account_id = :account_id;

--! bump_email_modseq
UPDATE state SET email_modseq = email_modseq + 1
WHERE account_id = :account_id RETURNING email_modseq;

--! bump_mailbox_modseq
UPDATE state SET mailbox_modseq = mailbox_modseq + 1
WHERE account_id = :account_id RETURNING mailbox_modseq;

--! bump_submission_modseq
UPDATE state SET submission_modseq = submission_modseq + 1
WHERE account_id = :account_id RETURNING submission_modseq;

--: StateRow()

--! get_state : StateRow
SELECT email_modseq, mailbox_modseq, submission_modseq
FROM state WHERE account_id = :account_id;

--! count_ready_accounts
SELECT COUNT(*) FROM state
WHERE initial_sync_done = 1 AND account_id = ANY(:account_ids);
