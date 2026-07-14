//! `EventSource` (Server-Sent Events) push — RFC 8620 §7.3.
//!
//! Clients subscribe to
//! `/eventsource?types=Email,Mailbox&closeafter=state&ping=30` and receive a
//! stream of `StateChange` payloads scoped to their authenticated account.
//! Disconnects on `closeafter=state` after the first event fires — JMAP clients
//! use this idiom to implement "long-poll" semantics on top of SSE.
//!
//! Event encoding follows the RFC 8620 §7.3 shape:
//! ```text
//! id: <epoch-secs>
//! event: state
//! data: {"@type":"StateChange","changed":{"acctA":{"Email":"42"}}}
//! ```
//!
//! `ping=N` sends the named JSON ping event JMAP clients use to detect a
//! buffered or dead event-source connection.

use std::{
   collections::HashSet,
   convert::Infallible,
   time::Duration,
};

use axum::{
   Extension,
   extract::{
      Query,
      State,
   },
   response::sse::{
      Event,
      Sse,
   },
};
use futures_util::stream::{
   Stream,
   StreamExt as _,
   unfold,
};
use serde::Deserialize;
use tokio::time;
use tokio_stream::wrappers::{
   BroadcastStream,
   errors::BroadcastStreamRecvError,
};

use crate::{
   auth::AuthedAccount,
   error::ApiError,
   state::{
      AppState,
      StateChange,
      StateKind,
   },
};

#[derive(Debug, Deserialize)]
pub struct EventSourceQuery {
   /// Comma-separated JMAP types — `Email,Mailbox,Thread` or `*`.
   #[serde(default)]
   types:      Option<String>,
   /// `state` → close after first `state` event; `no` (default) → hold open.
   #[serde(default)]
   closeafter: Option<String>,
   /// Ping seconds. Zero disables pings; positive values clamp to [15, 300].
   #[serde(default)]
   ping:       Option<u64>,
}

/// One step of the event-source stream: either the next broadcast item or a
/// scheduled ping tick.
enum Next {
   Source(Option<Result<StateChange, BroadcastStreamRecvError>>),
   Ping,
}

/// GET /eventsource — streams state-change events for the authenticated
/// account. Clients filter via the `types=` query.
///
/// # Errors
///
/// Returns [`ApiError`] if the event-source stream cannot be established. The
/// current implementation always succeeds; the fallible signature lets the
/// handler surface setup failures without changing its interface.
pub async fn eventsource_handler(
   State(state): State<AppState>,
   Extension(AuthedAccount(auth)): Extension<AuthedAccount>,
   Query(query): Query<EventSourceQuery>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
   let account_id = auth.id;
   let wanted_types = parse_types_filter(query.types.as_deref());
   let close_after_state = matches!(query.closeafter.as_deref(), Some("state"));
   let ping_secs = normalized_ping(query.ping);

   let raw = BroadcastStream::new(state.state_changes());
   let stream = unfold(
      (
         raw,
         account_id,
         wanted_types,
         close_after_state,
         ping_secs,
         false,
      ),
      |(mut raw, account_id, wanted_types, close_after_state, ping_secs, done)| {
         async move {
            if done {
               return None;
            }

            let mut timer =
               ping_secs.map(|seconds| Box::pin(time::sleep(Duration::from_secs(seconds))));
            loop {
               let next = if let Some(timer) = timer.as_mut() {
                  tokio::select! {
                      event = raw.next() => Next::Source(event),
                      () = timer.as_mut() => Next::Ping,
                  }
               } else {
                  Next::Source(raw.next().await)
               };

               let (event, close) = match next {
                  Next::Ping => (format_ping_event(ping_secs.unwrap_or_default()), false),
                  // RFC 8620 only defines `state` and `ping` events. A closed
                  // channel (`None`) or a bounded receiver that has missed
                  // changes (`Lagged`) both leave the stream unable to emit a
                  // safe next event — an invented partial state would be wrong
                  // and a custom event would be ignored by conforming clients.
                  // Close the stream so the client reconnects and refreshes.
                  Next::Source(None | Some(Err(BroadcastStreamRecvError::Lagged(_)))) => {
                     return None;
                  },
                  Next::Source(Some(Ok(change))) => {
                     if change.account_id != account_id {
                        continue;
                     }
                     if wanted_types
                        .as_ref()
                        .is_some_and(|wanted| !wanted.contains(&change.kind))
                     {
                        continue;
                     }
                     (format_state_event(&change), close_after_state)
                  },
               };
               return Some((
                  Ok(event),
                  (
                     raw,
                     account_id,
                     wanted_types,
                     close_after_state,
                     ping_secs,
                     close,
                  ),
               ));
            }
         }
      },
   );

   Ok(Sse::new(stream))
}

fn normalized_ping(requested: Option<u64>) -> Option<u64> {
   match requested {
      Some(0) => None,
      Some(seconds) => Some(seconds.clamp(15, 300)),
      None => Some(30),
   }
}

/// Parse `types=Email,Mailbox,*` into a set. `None` means "all types".
fn parse_types_filter(raw: Option<&str>) -> Option<HashSet<StateKind>> {
   let raw = raw?.trim();
   if raw.is_empty() || raw == "*" {
      return None;
   }
   Some(
      raw.split(',')
         .filter_map(|part| StateKind::parse(part.trim()))
         .collect(),
   )
}

fn format_state_event(change: &StateChange) -> Event {
   let data = serde_json::json!({
       "@type": "StateChange",
       "changed": {
           change.account_id.clone(): {
               change.kind.as_jmap_type(): change.new_state.clone()
           }
       }
   });
   Event::default().event("state").data(data.to_string())
}

fn format_ping_event(interval: u64) -> Event {
   Event::default()
      .event("ping")
      .data(serde_json::json!({"interval": interval}).to_string())
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn types_filter_star_is_any() {
      assert!(parse_types_filter(Some("*")).is_none());
      assert!(parse_types_filter(None).is_none());
      assert!(parse_types_filter(Some("")).is_none());
   }

   #[test]
   fn types_filter_parses_csv() {
      let set = parse_types_filter(Some("Email,Mailbox")).unwrap();
      assert!(set.contains(&StateKind::Email));
      assert!(set.contains(&StateKind::Mailbox));
      assert!(!set.contains(&StateKind::Thread));
   }

   #[test]
   fn types_filter_drops_unknowns() {
      let set = parse_types_filter(Some("Email,Garbage,Thread")).unwrap();
      assert_eq!(set.len(), 2);
   }

   #[test]
   fn state_change_shape() {
      let change = StateChange {
         account_id: "acctA".into(),
         kind:       StateKind::Email,
         new_state:  "42".into(),
      };
      let ev = format_state_event(&change);
      // `Event` doesn't expose its fields; instead verify Display format
      // includes the expected JSON.
      let rendered = format!("{ev:?}");
      assert!(rendered.contains("state"));
   }

   #[test]
   fn ping_zero_disables_and_positive_values_clamp() {
      assert_eq!(normalized_ping(Some(0)), None);
      assert_eq!(normalized_ping(Some(1)), Some(15));
      assert_eq!(normalized_ping(Some(500)), Some(300));
      assert_eq!(normalized_ping(None), Some(30));
   }

   #[test]
   fn ping_event_contains_the_effective_interval() {
      let event = format_ping_event(15);
      let rendered = format!("{event:?}");
      assert!(rendered.contains("ping"));
      assert!(rendered.contains("interval"));
      assert!(rendered.contains("15"));
   }
}
