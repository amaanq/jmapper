-- Bindings are never rebutted: once a Message-ID maps to a thrid it stays,
-- so replies threading through either the Gmail or RFC 5322 path converge.
--! bind_message_id
INSERT INTO thread_index (account_id, message_id, thrid)
VALUES (:account_id, :message_id, :thrid)
ON CONFLICT (account_id, message_id) DO NOTHING;

--! thrid_from_refs
SELECT thrid FROM thread_index
WHERE account_id = :account_id AND message_id = ANY(:refs) LIMIT 1;

--! record_subject
INSERT INTO thread_by_subject (account_id, norm_subject, thrid, last_received_at)
VALUES (:account_id, :norm_subject, :thrid, :last_received_at)
ON CONFLICT (account_id, norm_subject) DO UPDATE SET
    thrid = EXCLUDED.thrid,
    last_received_at = GREATEST(thread_by_subject.last_received_at, EXCLUDED.last_received_at);

--! thrid_from_subject
SELECT thrid FROM thread_by_subject
WHERE account_id = :account_id AND norm_subject = :norm_subject
  AND last_received_at >= :cutoff;
