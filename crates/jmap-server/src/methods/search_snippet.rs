//! SearchSnippet/get (RFC 8621 §5) — highlighted search context.
//!
//! Works entirely off the cached `subject` + `preview` columns: the same
//! text `Email/query`'s LIKE filters match against, so highlights always
//! agree with what made the message a hit. Body-deep snippets would need
//! the raw message cached for every result; preview text is the honest
//! cheap approximation.

use std::cmp::Reverse;

use jmap_protocol::{
   email::EmailFilter,
   error::MethodError,
   filter::{
      Filter,
      FilterOp,
      has_unsupported_fields,
   },
   ids::{
      AccountId,
      Id,
   },
};

use super::{
   MethodResult,
   bad_args,
   enforce_get_limit,
   require_auth_match,
   server_fail,
};
use crate::{
   methods::email::EMAIL_FILTER_FIELDS,
   state::{
      AccountInfo,
      AppState,
   },
};

/// # Errors
///
/// Returns [`MethodError::UnsupportedFilter`] if the filter references fields
/// outside [`EMAIL_FILTER_FIELDS`], a `bad_args` error if the arguments fail to
/// deserialize, an auth error if `auth` does not match the requested account, a
/// limit error if too many `emailIds` are requested, and a `server_fail` error
/// if acquiring a database connection or running the snippet lookup fails.
pub async fn get(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id: AccountId,
      #[serde(default)]
      filter:     Option<Filter<EmailFilter>>,
      #[serde(rename = "emailIds")]
      email_ids:  Vec<Id>,
   }
   if args
      .get("filter")
      .is_some_and(|filter| has_unsupported_fields(filter, EMAIL_FILTER_FIELDS))
   {
      return Err(MethodError::UnsupportedFilter);
   }
   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid SearchSnippet/get args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   enforce_get_limit(req.email_ids.len())?;

   let terms = req.filter.as_ref().map(collect_terms).unwrap_or_default();

   let mut list = Vec::with_capacity(req.email_ids.len());
   let mut not_found = Vec::new();
   for id in &req.email_ids {
      let row = state
         .pool()
         .get()
         .await
         .map_err(|err| server_fail(format!("db pool: {err}")))?
         .query_opt(
            "SELECT subject, preview FROM messages WHERE account_id = $1 AND msgid = $2",
            &[&account_id, &id.as_ref()],
         )
         .await
         .map_err(|err| server_fail(format!("SearchSnippet/get: {err}")))?
         .map(|row| {
            (
               row.get::<_, Option<String>>(0),
               row.get::<_, Option<String>>(1),
            )
         });
      let Some((subject, preview)) = row else {
         not_found.push(id.clone());
         continue;
      };
      list.push(serde_json::json!({
          "emailId": id,
          "subject": subject.as_deref().and_then(|text| highlight(text, &terms)),
          "preview": preview.as_deref().and_then(|text| highlight(text, &terms)),
      }));
   }

   Ok(serde_json::json!({
       "accountId": account_id,
       "list": list,
       "notFound": not_found,
   }))
}

/// The free-text terms a filter tree searches for. NOT subtrees are skipped:
/// their terms describe what the results *don't* contain.
fn collect_terms(filter: &Filter<EmailFilter>) -> Vec<String> {
   fn walk(filter: &Filter<EmailFilter>, out: &mut Vec<String>) {
      match filter {
         Filter::Condition(condition) => {
            for term in [
               &condition.text,
               &condition.subject,
               &condition.from,
               &condition.to,
               &condition.body,
            ]
            .into_iter()
            .flatten()
            {
               if !term.is_empty() {
                  out.push(term.clone());
               }
            }
            for term in [&condition.cc, &condition.bcc]
               .into_iter()
               .flatten()
               .flatten()
            {
               if !term.is_empty() {
                  out.push(term.clone());
               }
            }
         },
         Filter::Operator {
            operator,
            conditions,
         } => {
            if *operator != FilterOp::Not {
               for condition in conditions {
                  walk(condition, out);
               }
            }
         },
      }
   }
   let mut out = Vec::new();
   walk(filter, &mut out);
   out.sort_by_key(|term| Reverse(term.len()));
   out.dedup();
   out
}

/// HTML-escape `text` and wrap case-insensitive matches of any term in
/// `<mark>` (RFC 8621 §5: only `<mark>` and HTML escaping are allowed).
/// Returns None when nothing matches — the spec's "null if not relevant".
fn highlight(text: &str, terms: &[String]) -> Option<String> {
   if terms.is_empty() {
      return None;
   }
   let lower = text.to_lowercase();
   // Collect non-overlapping match ranges, longest terms first (sorted by
   // the caller) so "foobar" wins over "foo".
   let mut ranges = Vec::<(usize, usize)>::new();
   for term in terms {
      let term_lower = term.to_lowercase();
      if term_lower.is_empty() {
         continue;
      }
      let mut start = 0;
      while let Some(pos) = lower[start..].find(&term_lower) {
         let begin = start + pos;
         let end = begin + term_lower.len();
         // The lowercased haystack can shift byte offsets for a handful of
         // Unicode characters (e.g. İ); guard against slicing mid-char.
         if text.is_char_boundary(begin) && text.is_char_boundary(end.min(text.len())) {
            let overlaps = ranges
               .iter()
               .any(|(other_begin, other_end)| begin < *other_end && *other_begin < end);
            if !overlaps {
               ranges.push((begin, end.min(text.len())));
            }
         }
         start = end.min(lower.len()).max(start + 1);
      }
   }
   if ranges.is_empty() {
      return None;
   }
   ranges.sort_unstable();

   let mut out = String::with_capacity(text.len() + ranges.len() * 13);
   let mut cursor = 0;
   for (begin, end) in ranges {
      out.push_str(&escape_html(&text[cursor..begin]));
      out.push_str("<mark>");
      out.push_str(&escape_html(&text[begin..end]));
      out.push_str("</mark>");
      cursor = end;
   }
   out.push_str(&escape_html(&text[cursor..]));
   Some(out)
}

fn escape_html(text: &str) -> String {
   text
      .replace('&', "&amp;")
      .replace('<', "&lt;")
      .replace('>', "&gt;")
      .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
   use super::*;

   fn filter(json: &str) -> Filter<EmailFilter> {
      serde_json::from_str(json).unwrap()
   }

   #[test]
   fn terms_from_tree_skip_not() {
      let tree = filter(
         r#"{"operator":"AND","conditions":[
                {"text":"rust"},
                {"operator":"NOT","conditions":[{"subject":"spam"}]}
            ]}"#,
      );
      assert_eq!(collect_terms(&tree), vec!["rust"]);
   }

   #[test]
   fn highlight_marks_and_escapes() {
      let terms = vec!["rust".to_owned()];
      assert_eq!(
         highlight("Rust <3 & jmap", &terms).as_deref(),
         Some("<mark>Rust</mark> &lt;3 &amp; jmap")
      );
      assert_eq!(highlight("nothing here", &terms), None);
      assert_eq!(highlight("anything", &[]), None);
   }

   #[test]
   fn longest_term_wins_overlap() {
      let mut terms = vec!["foo".to_owned(), "foobar".to_owned()];
      terms.sort_by_key(|term| Reverse(term.len()));
      assert_eq!(
         highlight("a foobar b", &terms).as_deref(),
         Some("a <mark>foobar</mark> b")
      );
   }

   #[test]
   fn unicode_terms_highlight() {
      let terms = vec!["köln".to_owned()];
      assert_eq!(
         highlight("Grüße aus Köln!", &terms).as_deref(),
         Some("Grüße aus <mark>Köln</mark>!")
      );
   }
}
