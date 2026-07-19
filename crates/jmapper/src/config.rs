//! TOML config schema + loader.

use std::{
   collections::HashSet,
   fs,
   net::SocketAddr,
   path::Path,
};

use anyhow::{
   Context as _,
   Result,
};
use imap_sync::{
   account::GmailAuth,
   provider::{
      ImapTls,
      ProviderKind,
   },
};
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
   pub server:   Server,
   #[serde(default)]
   pub accounts: Vec<Account>,
}

#[derive(Clone, Deserialize)]
pub struct Server {
   pub bind:                      SocketAddr,
   pub session_url:               String,
   #[serde(default)]
   pub cors_origins:              Vec<String>,
   /// libpq-style connection string, e.g. `host=/run/postgresql dbname=jmapper`
   /// or `host=localhost user=jmapper password=… dbname=jmapper`.
   pub database_url:              String,
   /// Seconds between background CalDAV/CardDAV refresh rounds. Zero
   /// disables periodic refresh; JMAP writes and explicit initial syncs
   /// still use the configured endpoints.
   #[serde(default = "default_dav_sync_interval_seconds")]
   pub dav_sync_interval_seconds: u64,
}

#[derive(Clone, Deserialize)]
pub struct Account {
   pub id:            String,
   pub email:         String,
   pub display_name:  String,
   pub provider:      ProviderKind,
   pub bearer_token:  String,
   /// How many days back to ingest on the first sync. Zero fetches the full
   /// folder history.
   #[serde(default = "default_backfill_days")]
   pub backfill_days: u32,
   pub gmail:         Option<GmailAuth>,
   pub imap:          Option<ImapCreds>,
   /// SMTP submission endpoint. Optional for Gmail (defaults to
   /// smtp.gmail.com:465 with the same credentials); required on generic
   /// IMAP accounts for `EmailSubmission/set` to work.
   pub smtp:          Option<SmtpCreds>,
   pub caldav:        Option<DavEndpointConfig>,
   pub carddav:       Option<DavEndpointConfig>,
}

const fn default_backfill_days() -> u32 {
   0
}

const fn default_dav_sync_interval_seconds() -> u64 {
   60
}

#[derive(Clone, Deserialize)]
#[serde(tag = "auth", rename_all = "lowercase")]
pub enum DavEndpointConfig {
   None {
      url: String,
   },
   Basic {
      url:      String,
      username: String,
      password: String,
   },
   Bearer {
      url:   String,
      token: String,
   },
}

impl DavEndpointConfig {
   pub fn url(&self) -> &str {
      match self {
         Self::None { url } | Self::Basic { url, .. } | Self::Bearer { url, .. } => url,
      }
   }

   pub fn auth_parts(&self) -> (&'static str, Option<&str>, Option<&str>) {
      match self {
         Self::None { .. } => ("none", None, None),
         Self::Basic {
            username, password, ..
         } => ("basic", Some(username), Some(password)),
         Self::Bearer { token, .. } => ("bearer", None, Some(token)),
      }
   }

   fn validate(&self, account_id: &str, label: &str) -> Result<()> {
      let parsed = url::Url::parse(self.url())
         .with_context(|| format!("account {account_id}: invalid {label} URL"))?;
      if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
         anyhow::bail!("account {account_id}: {label} URL must be an absolute http(s) URL");
      }
      if !parsed.username().is_empty() || parsed.password().is_some() {
         anyhow::bail!("account {account_id}: {label} credentials must not be embedded in the URL");
      }
      match self {
         Self::Basic {
            username, password, ..
         } if username.is_empty() || password.is_empty() => {
            anyhow::bail!("account {account_id}: {label} basic username/password must not be empty")
         },
         Self::Bearer { token, .. } if token.is_empty() => {
            anyhow::bail!("account {account_id}: {label} bearer token must not be empty")
         },
         _ => Ok(()),
      }
   }
}

#[derive(Clone, Deserialize)]
pub struct SmtpCreds {
   pub host:     String,
   pub port:     u16,
   #[serde(default = "default_smtp_tls")]
   pub tls:      ImapTls,
   /// Fall back to the IMAP username/password when unset.
   pub username: Option<String>,
   pub password: Option<String>,
}

const fn default_smtp_tls() -> ImapTls {
   ImapTls::Implicit
}

#[derive(Clone, Deserialize)]
pub struct ImapCreds {
   pub host:     String,
   pub port:     u16,
   pub tls:      ImapTls,
   pub username: String,
   pub password: String,
}

impl Config {
   pub fn load(path: &Path) -> Result<Self> {
      let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
      let cfg = toml::from_str::<Self>(&text).context("parsing TOML")?;
      cfg.validate()?;
      Ok(cfg)
   }

