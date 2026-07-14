--: AccountRow()

--! upsert_account
INSERT INTO accounts (id, email, provider, display_name, bearer_token_hash, created_at)
VALUES (:id, :email, :provider, :display_name, :bearer_token_hash, EXTRACT(EPOCH FROM now())::bigint)
ON CONFLICT (id) DO UPDATE SET
    email = EXCLUDED.email,
    provider = EXCLUDED.provider,
    display_name = EXCLUDED.display_name,
    bearer_token_hash = EXCLUDED.bearer_token_hash;

--! ensure_state_row
INSERT INTO state (account_id) VALUES (:account_id)
ON CONFLICT (account_id) DO NOTHING;

--! get_account : AccountRow
SELECT id, email, provider, display_name, bearer_token_hash, created_at
FROM accounts WHERE id = :id;

--! list_accounts : AccountRow
SELECT id, email, provider, display_name, bearer_token_hash, created_at
FROM accounts ORDER BY id;
