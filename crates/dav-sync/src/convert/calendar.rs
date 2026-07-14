use calcard::{
   Entry,
   Parser,
   icalendar::{
      ICalendar,
      ICalendarComponentType,
      ICalendarProperty,
      ICalendarValue,
   },
   jscalendar::{
      JSCalendar,
      import::ConversionOptions,
   },
};
use serde_json::Value;

use crate::error::{
   ConvertError,
   Result,
};

/// # Errors
///
/// Returns a conversion error when the iCalendar resource is invalid or does
/// not describe an event.
#[inline]
pub fn ical_to_event(input: &str) -> Result<(String, Value), ConvertError> {
   let calendar = parse_calendar(input)?;
   validate_calendar(&calendar)?;

   let converted = calendar
      .into_jscalendar_with_opt::<String, String>(ConversionOptions::default().return_first(true));
   let mut event =
      serde_json::to_value(converted).map_err(|error| invalid("JSCalendar", &error.to_string()))?;
   let object = event
      .as_object_mut()
      .ok_or_else(|| invalid("JSCalendar", "conversion did not produce an object"))?;
   if object.get("@type").and_then(Value::as_str) != Some("Event") {
      return Err(invalid(
         "JSCalendar",
         "iCalendar resource did not convert to an Event",
      ));
   }
   object.insert("version".into(), Value::String("2.0".into()));
   let uid = required_string(object, "uid", "JSCalendar Event")?.to_owned();
   required_string(object, "start", "JSCalendar Event")?;
   Ok((uid, event))
}

/// # Errors
///
/// Returns a conversion error when the `JSCalendar` value is invalid or cannot
/// be converted to iCalendar.
#[inline]
pub fn event_to_ical(event: &Value) -> Result<String, ConvertError> {
   let mut event_value = event.clone();
   let object = event_value
      .as_object_mut()
      .ok_or_else(|| invalid("JSCalendar", "Event must be an object"))?;
   if object.get("@type").and_then(Value::as_str) != Some("Event") {
      return Err(invalid("JSCalendar", "@type must be Event"));
   }
   required_string(object, "uid", "JSCalendar Event")?;
   required_string(object, "start", "JSCalendar Event")?;
   object.remove("version");

   let group = serde_json::json!({ "@type": "Group", "entries": [event_value] });
   let json =
      serde_json::to_string(&group).map_err(|error| invalid("JSCalendar", &error.to_string()))?;
   let calendar = JSCalendar::<String, String>::parse(&json)
      .map_err(|error| invalid("JSCalendar", &error))?
      .into_icalendar()
      .ok_or_else(|| invalid("JSCalendar", "Event could not be converted to iCalendar"))?;
   Ok(calendar.to_string())
}

/// # Errors
///
/// Returns a conversion error when the iCalendar resource cannot be parsed or
/// expanded.
#[inline]
pub fn expand_event_occurrences(
   input: &str,
   default_timezone: chrono_tz::Tz,
   requested_limit: usize,
) -> Result<(Vec<(i64, i64)>, bool), ConvertError> {
   let calendar = parse_calendar(input)?;
   validate_calendar(&calendar)?;
   let limit = requested_limit.max(1);
   let expansion_limit = limit.saturating_add(1);
   let rules_complete = calendar
      .components
      .iter()
      .flat_map(|component| &component.entries)
      .filter(|entry| matches!(entry.name, ICalendarProperty::Rrule))
      .flat_map(|entry| &entry.values)
      .filter_map(|value| {
         match value {
            &ICalendarValue::RecurrenceRule(ref rule) => Some(rule),
            _ => None,
         }
      })
      .all(|rule| {
         rule
            .count
            .is_some_and(|count| count as usize <= expansion_limit)
      });
   let expanded = calendar.expand_dates(default_timezone, expansion_limit);
   if !expanded.errors.is_empty() {
      return Err(invalid(
         "iCalendar",
         &expanded
            .errors
            .iter()
            .map(|error| format!("component {}: {}", error.comp_id, error.error))
            .collect::<Vec<_>>()
            .join(", "),
      ));
   }
   let mut occurrences = expanded
      .events
      .into_iter()
      .filter(|event| {
         calendar
            .components
            .get(event.comp_id as usize)
            .is_some_and(|component| {
               matches!(component.component_type, ICalendarComponentType::VEvent)
            })
      })
      .map(|event| event.timestamps())
      .collect::<Vec<_>>();
   let truncated = !rules_complete || occurrences.len() > limit;
   occurrences.truncate(limit);
   Ok((occurrences, truncated))
}