   fn validate(&self) -> Result<()> {
      if self.server.cors_origins.is_empty() {
         anyhow::bail!(
            "server.cors_origins must not be empty — set the origin your client serves from. Use \
             [\"*\"] only if you truly mean to allow any."
         );
      }
      let mut seen = HashSet::new();
      for account in &self.accounts {
         if !seen.insert(&account.id) {
            anyhow::bail!("duplicate account id: {}", account.id);
         }
         if account.bearer_token.trim().is_empty() {
            anyhow::bail!(
               "account {}: bearer_token is empty — generate one with e.g. `openssl rand -hex 32`",
               account.id
            );
         }
         if account.bearer_token.len() < 16 {
            anyhow::bail!(
               "account {}: bearer_token is shorter than 16 chars; refusing weak tokens",
               account.id
            );
         }
         if account.gmail.is_some() && account.imap.is_some() {
            anyhow::bail!(
               "account {}: only one of [accounts.gmail] or [accounts.imap] may be set",
               account.id
            );
         }
         match account.provider {
            ProviderKind::Gmail => {
               let gmail = account.gmail.as_ref().ok_or_else(|| {
                  anyhow::anyhow!(
                     "account {}: provider=gmail requires [accounts.gmail]",
                     account.id
                  )
               })?;
               match gmail {
                  GmailAuth::AppPassword { app_password }
                     if app_password.chars().all(char::is_whitespace) =>
                  {
                     anyhow::bail!(
                        "account {}: Gmail app_password must not be empty",
                        account.id
                     )
                  },
                  GmailAuth::OAuth {
                     client_id,
                     client_secret,
                  } if client_id.trim().is_empty() || client_secret.trim().is_empty() => {
                     anyhow::bail!(
                        "account {}: Gmail OAuth client_id/client_secret must not be empty",
                        account.id
                     )
                  },
                  _ => {},
               }
            },
            ProviderKind::Imap => {
               if account.imap.is_none() {
                  anyhow::bail!(
                     "account {}: provider=imap requires [accounts.imap]",
                     account.id
                  );
               }
            },
         }
         if let Some(endpoint) = &account.caldav {
            endpoint.validate(&account.id, "CalDAV")?;
         }
         if let Some(endpoint) = &account.carddav {
            endpoint.validate(&account.id, "CardDAV")?;
         }
      }
      Ok(())
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   const BASE: &str = r#"
[server]
bind = "127.0.0.1:8765"
session_url = "http://127.0.0.1:8765"
cors_origins = ["http://localhost:3000"]
database_url = "host=/tmp dbname=jmapper"

[[accounts]]
id = "account"
email = "user@example.test"
display_name = "Example"
provider = "imap"
bearer_token = "long-enough-bearer-token"

[accounts.imap]
host = "imap.example.test"
port = 993
tls = "implicit"
username = "user@example.test"
password = "imap-secret"
"#;

   #[test]
   fn dav_endpoints_parse_and_the_refresh_interval_defaults() {
      let text = format!(
            "{BASE}\n\
             [accounts.caldav]\n\
             url = \"https://dav.example.test/cal/\"\n\
             auth = \"basic\"\n\
             username = \"user@example.test\"\n\
             password = \"dav-secret\"\n\n\
             [accounts.carddav]\n\
             url = \"https://dav.example.test/card/\"\n\
             auth = \"bearer\"\n\
             token = \"dav-token\"\n"
        );
      let config = toml::from_str::<Config>(&text).unwrap();

      config.validate().unwrap();
      assert_eq!(config.server.dav_sync_interval_seconds, 60);
      assert_eq!(config.accounts[0].backfill_days, 0);
      assert_eq!(
         config.accounts[0].caldav.as_ref().unwrap().auth_parts(),
         ("basic", Some("user@example.test"), Some("dav-secret"))
      );
      assert_eq!(
         config.accounts[0].carddav.as_ref().unwrap().auth_parts(),
         ("bearer", None, Some("dav-token"))
      );
   }

   #[test]
   fn dav_endpoint_validation_rejects_embedded_or_empty_credentials() {
      let embedded = format!(
         "{BASE}\n[accounts.caldav]\nurl = \"https://user:secret@dav.example.test/\"\nauth = \
          \"none\"\n"
      );
      let config = toml::from_str::<Config>(&embedded).unwrap();
      assert!(
         config
            .validate()
            .unwrap_err()
            .to_string()
            .contains("embedded")
      );

      let empty = format!(
         "{BASE}\n[accounts.carddav]\nurl = \"https://dav.example.test/\"\nauth = \
          \"bearer\"\ntoken = \"\"\n"
      );
      let config = toml::from_str::<Config>(&empty).unwrap();
      assert!(config.validate().unwrap_err().to_string().contains("empty"));
   }

   #[test]
   fn gmail_app_password_is_compacted_for_login() {
      let text = r#"
[server]
bind = "127.0.0.1:8765"
session_url = "http://127.0.0.1:8765"
cors_origins = ["http://localhost:3000"]
database_url = "host=/tmp dbname=jmapper"

[[accounts]]
id = "gmail"
email = "user@gmail.com"
display_name = "Gmail"
provider = "gmail"
bearer_token = "long-enough-bearer-token"

[accounts.gmail]
app_password = "abcd efgh ijkl mnop"
"#;
      let config = toml::from_str::<Config>(text).unwrap();

      config.validate().unwrap();
      let password = config.accounts[0]
         .gmail
         .as_ref()
         .unwrap()
         .app_password()
         .unwrap();
      assert_eq!(password, "abcdefghijklmnop");

      let oauth = text.replace(
         "app_password = \"abcd efgh ijkl mnop\"",
         "client_id = \"client-id\"\nclient_secret = \"client-secret\"",
      );
      let config = toml::from_str::<Config>(&oauth).unwrap();
      config.validate().unwrap();
      assert_eq!(
         config.accounts[0].gmail.as_ref().unwrap().oauth(),
         Some(("client-id", "client-secret"))
      );
   }
}
