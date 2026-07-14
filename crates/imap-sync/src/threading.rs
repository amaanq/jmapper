//! RFC 5322 threading (JMAP `Thread` object, RFC 8621 §3).
//!
//! Mirrors Perl's `JMAP/ImapDB.pm:830` `calcmsgid` heuristic:
//!
//! 1. **In-Reply-To / References lookup.** For every referenced message-id,
//!    consult `thread_index`. First hit wins (hits are stable: once a
//!    Message-ID is bound to a thrid we never rebind it).
//! 2. **Normalized-subject fallback.** Strip leading `Re:`/`Fwd:` (any case,
//!    any locale within reason), collapse whitespace, lowercase. Look up
//!    `thread_by_subject`; if a thread with that normalized subject received a
//!    message recently, join it.
//! 3. **New thrid.** `t-{sha1(message_id||account||received_at)}`.
//!
//! On servers advertising `X-GM-EXT-1`, the sync loop short-circuits all of
//! this with Gmail's `X-GM-THRID` (see `imap::fetch_gmail_thrids`) and calls
//! [`record_known_thrid`] so the RFC 5322 path joins the same threads for
//! replies that arrive without one.
//!
//! The account task drives this from the sync loop; it never touches IMAP.

use deadpool_postgres::Pool as PgPool;
use jmapper_codegen::queries;
use sha1::{
   Digest as _,
   Sha1,
};

use crate::{
   db,
   error::Result,
};

/// Inputs to the thrid derivation. Message-ID and subject are optional
/// because malformed mails in the wild sometimes omit them; we still need a
/// thrid in those cases.
#[derive(Debug, Clone, Default)]
pub struct ThreadInputs<'a> {
   pub account_id:       &'a str,
   /// Our synthesized msgid for this message (the primary key in `messages`).
   /// Used to mint a fallback thrid if nothing else matches.
   pub msgid:            &'a str,
   /// RFC 5322 Message-ID (angle brackets stripped).
   pub message_id:       Option<&'a str>,
   /// RFC 5322 In-Reply-To (angle brackets stripped).
   pub in_reply_to:      Option<&'a str>,
   /// Whitespace-separated RFC 5322 References header (angle brackets
   /// stripped on each token).
   pub references:       Option<&'a str>,
   pub subject:          Option<&'a str>,
   pub received_at_secs: i64,
}

/// Assign a thrid and record the mapping so future messages in this thread
/// find it. Idempotent: re-running for the same message returns the same
/// thrid as long as the `thread_index` row is still there.
///
/// # Errors
///
/// Returns an error if a database client cannot be checked out of `pool` or
/// any of the threading queries fail.
pub async fn resolve_and_record_thrid(pool: &PgPool, inputs: &ThreadInputs<'_>) -> Result<String> {
   // (1) Look up any referenced Message-ID we've already threaded.
   let refs = references_chain(inputs);
   if !refs.is_empty() {
      let thrid = thrid_from_refs(pool, inputs.account_id, &refs).await?;
      if let Some(thrid) = thrid {
         bind_message_id(pool, inputs.account_id, inputs.message_id, &thrid).await?;
         for reference in &refs {
            bind_message_id(pool, inputs.account_id, Some(reference.as_str()), &thrid).await?;
         }
         record_subject(
            pool,
            inputs.account_id,
            inputs.subject,
            &thrid,
            inputs.received_at_secs,
         )
         .await?;
         return Ok(thrid);
      }
   }

   // (2) Normalized-subject fallback. Only match against a recent thread —
   // "Re: meeting" from 2019 shouldn't swallow a new "Re: meeting" in 2026.
   if let Some(norm) = normalized_subject(inputs.subject)
      && let Some(thrid) =
         thrid_from_subject(pool, inputs.account_id, &norm, inputs.received_at_secs).await?
   {
      bind_message_id(pool, inputs.account_id, inputs.message_id, &thrid).await?;
      record_subject(
         pool,
         inputs.account_id,
         Some(norm.as_str()),
         &thrid,
         inputs.received_at_secs,
      )
      .await?;
      return Ok(thrid);
   }

   // (3) Mint a new thrid.
   let thrid = mint_thrid(inputs);
   bind_message_id(pool, inputs.account_id, inputs.message_id, &thrid).await?;
   record_subject(
      pool,
      inputs.account_id,
      inputs.subject,
      &thrid,
      inputs.received_at_secs,
   )
   .await?;
   Ok(thrid)
}

