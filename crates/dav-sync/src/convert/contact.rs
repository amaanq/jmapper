use calcard::{
   Entry,
   Parser,
   jscontact::JSContact,
   vcard::{
      VCard,
      VCardProperty,
      VCardVersion,
   },
};
use serde_json::Value;

use crate::error::ConvertError;

/// # Errors
///
/// Returns a conversion error when the vCard is invalid or does not describe
/// a contact card.
#[inline]
pub fn vcard_to_card(input: &str) -> Result<(String, Value), ConvertError> {
   let card = parse_vcard(input)?;
   validate_vcard(&card)?;
   let uid = card
      .uid()
      .ok_or_else(|| invalid("vCard", "validated card is missing UID"))?
      .to_owned();
   let converted = card.into_jscontact::<String, String>();
   let mut value =
      serde_json::to_value(converted).map_err(|error| invalid("JSContact", &error.to_string()))?;
   let object = value
      .as_object_mut()
      .ok_or_else(|| invalid("JSContact", "conversion did not produce an object"))?;
   if object.get("@type").and_then(Value::as_str) != Some("Card") {
      return Err(invalid(
         "JSContact",
         "vCard resource did not convert to a Card",
      ));
   }
   object.insert("version".into(), Value::String("2.0".into()));
   required_string(object, "uid", "JSContact Card")?;
   Ok((uid, value))
}

/// # Errors
///
/// Returns a conversion error when the `JSContact` value is invalid or cannot
/// be converted to a vCard.
#[inline]
pub fn card_to_vcard(card: &Value) -> Result<String, ConvertError> {
   let mut card_value = card.clone();
   let object = card_value
      .as_object_mut()
      .ok_or_else(|| invalid("JSContact", "Card must be an object"))?;
   if object.get("@type").and_then(Value::as_str) != Some("Card") {
      return Err(invalid("JSContact", "@type must be Card"));
   }
   required_string(object, "uid", "JSContact Card")?;
   object.insert("version".into(), Value::String("1.0".into()));

   let json = serde_json::to_string(&card_value)
      .map_err(|error| invalid("JSContact", &error.to_string()))?;
   let parsed_card = JSContact::<String, String>::parse(&json)
      .map_err(|error| invalid("JSContact", &error))?
      .into_vcard()
      .ok_or_else(|| invalid("JSContact", "Card could not be converted to vCard"))?;
   let mut output = String::new();
   parsed_card
      .write_to(&mut output, VCardVersion::V4_0)
      .map_err(|error| invalid("vCard", &error.to_string()))?;
   Ok(output)
}

fn parse_vcard(input: &str) -> Result<VCard, ConvertError> {
   let mut parser = Parser::new(input).strict();
   let card = match parser.entry() {
      Entry::VCard(card) => card,
      entry => return Err(invalid("vCard", &format!("{entry:?}"))),
   };
   match parser.entry() {
      Entry::Eof => Ok(card),
      entry => {
         Err(invalid(
            "vCard",
            &format!("multiple top-level components: {entry:?}"),
         ))
      },
   }
}

fn validate_vcard(card: &VCard) -> Result<(), ConvertError> {
   if !matches!(
      card.version(),
      Some(VCardVersion::V3_0 | VCardVersion::V4_0)
   ) {
      return Err(invalid("vCard", "VERSION must be 3.0 or 4.0"));
   }
   if card.uid().is_none_or(str::is_empty) {
      return Err(invalid("vCard", "VCARD is missing UID"));
   }
   if card.property(&VCardProperty::Fn).is_none() {
      return Err(invalid("vCard", "VCARD is missing FN"));
   }
   Ok(())
}

fn required_string<'a>(
   object: &'a serde_json::Map<String, Value>,
   property: &str,
   format: &'static str,
) -> Result<&'a str, ConvertError> {
   object
      .get(property)
      .and_then(Value::as_str)
      .filter(|value| !value.is_empty())
      .ok_or_else(|| invalid(format, &format!("missing string property {property}")))
}

fn invalid(format: &'static str, reason: &str) -> ConvertError {
   ConvertError::Invalid {
      format,
      reason: reason.to_owned(),
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   const CARD: &str = concat!(
      "BEGIN:VCARD\r\n",
      "VERSION:4.0\r\n",
      "UID:card-1\r\n",
      "FN:Ada Lovelace\r\n",
      "N:Lovelace;Ada;;;\r\n",
      "EMAIL;TYPE=work:ada@example.test\r\n",
      "X-TEST:kept\r\n",
      "END:VCARD\r\n",
   );

   #[test]
   fn converts_and_round_trips_card() {
      let (uid, card) = vcard_to_card(CARD).unwrap();
      assert_eq!(uid, "card-1");
      assert_eq!(card["@type"], "Card");
      assert_eq!(card["uid"], "card-1");

      let raw = card_to_vcard(&card).unwrap();
      assert!(raw.contains("VERSION:4.0"));
      assert!(raw.contains("X-TEST:kept"));
      let (uid, reparsed) = vcard_to_card(&raw).unwrap();
      assert_eq!(uid, "card-1");
      assert_eq!(reparsed["uid"], "card-1");
   }

   #[test]
   fn accepts_vcard_three_and_writes_four() {
      let input = CARD.replace("VERSION:4.0", "VERSION:3.0");
      let (_, card) = vcard_to_card(&input).unwrap();
      assert!(card_to_vcard(&card).unwrap().contains("VERSION:4.0"));
   }

   #[test]
   fn rejects_legacy_and_malformed_cards() {
      vcard_to_card(&CARD.replace("VERSION:4.0", "VERSION:2.1")).unwrap_err();
      vcard_to_card(&CARD.replace("UID:card-1\r\n", "")).unwrap_err();
      vcard_to_card("BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n").unwrap_err();
   }
}
