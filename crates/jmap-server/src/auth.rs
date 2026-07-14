//! Token authentication middleware.

use std::str;

use axum::{
   extract::{
      Request,
      State,
   },
   http::header::AUTHORIZATION,
   middleware::Next,
   response::Response,
};
use base64::{
   Engine as _,
   engine::general_purpose::STANDARD,
};

use crate::{
   error::ApiError,
   state::{
      AccountInfo,
      AppState,
   },
};

#[derive(Debug, PartialEq, Eq)]
enum Credentials {
   Bearer(String),
   Basic { username: String, token: String },
}

fn parse_credentials(header: &str) -> Option<Credentials> {
   let mut parts = header.split_whitespace();
   let scheme = parts.next()?;
   let value = parts.next()?;
   if parts.next().is_some() {
      return None;
   }

   if scheme.eq_ignore_ascii_case("Bearer") {
      return Some(Credentials::Bearer(value.to_owned()));
   }
   if !scheme.eq_ignore_ascii_case("Basic") {
      return None;
   }

   let decoded = STANDARD.decode(value).ok()?;
   let decoded = str::from_utf8(&decoded).ok()?;
   let (username, token) = decoded.split_once(':')?;
   if username.is_empty() || token.is_empty() {
      return None;
   }
   Some(Credentials::Basic {
      username: username.to_owned(),
      token:    token.to_owned(),
   })
}

/// Identity of the request, injected by [`auth_middleware`].
#[derive(Debug, Clone)]
pub struct AuthedAccount(pub AccountInfo);

/// Authenticates the request from its `Authorization` header and injects the
/// resolved [`AuthedAccount`] into request extensions.
///
/// # Errors
///
/// Returns [`ApiError::Unauthorized`] if the `Authorization` header is missing
/// or non-ASCII, its credentials are malformed, or no account matches the
/// presented bearer or basic credentials.
pub async fn auth_middleware(
   State(state): State<AppState>,
   mut req: Request,
   next: Next,
) -> Result<Response, ApiError> {
   let header = req
      .headers()
      .get(AUTHORIZATION)
      .and_then(|value| value.to_str().ok())
      .ok_or(ApiError::Unauthorized)?;
   let acct = match parse_credentials(header).ok_or(ApiError::Unauthorized)? {
      Credentials::Bearer(token) => state.account_by_bearer(&token),
      Credentials::Basic { username, token } => state.account_by_basic(&username, &token),
   }
   .ok_or(ApiError::Unauthorized)?;
   req.extensions_mut().insert(AuthedAccount(acct));
   Ok(next.run(req).await)
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn parses_bearer_and_basic_credentials() {
      assert_eq!(
         parse_credentials("Bearer token"),
         Some(Credentials::Bearer("token".into()))
      );
      let basic = STANDARD.encode("user@example.test:token:with:colons");
      assert_eq!(
         parse_credentials(&format!("Basic {basic}")),
         Some(Credentials::Basic {
            username: "user@example.test".into(),
            token:    "token:with:colons".into(),
         })
      );
   }

   #[test]
   fn rejects_malformed_credentials() {
      for header in [
         "",
         "Digest token",
         "Bearer",
         "Bearer token extra",
         "Basic not-base64",
         "Basic Og==",
      ] {
         assert_eq!(parse_credentials(header), None, "{header:?}");
      }
   }
}