/// Record an externally-derived thrid (Gmail's X-GM-THRID) for this message.
///
/// Binds the message's own Message-ID, its references chain, and its
/// normalized subject to the given thrid so later messages resolved through
/// the RFC 5322 path — a reply fetched before its Gmail thrid, a non-Gmail
/// account CC'd on the same thread — land in the same thread.
///
/// # Errors
///
/// Returns an error if a database client cannot be checked out of `pool` or
/// any of the threading queries fail.
pub async fn record_known_thrid(
   pool: &PgPool,
   inputs: &ThreadInputs<'_>,
   thrid: &str,
) -> Result<()> {
   bind_message_id(pool, inputs.account_id, inputs.message_id, thrid).await?;
   for reference in references_chain(inputs) {
      bind_message_id(pool, inputs.account_id, Some(reference.as_str()), thrid).await?;
   }
   record_subject(
      pool,
      inputs.account_id,
      inputs.subject,
      thrid,
      inputs.received_at_secs,
   )
   .await?;
   Ok(())
}

fn references_chain(inputs: &ThreadInputs<'_>) -> Vec<String> {
   let mut out = Vec::<String>::new();
   if let Some(irt) = inputs.in_reply_to {
      out.push(strip_angles(irt).to_owned());
   }
   if let Some(refs) = inputs.references {
      for token in refs.split_whitespace() {
         let stripped = strip_angles(token);
         if !stripped.is_empty() {
            out.push(stripped.to_owned());
         }
      }
   }
   out.retain(|entry| !entry.is_empty());
   out
}

fn strip_angles(text: &str) -> &str {
   text.trim().trim_matches(|ch| ch == '<' || ch == '>').trim()
}

async fn thrid_from_refs(
   pool: &PgPool,
   account_id: &str,
   refs: &[String],
) -> Result<Option<String>> {
   // References chains are normally short (<10 entries). Cap pathological
   // headers at 32 to bound query parameters and lookup work.
   let refs = refs.iter().take(32).cloned().collect::<Vec<String>>();
   Ok(queries::threading::thrid_from_refs()
      .bind(&db::client(pool).await?, &account_id, &refs)
      .opt()
      .await?)
}

async fn thrid_from_subject(
   pool: &PgPool,
   account_id: &str,
   norm: &str,
   received_at_secs: i64,
) -> Result<Option<String>> {
   // 30-day window. Subjects like "hi" / "meeting" recur across years;
   // demanding recency keeps false merges rare while catching the common
   // "Re: …" reply whose References/In-Reply-To got lost by an intermediate
   // MUA.
   const RECENT_WINDOW_SECS: i64 = 30 * 24 * 3600;
   let cutoff = received_at_secs - RECENT_WINDOW_SECS;
   Ok(queries::threading::thrid_from_subject()
      .bind(&db::client(pool).await?, &account_id, &norm, &cutoff)
      .opt()
      .await?)
}

async fn bind_message_id(
   pool: &PgPool,
   account_id: &str,
   message_id: Option<&str>,
   thrid: &str,
) -> Result<()> {
   let Some(mid) = message_id else {
      return Ok(());
   };
   let mid = strip_angles(mid);
   if mid.is_empty() {
      return Ok(());
   }
   queries::threading::bind_message_id()
      .bind(&db::client(pool).await?, &account_id, &mid, &thrid)
      .await?;
   Ok(())
}

