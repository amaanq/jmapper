use chrono::{
   DateTime,
   Utc,
};
use jmap_protocol::email::EmailAddress;
use mail_builder::{
   MessageBuilder,
   headers::{
      address::Address,
      content_type::ContentType,
   },
   mime::MimePart,
};

pub struct ComposeInput {
   pub from:        Vec<EmailAddress>,
   pub to:          Vec<EmailAddress>,
   pub cc:          Vec<EmailAddress>,
   pub bcc:         Vec<EmailAddress>,
   pub reply_to:    Vec<EmailAddress>,
   pub subject:     Option<String>,
   pub in_reply_to: Vec<String>,
   pub references:  Vec<String>,
   pub sent_at:     DateTime<Utc>,
   pub message_id:  String,
   pub text_body:   Option<String>,
   pub html_body:   Option<String>,
   pub attachments: Vec<Attachment>,
}

pub struct Attachment {
   pub bytes:        Vec<u8>,
   pub content_type: String,
   pub name:         Option<String>,
}

/// Serializes a [`ComposeInput`] into an RFC 5322 message.
///
/// # Panics
///
/// Panics if writing the message to the in-memory buffer fails, which cannot
/// happen for a `Vec` sink.
#[must_use]
pub fn build_rfc5322(input: &ComposeInput) -> Vec<u8> {
   let mut message = MessageBuilder::new()
      .date(input.sent_at.timestamp())
      .message_id(clean_id(&input.message_id));

   if !input.from.is_empty() {
      message = message.from(addresses(&input.from));
   }
   if !input.to.is_empty() {
      message = message.to(addresses(&input.to));
   }
   if !input.cc.is_empty() {
      message = message.cc(addresses(&input.cc));
   }
   if !input.bcc.is_empty() {
      message = message.bcc(addresses(&input.bcc));
   }
   if !input.reply_to.is_empty() {
      message = message.reply_to(addresses(&input.reply_to));
   }
   if let Some(subject) = &input.subject {
      message = message.subject(clean_text(subject));
   }
   if !input.in_reply_to.is_empty() {
      message = message.in_reply_to(clean_ids(&input.in_reply_to));
   }
   if !input.references.is_empty() {
      message = message.references(clean_ids(&input.references));
   }
   if let Some(text) = &input.text_body {
      message = message.text_body(text.as_str());
   }
   if let Some(html) = &input.html_body {
      message = message.html_body(html.as_str());
   }

   for attachment in &input.attachments {
      let part = MimePart::new(
         clean_content_type(&attachment.content_type),
         attachment.bytes.as_slice(),
      );
      let part = match attachment.name.as_deref() {
         Some(name) => part.attachment(clean_text(name)),
         None => part.header("Content-Disposition", ContentType::new("attachment")),
      };
      message.attachments.get_or_insert_default().push(part);
   }

   message
      .write_to_vec()
      .expect("writing an RFC 5322 message to memory cannot fail")
}

fn addresses(values: &[EmailAddress]) -> Address<'static> {
   Address::new_list(
      values
         .iter()
         .map(|address| {
            Address::new_address(
               address.name.as_deref().map(clean_text),
               address
                  .email
                  .chars()
                  .filter(|ch| !matches!(ch, '\r' | '\n' | '<' | '>'))
                  .collect::<String>(),
            )
         })
         .collect(),
   )
}

fn clean_ids(values: &[String]) -> Vec<String> {
   values.iter().map(|value| clean_id(value)).collect()
}

fn clean_id(value: &str) -> String {
   value
      .chars()
      .filter(|ch| !matches!(ch, '\r' | '\n' | '<' | '>'))
      .collect()
}

fn clean_text(value: &str) -> String {
   value.replace(['\r', '\n'], " ")
}

fn clean_content_type(value: &str) -> String {
   let value = value.trim();
   if value.contains('/')
      && value.bytes().all(|byte| {
         byte.is_ascii_alphanumeric()
            || matches!(
               byte,
               b'/' | b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
            )
      })
   {
      value.to_ascii_lowercase()
   } else {
      "application/octet-stream".to_owned()
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   fn addr(name: Option<&str>, email: &str) -> EmailAddress {
      EmailAddress {
         name:  name.map(str::to_owned),
         email: email.to_owned(),
      }
   }

   fn input() -> ComposeInput {
      ComposeInput {
         from:        vec![addr(Some("Alice"), "alice@example.com")],
         to:          vec![addr(None, "bob@example.com")],
         cc:          vec![],
         bcc:         vec![],
         reply_to:    vec![],
         subject:     Some("Hello".into()),
         in_reply_to: vec![],
         references:  vec![],
         sent_at:     DateTime::from_timestamp(1_752_000_000, 0).unwrap(),
         message_id:  "test-1@jmapper".into(),
         text_body:   Some("Hi Bob!\n".into()),
         html_body:   None,
         attachments: vec![],
      }
   }

   #[test]
   fn builds_message_and_multipart_body() {
      let mut input = input();
      input.subject = Some("Grüße aus Köln".into());
      input.html_body = Some("<p>Hi <b>Bob</b>!</p>".into());
      input.attachments.push(Attachment {
         bytes:        vec![0, 159, 146, 150],
         content_type: "application/octet-stream".into(),
         name:         Some("blob.bin".into()),
      });

      let bytes = build_rfc5322(&input);
      let parsed = mail_parser::MessageParser::default().parse(&bytes).unwrap();
      assert_eq!(parsed.subject(), Some("Grüße aus Köln"));
      assert_eq!(parsed.message_id(), Some("test-1@jmapper"));
      assert_eq!(parsed.body_text(0).as_deref(), Some("Hi Bob!\r\n"));
      assert_eq!(
         parsed.body_html(0).as_deref(),
         Some("<p>Hi <b>Bob</b>!</p>")
      );
      assert_eq!(parsed.attachment(0).unwrap().contents(), &[
         0, 159, 146, 150
      ]);
   }

   #[test]
   fn neutralizes_header_injection() {
      let mut input = input();
      input.subject = Some("legit\r\nBcc: evil@example.com".into());
      input.to = vec![addr(
         Some("name\r\nX-Inject: 1"),
         "b@example.com>\r\nRCPT: x",
      )];

      let bytes = build_rfc5322(&input);
      let parsed = mail_parser::MessageParser::default().parse(&bytes).unwrap();
      assert!(parsed.header("Bcc").is_none());
      assert!(parsed.header("X-Inject").is_none());
      assert!(parsed.header("RCPT").is_none());
   }

   #[test]
   fn preserves_threading_headers() {
      let mut input = input();
      input.in_reply_to = vec!["parent@example.com".into()];
      input.references = vec!["root@example.com".into(), "parent@example.com".into()];

      let text = String::from_utf8(build_rfc5322(&input)).unwrap();
      assert!(text.contains("In-Reply-To: <parent@example.com>\r\n"));
      assert!(text.contains("References: <root@example.com> <parent@example.com>\r\n"));
   }
}