fn parse_calendar(input: &str) -> Result<ICalendar, ConvertError> {
   // calcard strict mode rejects nested components; validate the shape below.
   let mut parser = Parser::new(input);
   let calendar = match parser.entry() {
      Entry::ICalendar(calendar) => calendar,
      entry => return Err(invalid("iCalendar", &format!("{entry:?}"))),
   };
   match parser.entry() {
      Entry::Eof => Ok(calendar),
      entry => {
         Err(invalid(
            "iCalendar",
            &format!("multiple top-level components: {entry:?}"),
         ))
      },
   }
}

fn validate_calendar(calendar: &ICalendar) -> Result<(), ConvertError> {
   let root = calendar
      .components
      .first()
      .ok_or_else(|| invalid("iCalendar", "empty resource"))?;
   if !matches!(&root.component_type, ICalendarComponentType::VCalendar) {
      return Err(invalid("iCalendar", "root component must be VCALENDAR"));
   }

   let events = root
      .component_ids
      .iter()
      .filter_map(|id| calendar.components.get(*id as usize))
      .filter(|component| matches!(&component.component_type, ICalendarComponentType::VEvent))
      .collect::<Vec<_>>();
   let masters = events
      .iter()
      .copied()
      .filter(|component| {
         component
            .property(&ICalendarProperty::RecurrenceId)
            .is_none()
      })
      .collect::<Vec<_>>();
   if masters.len() != 1 {
      return Err(invalid(
         "iCalendar",
         &format!("expected one master VEVENT, found {}", masters.len()),
      ));
   }

   let master = masters[0];
   let uid = master
      .uid()
      .ok_or_else(|| invalid("iCalendar", "VEVENT is missing UID"))?;
   if master.property(&ICalendarProperty::Dtstart).is_none() {
      return Err(invalid("iCalendar", "VEVENT is missing DTSTART"));
   }
   if events
      .iter()
      .any(|component| component.uid().is_some_and(|event_uid| event_uid != uid))
   {
      return Err(invalid(
         "iCalendar",
         "recurrence overrides must use the master UID",
      ));
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

   const EVENT: &str = concat!(
      "BEGIN:VCALENDAR\r\n",
      "VERSION:2.0\r\n",
      "PRODID:-//jmapper//EN\r\n",
      "BEGIN:VEVENT\r\n",
      "UID:event-1\r\n",
      "DTSTART;TZID=Europe/Berlin:20260715T100000\r\n",
      "DTEND;TZID=Europe/Berlin:20260715T110000\r\n",
      "SUMMARY:Planning\r\n",
      "X-TEST:kept\r\n",
      "END:VEVENT\r\n",
      "END:VCALENDAR\r\n",
   );

   #[test]
   fn converts_and_round_trips_event() {
      let (uid, event) = ical_to_event(EVENT).unwrap();
      assert_eq!(uid, "event-1");
      assert_eq!(event["@type"], "Event");
      assert_eq!(event["version"], "2.0");
      assert_eq!(event["title"], "Planning");

      let raw = event_to_ical(&event).unwrap();
      assert!(raw.contains("BEGIN:VCALENDAR"));
      assert!(raw.contains("X-TEST:kept"));
      let (uid, reparsed) = ical_to_event(&raw).unwrap();
      assert_eq!(uid, "event-1");
      assert_eq!(reparsed["title"], "Planning");
   }

   #[test]
   fn recurrence_overrides_survive() {
      let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:r1\r\nDTSTART:\
                 20260715T100000Z\r\nDURATION:PT1H\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\nEXDATE:\
                 20260722T100000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:r1\r\nRECURRENCE-ID:\
                 20260729T100000Z\r\nDTSTART:20260729T120000Z\r\nDURATION:PT1H\r\nEND:VEVENT\r\\
                 nEND:VCALENDAR\r\n";
      let (_, event) = ical_to_event(raw).unwrap();
      assert!(event["recurrenceRule"].is_object());
      assert!(event["recurrenceOverrides"].is_object());
      let output = event_to_ical(&event).unwrap();
      assert!(output.contains("RRULE:"));
      assert!(output.contains("RECURRENCE-ID"));
   }

   #[test]
   fn expands_recurrence_dates() {
      let raw = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:r1\r\nDTSTART:\
                 20260702T090000Z\r\nDURATION:PT1H\r\nRRULE:FREQ=WEEKLY;COUNT=3\r\nEXDATE:\
                 20260709T090000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
      let (dates, truncated) = expand_event_occurrences(raw, chrono_tz::UTC, 100).unwrap();
      assert!(!truncated);
      assert_eq!(dates.len(), 2);
      assert_eq!(dates[1].1 - dates[1].0, 3600);
   }

   #[test]
   fn rejects_non_event_and_missing_uid() {
      ical_to_event("BEGIN:VCARD\r\nVERSION:4.0\r\nEND:VCARD\r\n").unwrap_err();
      ical_to_event(
         "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nDTSTART:20260715T100000Z\r\nEND:\
          VEVENT\r\nEND:VCALENDAR\r\n",
      )
      .unwrap_err();
   }
}