async fn record_subject(
   pool: &PgPool,
   account_id: &str,
   subject: Option<&str>,
   thrid: &str,
   received_at_secs: i64,
) -> Result<()> {
   let Some(norm) = normalized_subject(subject) else {
      return Ok(());
   };
   queries::threading::record_subject()
      .bind(
         &db::client(pool).await?,
         &account_id,
         &norm.as_str(),
         &thrid,
         &received_at_secs,
      )
      .await?;
   Ok(())
}

fn mint_thrid(inputs: &ThreadInputs<'_>) -> String {
   let mut hasher = Sha1::new();
   hasher.update(inputs.account_id.as_bytes());
   hasher.update(b"\0");
   if let Some(msg) = inputs.message_id {
      hasher.update(strip_angles(msg).as_bytes());
   } else {
      hasher.update(inputs.msgid.as_bytes());
   }
   hasher.update(b"\0");
   hasher.update(inputs.received_at_secs.to_le_bytes());
   let digest = hasher.finalize();
   format!("t-{}", hex::encode(&digest[..10]))
}

/// Normalize a subject for threading.
///
/// Strips leading `Re:` / `Fwd:` tokens (any case, any number of iterations,
/// optional whitespace / brackets), collapses whitespace, and lowercases.
/// Returns `None` if the subject is empty or degenerates to whitespace.
///
/// Mirrors Perl's `_normalize_subject` (not shown here but equivalent behavior
/// across dozens of JMAP interop tests).
#[must_use]
pub fn normalized_subject(raw: Option<&str>) -> Option<String> {
   let mut subject = raw?.to_owned();
   loop {
      let before = subject.clone();
      let trimmed = subject.trim_start().to_owned();
      subject = trimmed;
      if let Some(rest) = strip_reply_fwd_prefix(&subject) {
         subject = rest.to_owned();
         continue;
      }
      if subject == before {
         break;
      }
   }
   let collapsed = subject
      .split_whitespace()
      .collect::<Vec<_>>()
      .join(" ")
      .to_lowercase();
   if collapsed.is_empty() {
      None
   } else {
      Some(collapsed)
   }
}

/// Strip a single `Re:` / `Fwd:` / `Fw:` / bracketed list prefix; returns
/// `None` if no prefix was present. Caller loops until `None`.
fn strip_reply_fwd_prefix(subject: &str) -> Option<&str> {
   for prefix in ["re:", "fwd:", "fw:"] {
      if subject
         .get(..prefix.len())
         .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
      {
         return subject.get(prefix.len()..);
      }
   }
   // Strip a bracketed list tag: `[list-name] …`. Common on mailing lists.
   if let Some(rest) = subject.strip_prefix('[')
      && let Some(end) = rest.find(']')
   {
      return Some(&rest[end + 1..]);
   }
   None
}

#[cfg(test)]
mod tests {
   use super::*;
   use crate::testkit;

   #[test]
   fn normalize_strips_re() {
      assert_eq!(
         normalized_subject(Some("Re: hello")).as_deref(),
         Some("hello")
      );
      assert_eq!(
         normalized_subject(Some("RE: Re: re: meeting")).as_deref(),
         Some("meeting"),
      );
   }

   #[test]
   fn normalize_strips_fwd_variants() {
      assert_eq!(normalized_subject(Some("Fwd: hi")).as_deref(), Some("hi"));
      assert_eq!(normalized_subject(Some("FW: hi")).as_deref(), Some("hi"));
   }

   #[test]
   fn normalize_strips_list_prefix() {
      assert_eq!(
         normalized_subject(Some("[users-list] Re: big topic")).as_deref(),
         Some("big topic"),
      );
   }

   #[test]
   fn normalize_collapses_whitespace() {
      assert_eq!(
         normalized_subject(Some("   Big    news   today  ")).as_deref(),
         Some("big news today"),
      );
   }

