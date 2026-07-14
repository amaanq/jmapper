--: SubmissionRow(thread_id?, delivery_status_json?)

--! insert_submission (thread_id?, raw_rfc822?, delivery_status_json?)
INSERT INTO email_submissions
    (account_id, id, email_id, identity_id, thread_id, envelope_json, send_at,
     undo_status, raw_rfc822, delivery_status_json, modseq)
VALUES (:account_id, :id, :email_id, :identity_id, :thread_id, :envelope_json, :send_at,
        :undo_status, :raw_rfc822, :delivery_status_json, :modseq);

--! get_submissions_by_ids : SubmissionRow
SELECT id, email_id, identity_id, thread_id, envelope_json, send_at, undo_status,
       delivery_status_json
FROM email_submissions
WHERE account_id = :account_id AND destroyed = 0 AND id = ANY(:ids);

--! list_submissions : SubmissionRow
SELECT id, email_id, identity_id, thread_id, envelope_json, send_at, undo_status,
       delivery_status_json
FROM email_submissions
WHERE account_id = :account_id AND destroyed = 0
ORDER BY send_at DESC LIMIT 500;

--: SubmissionChangeRow()

--! submissions_changed_since : SubmissionChangeRow
SELECT id, destroyed FROM email_submissions
WHERE account_id = :account_id AND modseq > :since
ORDER BY modseq LIMIT 500;

--! submission_undo_status
SELECT undo_status FROM email_submissions
WHERE account_id = :account_id AND id = :id AND destroyed = 0;

-- The WHERE clause is the cancel-vs-relay race arbiter: if the scheduler
-- claimed the row first, zero rows change and the client gets cannotUnsend.
--! cancel_pending_submission
UPDATE email_submissions SET undo_status = 'canceled', modseq = :modseq
WHERE account_id = :account_id AND id = :id AND destroyed = 0
  AND undo_status = 'pending';

--! tombstone_submission
UPDATE email_submissions SET destroyed = 1, raw_rfc822 = NULL, modseq = :modseq
WHERE account_id = :account_id AND id = :id AND destroyed = 0
  AND undo_status IN ('final', 'canceled');

--! recover_stranded_submissions
UPDATE email_submissions SET undo_status = 'pending' WHERE undo_status = 'sending';

--: DueSubmissionRow(raw_rfc822?)

--! due_submissions : DueSubmissionRow
SELECT account_id, id, envelope_json, raw_rfc822, attempts
FROM email_submissions
WHERE undo_status = 'pending' AND destroyed = 0 AND send_at <= :now
LIMIT 50;

--! claim_submission
UPDATE email_submissions SET undo_status = 'sending'
WHERE account_id = :account_id AND id = :id AND undo_status = 'pending';

--! finish_submission (delivery_status_json?)
UPDATE email_submissions SET undo_status = :undo_status, raw_rfc822 = NULL,
    delivery_status_json = COALESCE(:delivery_status_json, delivery_status_json),
    modseq = :modseq
WHERE account_id = :account_id AND id = :id;

--! retry_submission
UPDATE email_submissions SET undo_status = 'pending', attempts = attempts + 1
WHERE account_id = :account_id AND id = :id AND undo_status = 'sending';
