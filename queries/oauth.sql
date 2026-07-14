--: OAuthTokenRow(access_token?, expires_at?)

--! upsert_oauth (access_token?, expires_at?)
INSERT INTO oauth_tokens (account_id, access_token, refresh_token, expires_at)
VALUES (:account_id, :access_token, :refresh_token, :expires_at)
ON CONFLICT (account_id) DO UPDATE SET
    access_token = EXCLUDED.access_token,
    refresh_token = EXCLUDED.refresh_token,
    expires_at = EXCLUDED.expires_at;

--! get_oauth : OAuthTokenRow
SELECT account_id, access_token, refresh_token, expires_at
FROM oauth_tokens WHERE account_id = :account_id;
