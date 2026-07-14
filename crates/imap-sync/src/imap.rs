//! IMAP connection + folder listing.
//!
//! Transport layer:
//!   raw TCP → tokio-rustls TLS → async-imap's Tokio backend.
//!
//! We pin rustls (via tokio-rustls re-export) with the `ring` crypto provider
//! so nothing in this tree depends on OpenSSL.

use std::{
   collections::HashMap,
   sync::{
      Arc,
      Once,
   },
};

use async_imap::{
   Client,
   Session,
};
use rustls_pki_types::ServerName;
use tokio::net::TcpStream;
use tokio_rustls::{
   TlsConnector,
   client::TlsStream,
   rustls::{
      ClientConfig,
      RootCertStore,
      crypto::ring,
   },
};
use tracing::debug;

use crate::{
   error::{
      Result,
      SyncError,
   },
   provider::ImapTls,
};

/// Type alias for a fully wrapped async-imap session we own.
pub type ImapSession = Session<TlsStream<TcpStream>>;
/// Pre-login client.
pub type ImapClient = Client<TlsStream<TcpStream>>;

static CRYPTO_INIT: Once = Once::new();

fn ensure_crypto_provider() {
   CRYPTO_INIT.call_once(|| {
      // We use ring; if some other crate has already installed aws-lc or
      // similar, that's fine — install_default returns Err but we ignore it.
      let _ = ring::default_provider().install_default();
   });
}

fn rustls_config() -> Arc<ClientConfig> {
   let mut roots = RootCertStore::empty();
   roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
   let cfg = ClientConfig::builder()
      .with_root_certificates(roots)
      .with_no_client_auth();
   Arc::new(cfg)
}

/// TLS config for the SMTP submission client — same roots, same provider
/// bootstrap. Lives here so the ring-vs-aws-lc install stays in one place.
pub(crate) fn smtp_rustls_config() -> Arc<ClientConfig> {
   ensure_crypto_provider();
   rustls_config()
}

/// Open an IMAP connection to `host:port` and return a pre-login
/// [`ImapClient`] over TLS.
///
/// Implicit TLS (993) handshakes immediately. STARTTLS (143) speaks the
/// pre-TLS protocol first — greeting, `STARTTLS`, OK — then upgrades. Either
/// way the resulting stream type is identical. With implicit TLS the server
/// greeting stays unread in the buffer; async-imap treats it as ordinary
/// untagged data ahead of the first tagged reply. With STARTTLS we consumed
/// it during negotiation and no second greeting follows the handshake
/// (RFC 3501 §6.2.1), which async-imap is equally happy with.
///
/// # Errors
///
/// Returns [`SyncError`] if the TCP connection fails, STARTTLS negotiation is
/// rejected, `host` is not a valid TLS server name, or the TLS handshake fails.
pub async fn connect(host: &str, port: u16, tls: ImapTls) -> Result<ImapClient> {
   ensure_crypto_provider();

   debug!(host, port, "connecting tcp");
   let tcp = TcpStream::connect((host, port)).await?;
   let _ = tcp.set_nodelay(true);

   let tcp = match tls {
      ImapTls::Implicit => tcp,
      ImapTls::Starttls => negotiate_starttls(tcp, host).await?,
   };

   let connector = TlsConnector::from(rustls_config());
   let server_name = ServerName::try_from(host.to_owned())
      .map_err(|err| SyncError::Tls(format!("invalid server name {host:?}: {err}")))?;
   debug!(host, "starting tls handshake");
   let tls_stream = connector
      .connect(server_name, tcp)
      .await
      .map_err(|err| SyncError::Tls(err.to_string()))?;

   Ok(Client::new(tls_stream))
}

/// Plaintext IMAP exchange up to the point where TLS may begin: consume the
/// greeting, issue STARTTLS, wait for the tagged OK. Anything unexpected
/// aborts — we never fall back to plaintext auth.
///
/// Buffered over-read is safe here: after its OK the server sends nothing
/// until it sees our TLS `ClientHello`, so the buffer can't swallow TLS bytes.
async fn negotiate_starttls(tcp: TcpStream, host: &str) -> Result<TcpStream> {
   use tokio::io::{
      AsyncBufReadExt as _,
      AsyncWriteExt as _,
      BufReader,
   };

   let mut reader = BufReader::new(tcp);
   let mut line = String::new();

   reader.read_line(&mut line).await?;
   debug!(host, greeting = %line.trim_end(), "pre-TLS greeting");
   if !line.starts_with("* OK") && !line.starts_with("* PREAUTH") {
      return Err(SyncError::Tls(format!(
         "unexpected IMAP greeting before STARTTLS: {}",
         line.trim_end()
      )));
   }

   reader.get_mut().write_all(b"a0 STARTTLS\r\n").await?;
   loop {
      line.clear();
      if reader.read_line(&mut line).await? == 0 {
         return Err(SyncError::Tls(
            "connection closed during STARTTLS negotiation".into(),
         ));
      }
      let trimmed = line.trim_end();
      if let Some(rest) = trimmed.strip_prefix("a0 ") {
         if rest.starts_with("OK") {
            return Ok(reader.into_inner());
         }
         return Err(SyncError::Tls(format!(
            "server rejected STARTTLS: {trimmed}"
         )));
      }
      // Untagged noise (e.g. a CAPABILITY line) before the tagged reply.
      debug!(host, line = %trimmed, "pre-TLS untagged line");
   }
}

