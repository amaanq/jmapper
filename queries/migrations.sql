--! current_schema_version
SELECT COALESCE(MAX(version), 0) AS version FROM schema_version;

--! record_schema_version
INSERT INTO schema_version (version, applied_at)
VALUES (:version, EXTRACT(EPOCH FROM now())::bigint);

--! baseline_already_applied
SELECT (to_regclass('public.accounts') IS NOT NULL) AS applied;
