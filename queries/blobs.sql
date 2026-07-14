--! upsert_uploaded_blob
INSERT INTO uploaded_blobs (account_id, blob_id, content_type, bytes, uploaded_at, expires_at)
VALUES (:account_id, :blob_id, :content_type, :bytes, :uploaded_at, :expires_at)
ON CONFLICT (account_id, blob_id) DO UPDATE SET
    content_type = EXCLUDED.content_type,
    bytes = EXCLUDED.bytes,
    uploaded_at = EXCLUDED.uploaded_at,
    expires_at = EXCLUDED.expires_at;

--: UploadedBlobRow()

--! get_uploaded_blob : UploadedBlobRow
SELECT bytes, content_type FROM uploaded_blobs
WHERE account_id = :account_id AND blob_id = :blob_id
  AND expires_at > EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)::BIGINT;
