//! Interactive OAuth bootstrap for a Gmail account.
//!
//! Flow:
//!   1. Bind a loopback TCP listener on an ephemeral port.
//!   2. Compute `redirect_uri = http://127.0.0.1:<port>/callback`.
//!   3. Build the Google auth URL; try to open the browser, always print it.
//!   4. Accept a single HTTP GET on the listener; parse `code` and `state`.
//!   5. Exchange the code for access + refresh tokens.
//!   6. Write the refresh token into PostgreSQL.
//!   7. Print the refresh token on stdout so it can be copied to a remote
//!      server when needed.

use std::str;

use anyhow::{
   Context as _,
   Result,
   anyhow,
   bail,
};
use imap_sync::{
   cache,
   db,
   oauth::{
      ExchangedTokens,
      GmailOAuth,
   },
   provider::ProviderKind,
};
use sha2::{
   Digest as _,
   Sha256,
};
use tokio::{
   io::{
      AsyncReadExt as _,
      AsyncWriteExt as _,
   },
   net::{
      TcpListener,
      TcpStream,
   },
};
use tracing::{
   info,
   warn,
};

use crate::config::{
   Account,
   Config,
};

pub async fn run(config: &Config, account_id: &str) -> Result<()> {
   let account = config
      .accounts
      .iter()
      .find(|acct| acct.id == account_id)
      .ok_or_else(|| anyhow!("no account with id {account_id:?} in config"))?;
   if account.provider != ProviderKind::Gmail {
      bail!(
         "account {account_id} is provider={:?}; bootstrap only supports gmail",
         account.provider
      );
   }
   let creds = account
      .gmail
      .as_ref()
      .ok_or_else(|| anyhow!("account {account_id} missing [accounts.gmail]"))?;
   let (client_id, client_secret) = creds.oauth().ok_or_else(|| {
      anyhow!("account {account_id} uses an app password; OAuth bootstrap is unnecessary")
   })?;

   // 1-2. Bind loopback, compute redirect URI.
   let listener = TcpListener::bind("127.0.0.1:0")
      .await
      .context("binding loopback callback listener")?;
   let port = listener.local_addr()?.port();
   let redirect_uri = format!("http://127.0.0.1:{port}/callback");

   // 3. Build auth URL and hand it to the user.
   let oauth = GmailOAuth::new(client_id, client_secret)?;
   let pending = oauth.start(&redirect_uri)?;
   println!();
   println!("  Opening your browser to authorize {}.", account.email);
   println!("  If the browser does not open, paste this URL:");
   println!();
   println!("    {}", pending.url);
   println!();
   if webbrowser::open(pending.url.as_str()).is_err() {
      warn!("couldn't open browser automatically; use the URL above");
   }

   // 4. Accept a single callback connection.
   let (mut sock, peer) = listener
      .accept()
      .await
      .context("waiting for OAuth redirect")?;
   info!(%peer, "received callback");

   let mut buf = vec![0_u8; 8 * 1024];
   let mut total = 0;
   // Read until we have the request line + headers (ends in \r\n\r\n). We
   // don't need the body.
   loop {
      let n = sock.read(&mut buf[total..]).await?;
      if n == 0 {
         break;
      }
      total += n;
      if buf[..total].windows(4).any(|w| w == b"\r\n\r\n") {
         break;
      }
      if total >= buf.len() {
         bail!("callback request too large (>8KB)");
      }
   }
   let request = str::from_utf8(&buf[..total]).context("callback request was not UTF-8")?;
   let request_line = request
      .lines()
      .next()
      .ok_or_else(|| anyhow!("empty callback request"))?;

   // Parse `GET /callback?code=...&state=... HTTP/1.1`
   let path = request_line
      .split_whitespace()
      .nth(1)
      .ok_or_else(|| anyhow!("malformed request line: {request_line:?}"))?;
   let parsed =
      url::Url::parse(&format!("http://127.0.0.1{path}")).context("parsing callback path")?;

   let code = parsed
      .query_pairs()
      .find(|(key, _)| key == "code")
      .map(|(_, value)| value.into_owned());
   let state = parsed
      .query_pairs()
      .find(|(key, _)| key == "state")
      .map(|(_, value)| value.into_owned());
   let err_param = parsed
      .query_pairs()
      .find(|(key, _)| key == "error")
      .map(|(_, value)| value.into_owned());

   // Always answer the browser, even on error.
   if err_param.is_some() || code.is_none() || state.is_none() {
      respond(&mut sock, "Authorization failed. Check the terminal.").await?;
      bail!(
         "Google returned error={:?}, code.is_some={}, state.is_some={}",
         err_param,
         code.is_some(),
         state.is_some()
      );
   }
   respond(&mut sock, "Authorization received. You can close this tab.").await?;

   // 5. Exchange code for tokens.
   let code = code.unwrap();
   let state = state.unwrap();
   let tokens = oauth.exchange_code(pending, &state, &code).await?;
   info!(
      "got access token ({} bytes), refresh token ({} bytes)",
      tokens.access_token.len(),
      tokens.refresh_token.len(),
   );

   // 6. Persist in the per-account cache.
   write_tokens_to_cache(config, account, &tokens).await?;

   // 7. Print for copy-to-server workflows.
   println!();
   println!("  Refresh token (store only on trusted machines):");
   println!();
   println!("    {}", tokens.refresh_token);
   println!();
   println!(
      "  It has been written into the oauth_tokens table for account {}. If bootstrapping for a \
       different server, use the refresh token above to seed its database.",
      account.id,
   );
   Ok(())
}

async fn respond(sock: &mut TcpStream, body: &str) -> Result<()> {
   let html = format!(
      "<!doctype html><meta charset=utf-8><title>jmapper</title><body style=\"font-family: \
       system-ui, sans-serif; padding: 2rem;\"><h1>jmapper</h1><p>{body}</p></body>"
   );
   let response = format!(
      "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: \
       {}\r\nConnection: close\r\n\r\n{html}",
      html.len()
   );
   sock.write_all(response.as_bytes()).await?;
   let _ = sock.shutdown().await;
   Ok(())
}

async fn write_tokens_to_cache(
   config: &Config,
   account: &Account,
   tokens: &ExchangedTokens,
) -> Result<()> {
   let pool = cache::open(&config.server.database_url)
      .await
      .context("connecting to postgres / initializing schema")?;

   let hash = {
      let mut hasher = Sha256::new();
      hasher.update(account.bearer_token.as_bytes());
      hasher.finalize().to_vec()
   };
   db::upsert_account(
      &pool,
      &account.id,
      &account.email,
      account.provider,
      &account.display_name,
      &hash,
   )
   .await
   .context("upserting account record")?;

   db::upsert_oauth(
      &pool,
      &account.id,
      Some(&tokens.access_token),
      &tokens.refresh_token,
      tokens.expires_at,
   )
   .await
   .context("storing oauth tokens")?;

   Ok(())
}
