use std::time::Duration;

use mail_send::{
   Credentials,
   SmtpClient,
   SmtpClientBuilder,
};
use tokio::{
   io::{
      AsyncRead,
      AsyncWrite,
   },
   time,
};

use crate::{
   error::{
      Result,
      SyncError,
   },
   imap,
   provider::ImapTls,
};

pub enum SmtpAuth {
   Plain {
      username: String,
      password: String,
   },
   XOAuth2 {
      email:        String,
      access_token: String,
   },
}

pub struct SmtpParams {
   pub host: String,
   pub port: u16,
   pub tls:  ImapTls,
   pub auth: SmtpAuth,
}

/// # Errors
///
/// Returns [`SyncError`] if the envelope addresses are malformed, the SMTP TLS
/// setup or connection fails, or the server rejects the message during
/// transmission.
pub async fn submit(
   params: &SmtpParams,
   mail_from: &str,
   rcpt_to: &[String],
   body: &[u8],
) -> Result<String> {
   validate_envelope(mail_from, rcpt_to)?;

   let credentials = match &params.auth {
      SmtpAuth::Plain { username, password } => {
         Credentials::new(username.clone(), password.clone())
      },
      SmtpAuth::XOAuth2 {
         email,
         access_token,
      } => Credentials::new_xoauth2(email.clone(), access_token.clone()),
   };
   let mut builder = SmtpClientBuilder::new(params.host.clone(), params.port)
      .map_err(|error| SyncError::Tls(format!("smtp TLS setup: {error}")))?
      .implicit_tls(matches!(params.tls, ImapTls::Implicit))
      .credentials(credentials)
      .helo_host("jmapper")
      .timeout(Duration::from_secs(60));
   builder.tls_connector = tokio_rustls::TlsConnector::from(imap::smtp_rustls_config());

   let mut client = builder.connect().await?;
   let reply = transmit(&mut client, mail_from, rcpt_to, body).await?;
   let _ = client.quit().await;
   Ok(reply)
}

async fn transmit<T>(
   client: &mut SmtpClient<T>,
   mail_from: &str,
   rcpt_to: &[String],
   body: &[u8],
) -> Result<String>
where
   T: AsyncRead + AsyncWrite + Unpin,
{
   let parameters = mail_send::smtp::message::Parameters::default();
   client.mail_from(mail_from, &parameters).await?;
   for recipient in rcpt_to {
      client.rcpt_to(recipient, &parameters).await?;
   }

   let data = client.cmd(b"DATA\r\n").await?;
   if data.code() != 354 {
      return Err(SyncError::Other(format!("smtp DATA failed: {data}")));
   }

   let body = body.strip_suffix(b"\r\n").unwrap_or(body);
   let response = time::timeout(client.timeout, async {
      client.write_message(body).await?;
      client.read().await
   })
   .await
   .map_err(|_| SyncError::Other("smtp DATA timed out".into()))??;
   if !(200..300).contains(&response.code()) {
      return Err(SyncError::Other(format!(
         "smtp message rejected after DATA: {response}"
      )));
   }

   let enhanced = response.esc;
   if enhanced == [0, 0, 0] {
      Ok(format!("{} {}", response.code(), response.message()))
   } else {
      Ok(format!(
         "{} {}.{}.{} {}",
         response.code(),
         enhanced[0],
         enhanced[1],
         enhanced[2],
         response.message()
      ))
   }
}

fn validate_envelope(mail_from: &str, rcpt_to: &[String]) -> Result<()> {
   validate_addr(mail_from)?;
   if rcpt_to.is_empty() {
      return Err(SyncError::Other("submission has no recipients".into()));
   }
   rcpt_to
      .iter()
      .try_for_each(|address| validate_addr(address))
}

fn validate_addr(address: &str) -> Result<()> {
   let valid = !address.is_empty()
      && address.len() <= 254
      && address.contains('@')
      && !address
         .chars()
         .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | ' '));
   if valid {
      Ok(())
   } else {
      Err(SyncError::Other(format!(
         "invalid envelope address {address:?}"
      )))
   }
}

#[cfg(test)]
mod tests {
   use tokio::io::{
      self,
      AsyncBufReadExt as _,
      AsyncWriteExt as _,
      BufReader,
   };

   use super::*;

   #[test]
   fn validates_envelope() {
      validate_envelope("a@example.com", &["b@example.com".into()]).unwrap();
      assert!(validate_envelope("", &["b@example.com".into()]).is_err());
      assert!(validate_envelope("a@example.com", &[]).is_err());
      assert!(validate_addr("a@b>\r\nRCPT TO:<evil@x>").is_err());
      assert!(validate_addr("sp ace@example.com").is_err());
   }

   #[tokio::test]
   async fn transmits_and_returns_data_reply() {
      let (stream, server) = io::duplex(16 * 1024);
      let server = tokio::spawn(async move {
         let mut server = BufReader::new(server);
         let mut line = String::new();

         server.read_line(&mut line).await.unwrap();
         assert_eq!(line, "MAIL FROM:<a@example.com>\r\n");
         server
            .get_mut()
            .write_all(b"250 sender ok\r\n")
            .await
            .unwrap();
         line.clear();
         server.read_line(&mut line).await.unwrap();
         assert_eq!(line, "RCPT TO:<b@example.com>\r\n");
         server
            .get_mut()
            .write_all(b"250 recipient ok\r\n")
            .await
            .unwrap();
         line.clear();
         server.read_line(&mut line).await.unwrap();
         assert_eq!(line, "DATA\r\n");
         server
            .get_mut()
            .write_all(b"354 continue\r\n")
            .await
            .unwrap();

         let mut message = String::new();
         loop {
            line.clear();
            server.read_line(&mut line).await.unwrap();
            if line == ".\r\n" {
               break;
            }
            message.push_str(&line);
         }
         assert_eq!(message, "Subject: hi\r\n\r\n..dots\r\n");
         server
            .get_mut()
            .write_all(b"250 2.0.0 queued\r\n")
            .await
            .unwrap();
      });

      let mut client = SmtpClient {
         stream,
         timeout: Duration::from_secs(1),
      };
      let reply = transmit(
         &mut client,
         "a@example.com",
         &["b@example.com".into()],
         b"Subject: hi\r\n\r\n.dots\r\n",
      )
      .await
      .unwrap();
      assert_eq!(reply, "250 2.0.0 queued");
      server.await.unwrap();
   }
}