   #[test]
   fn normalize_preserves_non_ascii_subjects() {
      assert_eq!(
         normalized_subject(Some("تنبيهات جديدة")).as_deref(),
         Some("تنبيهات جديدة"),
      );
   }

   #[test]
   fn normalize_empty_is_none() {
      assert_eq!(normalized_subject(Some("   ")), None);
      assert_eq!(normalized_subject(Some("Re:   ")), None);
      assert_eq!(normalized_subject(None), None);
   }

   async fn fresh_pool() -> Option<PgPool> {
      testkit::test_pool().await
   }

   #[tokio::test]
   async fn reply_chain_joins_parent() {
      let Some(pool) = fresh_pool().await else {
         return;
      };

      // Seed the parent.
      let parent = ThreadInputs {
         account_id: "a",
         msgid: "mp",
         message_id: Some("<parent@x>"),
         subject: Some("topic"),
         received_at_secs: 100,
         ..Default::default()
      };
      let parent_thrid = resolve_and_record_thrid(&pool, &parent).await.unwrap();

      // A reply referencing the parent lands on the same thrid.
      let reply = ThreadInputs {
         account_id: "a",
         msgid: "mr",
         message_id: Some("<reply@x>"),
         in_reply_to: Some("<parent@x>"),
         subject: Some("Re: topic"),
         received_at_secs: 200,
         ..Default::default()
      };
      let reply_thrid = resolve_and_record_thrid(&pool, &reply).await.unwrap();
      assert_eq!(parent_thrid, reply_thrid);
   }

   #[tokio::test]
   async fn subject_fallback_merges_orphan_reply() {
      let Some(pool) = fresh_pool().await else {
         return;
      };

      let parent = ThreadInputs {
         account_id: "a",
         msgid: "mp",
         message_id: Some("<parent@x>"),
         subject: Some("Quarterly review"),
         received_at_secs: 100,
         ..Default::default()
      };
      let parent_thrid = resolve_and_record_thrid(&pool, &parent).await.unwrap();

      // An orphan reply (no References / In-Reply-To) but the matching
      // normalized subject should join.
      let orphan = ThreadInputs {
         account_id: "a",
         msgid: "mo",
         message_id: Some("<orphan@x>"),
         subject: Some("Re: Quarterly review"),
         received_at_secs: 200,
         ..Default::default()
      };
      let orphan_thrid = resolve_and_record_thrid(&pool, &orphan).await.unwrap();
      assert_eq!(parent_thrid, orphan_thrid);
   }

   #[tokio::test]
   async fn subject_fallback_ignores_stale_threads() {
      let Some(pool) = fresh_pool().await else {
         return;
      };

      let old = ThreadInputs {
         account_id: "a",
         msgid: "mo",
         message_id: Some("<old@x>"),
         subject: Some("ping"),
         received_at_secs: 0, // ancient
         ..Default::default()
      };
      let old_thrid = resolve_and_record_thrid(&pool, &old).await.unwrap();

      // New "Re: ping" 60 days later: window is 30 days → mint a new thrid.
      let fresh = ThreadInputs {
         account_id: "a",
         msgid: "mf",
         message_id: Some("<new@x>"),
         subject: Some("Re: ping"),
         received_at_secs: 60 * 24 * 3600,
         ..Default::default()
      };
      let fresh_thrid = resolve_and_record_thrid(&pool, &fresh).await.unwrap();
      assert_ne!(old_thrid, fresh_thrid);
   }

   #[tokio::test]
   async fn new_message_no_links_mints_thrid() {
      let Some(pool) = fresh_pool().await else {
         return;
      };
      let inputs = ThreadInputs {
         account_id: "a",
         msgid: "m1",
         message_id: Some("<lonely@x>"),
         subject: Some("lonely"),
         received_at_secs: 100,
         ..Default::default()
      };
      let thrid = resolve_and_record_thrid(&pool, &inputs).await.unwrap();
      assert!(thrid.starts_with("t-"));
      assert!(thrid.len() > 2);
   }
}