/// XOAUTH2 authenticator for Gmail.
///
/// Sends `user=<email>\x01auth=Bearer <token>\x01\x01` on the first server
/// challenge; async-imap base64-encodes the return value before emitting it.
pub struct XOAuth2<'a> {
   pub email:        &'a str,
   pub access_token: &'a str,
   pub sent:         bool,
}

impl<'a> XOAuth2<'a> {
   #[must_use]
   pub const fn new(email: &'a str, access_token: &'a str) -> Self {
      Self {
         email,
         access_token,
         sent: false,
      }
   }
}

impl async_imap::Authenticator for &mut XOAuth2<'_> {
   type Response = String;
   fn process(&mut self, _challenge: &[u8]) -> Self::Response {
      if self.sent {
         // If the server sends a second challenge it's an error blob; the
         // client side continues the IMAP flow by submitting an empty line.
         String::new()
      } else {
         self.sent = true;
         format!(
            "user={}\x01auth=Bearer {}\x01\x01",
            self.email, self.access_token
         )
      }
   }
}

/// Log in to `client` with the Gmail XOAUTH2 SASL mechanism, consuming the
/// pre-login client and returning an authenticated [`ImapSession`].
///
/// # Errors
///
/// Returns [`SyncError::Imap`] if the server rejects the credentials or the
/// XOAUTH2 exchange otherwise fails.
pub async fn authenticate_xoauth2(
   client: ImapClient,
   email: &str,
   access_token: &str,
) -> Result<ImapSession> {
   let mut auth = XOAuth2::new(email, access_token);
   let session = client
      .authenticate("XOAUTH2", &mut auth)
      .await
      .map_err(|(err, _client)| SyncError::Imap(err))?;
   Ok(session)
}

/// Normalized IMAP folder description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImapFolder {
   /// Canonical server name (what IMAP SELECT/STATUS use).
   pub name:      String,
   /// Separator character the server reported ('/' for Gmail, often '.' on
   /// Dovecot, '\0' if the server reports no separator).
   pub delimiter: char,
   /// RFC 6154 special-use flags: `\Inbox`, `\Sent`, `\Drafts`, `\Trash`,
   /// `\Junk`, `\Archive`, `\All`, `\Flagged`, `\Noinferiors`, etc. We keep
   /// them lowercase without the leading backslash.
   pub flags:     Vec<String>,
   /// Mapped JMAP role, if any. Derived from `flags` + `name` heuristics.
   pub role:      Option<String>,
}

impl ImapFolder {
   /// Map server flags + name to a JMAP role string. Gmail's XLIST/LIST
   /// SPECIAL-USE reliably marks `\All`, `\Sent`, `\Drafts`, `\Junk`,
   /// `\Trash`, `\Important`, `\Flagged`; INBOX gets the canonical `inbox`
   /// role regardless of flags.
   #[must_use]
   pub fn derive_role(name: &str, flags: &[String]) -> Option<String> {
      if name.eq_ignore_ascii_case("INBOX") {
         return Some("inbox".into());
      }
      for flag in flags {
         match flag.to_ascii_lowercase().as_str() {
            "inbox" => return Some("inbox".into()),
            "sent" => return Some("sent".into()),
            "drafts" => return Some("drafts".into()),
            "trash" => return Some("trash".into()),
            "junk" | "spam" => return Some("junk".into()),
            "archive" => return Some("archive".into()),
            "all" => return Some("all".into()),
            _ => {}, // `\Flagged`, `\Important`, etc. are not JMAP roles
         }
      }
      None
   }
}

