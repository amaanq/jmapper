//! OAuth 2 for Gmail: auth URL construction, code exchange, refresh.
//!
//! Google "Desktop app" OAuth client; scope `https://mail.google.com/` (IMAP +
//! SMTP). Refresh tokens live in the PostgreSQL cache and are used to
//! mint fresh access tokens that feed IMAP XOAUTH2 SASL.

use std::fmt::Display;

use chrono::{
   DateTime,
   Utc,
};
use oauth2::{
   AuthUrl,
   AuthorizationCode,
   ClientId,
   ClientSecret,
   CsrfToken,
   EndpointNotSet,
   EndpointSet,
   PkceCodeChallenge,
   PkceCodeVerifier,
   RedirectUrl,
   RefreshToken,
   Scope,
   TokenResponse as _,
   TokenUrl,
   basic::{
      BasicClient,
      BasicTokenResponse,
   },
};
use reqwest::redirect::Policy;
use url::Url;

use crate::error::{
   Result,
   SyncError,
};

pub const GMAIL_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GMAIL_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GMAIL_SCOPE: &str = "https://mail.google.com/";

/// Fully-typed Gmail OAuth client. Wrapping the `BasicClient` type aliases
/// keeps the verbose typestate generics out of the public API.
pub struct GmailOAuth {
   inner: BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>,
   http:  reqwest::Client,
}

pub struct PendingAuth {
   pub url:           Url,
   pub csrf:          CsrfToken,
   pub pkce_verifier: PkceCodeVerifier,
   pub redirect_uri:  String,
}

pub struct ExchangedTokens {
   pub access_token:  String,
   pub refresh_token: String,
   pub expires_at:    Option<DateTime<Utc>>,
}

impl GmailOAuth {
   /// # Errors
   ///
   /// Returns [`SyncError`] if the hardcoded Gmail auth/token URLs fail to
   /// parse, or if the underlying `reqwest` client cannot be built.
   pub fn new(client_id: &str, client_secret: &str) -> Result<Self> {
      let inner = BasicClient::new(ClientId::new(client_id.to_owned()))
         .set_client_secret(ClientSecret::new(client_secret.to_owned()))
         .set_auth_uri(AuthUrl::new(GMAIL_AUTH_URL.to_owned()).map_err(oauth_err)?)
         .set_token_uri(TokenUrl::new(GMAIL_TOKEN_URL.to_owned()).map_err(oauth_err)?);
      // SSRF-safe HTTP client per oauth2 crate guidance.
      let http = reqwest::Client::builder()
         .redirect(Policy::none())
         .build()?;
      Ok(Self { inner, http })
   }

   /// Build the authorization URL the user needs to visit. Caller is
   /// responsible for holding the [`PendingAuth`] until the browser
   /// redirects back.
   ///
   /// # Errors
   ///
   /// Returns [`SyncError`] if `redirect_uri` is not a valid URL.
   pub fn start(&self, redirect_uri: &str) -> Result<PendingAuth> {
      // set_redirect_uri takes ownership; cloning the inner client keeps this
      // callable multiple times with different loopback ports in principle.
      let client = self
         .inner
         .clone()
         .set_redirect_uri(RedirectUrl::new(redirect_uri.to_owned()).map_err(oauth_err)?);
      let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
      let (url, csrf) = client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new(GMAIL_SCOPE.to_owned()))
            // `offline` + `consent` forces Google to issue a refresh token even
            // if the user has previously authorized this client.
            .add_extra_param("access_type", "offline")
            .add_extra_param("prompt", "consent")
            .set_pkce_challenge(pkce_challenge)
            .url();
      Ok(PendingAuth {
         url,
         csrf,
         pkce_verifier,
         redirect_uri: redirect_uri.to_owned(),
      })
   }

   /// Exchange an authorization code for access + refresh tokens.
   ///
   /// # Errors
   ///
   /// Returns [`SyncError::OAuth`] if `received_state` does not match the CSRF
   /// token in `pending`, if the stored redirect URI is invalid, or if the
   /// token exchange request to Google fails.
   pub async fn exchange_code(
      &self,
      pending: PendingAuth,
      received_state: &str,
      code: &str,
   ) -> Result<ExchangedTokens> {
      if received_state != pending.csrf.secret() {
         return Err(SyncError::OAuth("CSRF state mismatch on callback".into()));
      }
      let client = self
         .inner
         .clone()
         .set_redirect_uri(RedirectUrl::new(pending.redirect_uri).map_err(oauth_err)?);
      let token = client
         .exchange_code(AuthorizationCode::new(code.to_owned()))
         .set_pkce_verifier(pending.pkce_verifier)
         .request_async(&self.http)
         .await
         .map_err(oauth_err)?;
      Ok(unpack_tokens(&token))
   }

   /// Trade a stored refresh token for a fresh access token (and sometimes a
   /// rotated refresh token — Google occasionally rotates).
   ///
   /// # Errors
   ///
   /// Returns [`SyncError::OAuth`] if the refresh request to Google fails.
   pub async fn refresh(&self, refresh_token: &str) -> Result<ExchangedTokens> {
      let rt = RefreshToken::new(refresh_token.to_owned());
      let token = self
         .inner
         .exchange_refresh_token(&rt)
         .request_async(&self.http)
         .await
         .map_err(oauth_err)?;
      let mut out = unpack_tokens(&token);
      if out.refresh_token.is_empty() {
         // Google often omits the refresh_token on refresh responses; reuse
         // the original so callers can blindly store whatever we return.
         refresh_token.clone_into(&mut out.refresh_token);
      }
      Ok(out)
   }
}

fn unpack_tokens(token: &BasicTokenResponse) -> ExchangedTokens {
   let access_token = token.access_token().secret().to_owned();
   let refresh_token = token
      .refresh_token()
      .map(|refresh| refresh.secret().to_owned())
      .unwrap_or_default();
   let expires_at = token.expires_in().map(|dur| Utc::now() + dur);
   ExchangedTokens {
      access_token,
      refresh_token,
      expires_at,
   }
}

fn oauth_err<E>(err: E) -> SyncError
where
   E: Display,
{
   SyncError::OAuth(err.to_string())
}

/// RFC 5802-ish SASL XOAUTH2 payload for IMAP AUTHENTICATE.
///
/// `base64("user=" + email + "^Aauth=Bearer " + access_token + "^A^A")`.
#[must_use]
pub fn xoauth2_payload(email: &str, access_token: &str) -> String {
   use base64::{
      Engine as _,
      engine::general_purpose::STANDARD,
   };
   let raw = format!("user={email}\x01auth=Bearer {access_token}\x01\x01");
   STANDARD.encode(raw)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn xoauth2_shape() {
      // decode check: roundtrip via base64
      use std::str;

      use base64::{
         Engine as _,
         engine::general_purpose::STANDARD,
      };

      let payload = xoauth2_payload("x@y.com", "tok");
      let decoded = STANDARD.decode(&payload).unwrap();
      assert_eq!(
         str::from_utf8(&decoded).unwrap(),
         "user=x@y.com\x01auth=Bearer tok\x01\x01"
      );
   }
}