/// List every folder the server exposes via `LIST "" "*"`, normalized into
/// [`ImapFolder`] values with derived JMAP roles.
///
/// # Errors
///
/// Returns [`SyncError`] if the `LIST` command fails or a folder entry cannot
/// be read from the response stream.
pub async fn list_folders(session: &mut ImapSession) -> Result<Vec<ImapFolder>> {
   use futures::StreamExt as _;

   // RFC 6154: LIST "" "*" RETURN (SPECIAL-USE) — but for broad compatibility
   // and because Gmail always emits special-use flags inline, a plain LIST is
   // fine.
   let mut names_stream = session.list(Some(""), Some("*")).await?;
   let mut out = Vec::new();
   while let Some(name) = names_stream.next().await {
      let name = name?;
      let flags = name
         .attributes()
         .iter()
         .filter_map(|attr| format_attribute(attr))
         .collect::<Vec<String>>();
      let folder = ImapFolder {
         name: name.name().to_owned(),
         delimiter: name
            .delimiter()
            .and_then(|sep| sep.chars().next())
            .unwrap_or('\0'),
         role: ImapFolder::derive_role(name.name(), &flags),
         flags,
      };
      out.push(folder);
   }
   Ok(out)
}

fn format_attribute(attr: &async_imap::types::NameAttribute) -> Option<String> {
   use async_imap::types::NameAttribute;
   match attr {
      NameAttribute::NoInferiors => Some("noinferiors".into()),
      NameAttribute::NoSelect => Some("noselect".into()),
      NameAttribute::Marked => Some("marked".into()),
      NameAttribute::Unmarked => Some("unmarked".into()),
      NameAttribute::All => Some("all".into()),
      NameAttribute::Archive => Some("archive".into()),
      NameAttribute::Drafts => Some("drafts".into()),
      NameAttribute::Flagged => Some("flagged".into()),
      NameAttribute::Junk => Some("junk".into()),
      NameAttribute::Sent => Some("sent".into()),
      NameAttribute::Trash => Some("trash".into()),
      NameAttribute::Extension(raw) => {
         // Extension flags arrive as `\All`, `\Sent`, etc. Strip the
         // leading backslash and lowercase so downstream matching is
         // case-insensitive.
         let flag = raw.trim_start_matches('\\').to_ascii_lowercase();
         if flag.is_empty() { None } else { Some(flag) }
      },
      _ => None, // `NameAttribute` is #[non_exhaustive]
   }
}

/// Whether the server advertises the `IDLE` capability (RFC 2177).
///
/// # Errors
///
/// Returns [`SyncError`] if the capability probe fails.
pub async fn has_idle_capability(session: &mut ImapSession) -> Result<bool> {
   has_capability(session, "IDLE").await
}

/// Gmail's IMAP extensions capability (X-GM-MSGID / X-GM-THRID / X-GM-LABELS).
pub const GMAIL_EXT_CAPABILITY: &str = "X-GM-EXT-1";

/// Fetch `X-GM-THRID` for a set of UIDs in the selected folder.
///
/// async-imap's typed `Fetch` parses the attribute (imap-proto 0.16 has
/// `AttributeValue::GmailThrId`) but exposes no accessor for it, and the
/// underlying `ResponseData` is private. The escape hatch: run the FETCH as
/// a raw command — untagged FETCH responses then arrive on the public
/// `unsolicited_responses` channel as `Other(ResponseData)`, whose `parsed()`
/// view IS public. The channel is bounded at 100 with silent drops, so we
/// chunk well below that and drain after every chunk.
///
/// # Errors
///
/// Returns [`SyncError`] if any `UID FETCH` command fails.
pub async fn fetch_gmail_thrids(
   session: &mut ImapSession,
   uids: &[u32],
) -> Result<HashMap<u32, u64>> {
   use async_imap::{
      imap_proto::types::{
         AttributeValue,
         Response,
      },
      types::UnsolicitedResponse,
   };

   let mut map = HashMap::with_capacity(uids.len());
   // Clear leftover IDLE-era noise so a full channel can't eat our replies.
   while session.unsolicited_responses.try_recv().is_ok() {}

   for chunk in uids.chunks(80) {
      let set = chunk
         .iter()
         .map(u32::to_string)
         .collect::<Vec<_>>()
         .join(",");
      session
         .run_command_and_check_ok(format!("UID FETCH {set} (X-GM-THRID)"))
         .await?;
      while let Ok(unsol) = session.unsolicited_responses.try_recv() {
         let UnsolicitedResponse::Other(data) = unsol else {
            continue;
         };
         let Response::Fetch(_, attrs) = data.parsed() else {
            continue;
         };
         let mut uid = None::<u32>;
         let mut thrid = None::<u64>;
         for attr in attrs {
            match attr {
               AttributeValue::Uid(id) => uid = Some(*id),
               AttributeValue::GmailThrId(tid) => thrid = Some(*tid),
               _ => {},
            }
         }
         if let (Some(id), Some(tid)) = (uid, thrid) {
            map.insert(id, tid);
         }
      }
   }
   Ok(map)
}

/// Case-insensitive capability probe used to select atomic `UID MOVE` when
/// the server supports RFC 6851.
///
/// # Errors
///
/// Returns [`SyncError`] if the `CAPABILITY` command fails.
pub async fn has_capability(session: &mut ImapSession, name: &str) -> Result<bool> {
   use async_imap::types::Capability;

   let caps = session.capabilities().await?;
   Ok(caps
      .iter()
      .any(|cap| matches!(cap, Capability::Atom(atom) if atom.eq_ignore_ascii_case(name))))
}

#[cfg(test)]
mod tests {
   use std::net::SocketAddr;

   use async_imap::types::NameAttribute;
   use futures::future::BoxFuture;
   use tokio::net::TcpListener;

   use super::*;

   #[test]
   fn role_derivation_inbox() {
      assert_eq!(
         ImapFolder::derive_role("INBOX", &[]).as_deref(),
         Some("inbox")
      );
      assert_eq!(
         ImapFolder::derive_role("Inbox", &[]).as_deref(),
         Some("inbox")
      );
   }

   #[test]
   fn role_derivation_gmail_flags() {
      assert_eq!(
         ImapFolder::derive_role("[Gmail]/All Mail", &["all".into()]).as_deref(),
         Some("all")
      );
      assert_eq!(
         ImapFolder::derive_role("[Gmail]/Sent Mail", &["sent".into()]).as_deref(),
         Some("sent")
      );
      assert_eq!(
         ImapFolder::derive_role("[Gmail]/Trash", &["trash".into()]).as_deref(),
         Some("trash")
      );
      assert_eq!(
         ImapFolder::derive_role("[Gmail]/Spam", &["junk".into()]).as_deref(),
         Some("junk")
      );
   }

   #[test]
   fn role_derivation_ignores_flagged() {
      assert_eq!(
         ImapFolder::derive_role("[Gmail]/Starred", &["flagged".into()]),
         None
      );
   }

   #[test]
   fn typed_special_use_attributes_are_preserved() {
      for (attribute, expected) in [
         (NameAttribute::All, "all"),
         (NameAttribute::Archive, "archive"),
         (NameAttribute::Drafts, "drafts"),
         (NameAttribute::Flagged, "flagged"),
         (NameAttribute::Junk, "junk"),
         (NameAttribute::Sent, "sent"),
         (NameAttribute::Trash, "trash"),
      ] {
         assert_eq!(format_attribute(&attribute).as_deref(), Some(expected));
      }
   }

   async fn mock_server(
      script: impl FnOnce(TcpStream) -> BoxFuture<'static, ()> + Send + 'static,
   ) -> SocketAddr {
      let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
      let addr = listener.local_addr().unwrap();
      tokio::spawn(async move {
         let (stream, _) = listener.accept().await.unwrap();
         script(stream).await;
      });
      addr
   }

   #[tokio::test]
   async fn starttls_negotiation_happy_path() {
      use futures::FutureExt as _;
      use tokio::io::{
         AsyncReadExt as _,
         AsyncWriteExt as _,
      };
      let addr = mock_server(|mut stream| {
         async move {
            stream.write_all(b"* OK ready\r\n").await.unwrap();
            let mut buf = [0_u8; 64];
            let n = stream.read(&mut buf).await.unwrap();
            assert!(buf[..n].starts_with(b"a0 STARTTLS"));
            stream
               .write_all(b"* CAPABILITY IMAP4rev1 STARTTLS\r\na0 OK begin TLS now\r\n")
               .await
               .unwrap();
         }
         .boxed()
      })
      .await;
      let tcp = TcpStream::connect(addr).await.unwrap();
      negotiate_starttls(tcp, "test").await.unwrap();
   }

   #[tokio::test]
   async fn starttls_rejection_is_fatal() {
      use futures::FutureExt as _;
      use tokio::io::{
         AsyncReadExt as _,
         AsyncWriteExt as _,
      };
      let addr = mock_server(|mut stream| {
         async move {
            stream.write_all(b"* OK ready\r\n").await.unwrap();
            let mut buf = [0_u8; 64];
            let _ = stream.read(&mut buf).await.unwrap();
            stream
               .write_all(b"a0 NO TLS unavailable\r\n")
               .await
               .unwrap();
         }
         .boxed()
      })
      .await;
      let tcp = TcpStream::connect(addr).await.unwrap();
      let err = negotiate_starttls(tcp, "test").await.unwrap_err();
      assert!(err.to_string().contains("rejected STARTTLS"), "{err}");
   }
}
