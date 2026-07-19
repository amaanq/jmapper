//! Email/* methods (RFC 8621 §4).

use std::{
   collections::{
      BTreeSet,
      HashMap,
      HashSet,
   },
   iter,
   time::Duration,
};

use chrono::{
   DateTime,
   Utc,
};
use imap_sync::{
   account::AccountRequest,
   db::{
      self,
      StateKind as DbStateKind,
   },
};
use jmap_protocol::{
   email::{
      Email,
      EmailAddress,
      EmailBodyPart,
      EmailBodyValue,
      EmailFilter,
   },
   error::MethodError,
   filter::{
      Comparator,
      Filter,
      has_unsupported_fields,
   },
   ids::{
      AccountId,
      Id,
      State,
   },
   methods::{
      ChangesResponse,
      GetResponse,
      QueryChangesRequest,
      QueryChangesResponse,
   },
   session::MAX_OBJECTS_IN_GET,
};
use tokio::{
   sync::oneshot,
   time,
};
use tokio_postgres::types::ToSql;

use super::{
   MethodResult,
   SqlParam,
   bad_args,
   cached_state,
   enforce_get_limit,
   escape_like,
   pg,
   pg_numbered,
   query_anchor_position,
   query_limit,
   query_position,
   require_auth_match,
   server_fail,
};
use crate::{
   methods::email_props,
   observability::METRICS,
   state::{
      AccountInfo,
      AppState,
   },
};

/// Upper bound on how many messages `Email/get` will return when the client
/// passes `ids: null` ("all emails"). The JMAP core advertises
/// `maxObjectsInGet = 500`; we enforce the same here and surface
/// `requestTooLarge` if the account has more than that cached.
const EMAIL_GET_MAX_IDS: usize = MAX_OBJECTS_IN_GET;

/// RFC 8621 §4.1 — `Email/get`.
///
/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, an
/// unsupported `bodyProperties` or `header:*` form is requested, the caller's
/// account does not match `accountId`, the account has more than
/// [`EMAIL_GET_MAX_IDS`] messages under an `ids: null` request
/// ([`MethodError::RequestTooLarge`]), a database query or the on-demand body
/// fetch fails, or the response cannot be serialized.
pub async fn get(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct GetArgs {
      #[serde(rename = "accountId")]
      account_id:             AccountId,
      #[serde(default)]
      ids:                    Option<Vec<Id>>,
      #[serde(default)]
      properties:             Option<Vec<String>>,
      #[serde(default, rename = "bodyProperties")]
      body_properties:        Option<Vec<String>>,
      #[serde(default, rename = "fetchTextBodyValues")]
      fetch_text_body_values: bool,
      #[serde(default, rename = "fetchHTMLBodyValues")]
      fetch_html_body_values: bool,
      #[serde(default, rename = "fetchAllBodyValues")]
      fetch_all_body_values:  bool,
      #[serde(default, rename = "maxBodyValueBytes")]
      max_body_value_bytes:   Option<u32>,
   }
   let req = serde_json::from_value::<GetArgs>(args)
      .map_err(|err| bad_args(format!("invalid Email/get args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   if let Some(property) = email_props::validate_body_properties(req.body_properties.as_deref()) {
      return Err(bad_args(format!(
         "unsupported EmailBodyPart property: {property}"
      )));
   }
   let body_properties = email_props::requested_body_properties(req.body_properties.as_deref());

   // bodyStructure and header:* materialize from the raw message. Malformed
   // header property shapes are rejected loudly — silently dropping them
   // would leave a client that genuinely needs them quietly broken.
   let mut wants_body_structure = false;
   let mut header_props = Vec::<email_props::HeaderProp>::new();
   if let Some(props) = req.properties.as_ref() {
      for prop in props {
         if prop == "bodyStructure" {
            wants_body_structure = true;
         } else if prop.starts_with("header:") {
            match email_props::parse_header_prop(prop) {
               Some(hp) => header_props.push(hp),
               None => {
                  return Err(bad_args(format!("unsupported header form: {prop}")));
               },
            }
         }
      }
   }
   let wants_raw = wants_body_structure || !header_props.is_empty();

   let wants_body = req.properties.is_none()
      || wants_raw
      || req.fetch_text_body_values
      || req.fetch_html_body_values
      || req.fetch_all_body_values
      || req.properties.as_ref().is_some_and(|props| {
         props.iter().any(|prop| {
            matches!(
               prop.as_str(),
               "bodyValues" | "textBody" | "htmlBody" | "attachments"
            )
         })
      });

   // RFC 8620 §5.1: `ids: null` means "return all matching objects". That
   // can't be honored for arbitrarily large accounts, so we cap at
   // `EMAIL_GET_MAX_IDS` — but we must *signal* the cap rather than silently
   // truncate. Fetch one row past the cap; if we get it, the account has
   // more messages than we're willing to return in a single Email/get and
   // the client should page via Email/query + Email/get(ids).
   let want_ids = if let Some(ids) = req.ids {
      let values = ids.into_iter().map(|i| i.0).collect::<Vec<String>>();
      enforce_get_limit(values.len())?;
      values
   } else {
      let probe = pg(state)
         .await?
         .query(
            "SELECT msgid FROM messages WHERE account_id = $1 ORDER BY received_at DESC LIMIT $2",
            &[&account_id, &(EMAIL_GET_MAX_IDS as i64 + 1)],
         )
         .await
         .map_err(|err| server_fail(format!("Email/get (ids=null) scan: {err}")))?
         .into_iter()
         .map(|row| row.get::<_, String>(0))
         .collect::<Vec<String>>();
      if probe.len() > EMAIL_GET_MAX_IDS {
         return Err(MethodError::RequestTooLarge);
      }
      probe
   };
   if want_ids.is_empty() {
      let resp = GetResponse::<Email> {
         account_id: AccountId(account_id.to_owned()),
         state:      cached_state(state, account_id, DbStateKind::Email).await?,
         list:       vec![],
         not_found:  vec![],
      };
      return serde_json::to_value(resp).map_err(|err| server_fail(err.to_string()));
   }

   let rows = pg(state)
      .await?
      .query(
         "SELECT m.*, string_agg(mm.mailbox_id, ',') AS mb_ids FROM messages m LEFT JOIN \
          message_mailboxes mm ON mm.account_id = m.account_id AND mm.msgid = m.msgid WHERE \
          m.account_id = $1 AND m.msgid = ANY($2) GROUP BY m.account_id, m.msgid",
         &[&account_id, &want_ids],
      )
      .await
      .map_err(|err| server_fail(format!("Email/get query: {err}")))?;

   let found = rows
      .into_iter()
      .filter_map(|row| email_from_row(&row).map(|email| (email.id.0.clone(), email)))
      .collect::<HashMap<String, Email>>();

   let mut list = Vec::with_capacity(want_ids.len());
   let mut not_found = Vec::new();
   for id in want_ids {
      match found.get(&id) {
         Some(email) => list.push(email.clone()),
         None => not_found.push(Id(id)),
      }
   }

   if wants_body {
      use std::sync::atomic::Ordering;
      let body_ids = list
         .iter()
         .map(|email| email.id.as_ref().to_owned())
         .collect::<Vec<_>>();
      match ensure_bodies_cached(state, account_id, &body_ids).await {
         Ok(fetched) => {
            METRICS
               .body_fetches_total
               .fetch_add(fetched as u64, Ordering::Relaxed);
         },
         Err(error) => {
            METRICS
               .body_fetch_failures_total
               .fetch_add(1, Ordering::Relaxed);
            tracing::warn!(error, "body fetch failed");
            return Err(server_fail(format!("body fetch failed: {error}")));
         },
      }

      for email in &mut list {
         if let Some((body_values, text_body, html_body, attachments)) = load_cached_body(
            state,
            account_id,
            email.id.as_ref(),
            req.max_body_value_bytes,
            req.fetch_text_body_values,
            req.fetch_html_body_values,
            req.fetch_all_body_values,
         )
         .await
         .ok()
         .flatten()
         {
            email.body_values = Some(body_values);
            email.text_body = Some(text_body);
            email.html_body = Some(html_body);
            email.attachments = Some(attachments);
         }
         if wants_raw {
            materialize_raw_props(
               state,
               account_id,
               email,
               wants_body_structure,
               &header_props,
            )
            .await?;
         }
      }
   }

   let mut resp_value = serde_json::to_value(GetResponse::<Email> {
      account_id: AccountId(account_id.to_owned()),
      state: cached_state(state, account_id, DbStateKind::Email).await?,
      list,
      not_found,
   })
   .map_err(|err| server_fail(err.to_string()))?;

   // RFC 8620 §5.1: if `properties` is provided, the server MUST only return
   // those properties (plus `id`, always). Filter the `list` entries to the
   // requested keys.
   if let Some(props) = req.properties.as_ref() {
      let mut allowed = props.iter().map(String::as_str).collect::<HashSet<&str>>();
      allowed.insert("id");
      if let Some(list) = resp_value
         .get_mut("list")
         .and_then(|value| value.as_array_mut())
      {
         for entry in list.iter_mut() {
            if let Some(map) = entry.as_object_mut() {
               map.retain(|key, _| allowed.contains(key.as_str()));
            }
         }
      }
   }

   if let Some(list) = resp_value
      .get_mut("list")
      .and_then(|value| value.as_array_mut())
   {
      for entry in list {
         for property in ["textBody", "htmlBody", "attachments", "bodyStructure"] {
            if let Some(value) = entry.get_mut(property) {
               email_props::project_body_properties(value, &body_properties);
            }
         }
      }
   }

   Ok(resp_value)
}

/// Fill `bodyStructure` / `header:*` entries from the cached raw bytes.
async fn materialize_raw_props(
   state: &AppState,
   account_id: &str,
   email: &mut Email,
   wants_body_structure: bool,
   header_props: &[email_props::HeaderProp],
) -> Result<(), MethodError> {
   let raw = pg(state)
      .await?
      .query_opt(
         "SELECT raw_rfc822 FROM raw_messages WHERE account_id = $1 AND msgid = $2",
         &[&account_id, &email.id.as_ref()],
      )
      .await
      .map_err(|err| server_fail(format!("raw load: {err}")))?
      .map(|row| row.get::<_, Vec<u8>>(0))
      .ok_or_else(|| server_fail(format!("raw cache vanished for {}", email.id)))?;
   let Some(parsed) = mail_parser::MessageParser::default().parse(&raw) else {
      return Err(server_fail(format!("unparseable raw message {}", email.id)));
   };
   if wants_body_structure {
      let tree = email_props::body_structure(&parsed, email.id.as_ref());
      email.extra.insert(
         "bodyStructure".to_owned(),
         serde_json::to_value(tree).map_err(|err| server_fail(err.to_string()))?,
      );
   }
   for hp in header_props {
      email
         .extra
         .insert(hp.key.clone(), email_props::header_value(&parsed, hp));
   }
   Ok(())
}

async fn ensure_bodies_cached(
   state: &AppState,
   account_id: &str,
   msgids: &[String],
) -> Result<usize, String> {
   if msgids.is_empty() {
      return Ok(0);
   }
   let wanted = msgids
      .iter()
      .cloned()
      .collect::<BTreeSet<_>>()
      .into_iter()
      .collect::<Vec<_>>();
   let cached = state
      .pool()
      .get()
      .await
      .map_err(|err| err.to_string())?
      .query(
         "SELECT msgid FROM raw_messages WHERE account_id = $1 AND msgid = ANY($2)",
         &[&account_id, &wanted],
      )
      .await
      .map_err(|err| err.to_string())?
      .into_iter()
      .map(|row| row.get::<_, String>(0))
      .collect::<HashSet<_>>();
   let missing = wanted
      .into_iter()
      .filter(|msgid| !cached.contains(msgid))
      .collect::<Vec<_>>();
   if missing.is_empty() {
      return Ok(0);
   }

   let tx = state
      .account_sender(account_id)
      .ok_or_else(|| format!("no sync task for account {account_id}"))?;
   let (respond, rx) = oneshot::channel();
   let fetched = missing.len();
   let result = time::timeout(Duration::from_secs(30), async move {
      tx.send(AccountRequest::FetchBodies {
         msgids: missing,
         respond,
      })
      .await
      .map_err(|_| "sync task channel closed".to_owned())?;
      rx.await.map_err(|_| "sync task dropped fetch".to_owned())
   })
   .await
   .map_err(|_| "body fetch timed out".to_owned())??;
   result.map_err(|err| err.to_string())?;
   Ok(fetched)
}

async fn load_cached_body(
   state: &AppState,
   account_id: &str,
   msgid: &str,
   max_bytes: Option<u32>,
   fetch_text_body_values: bool,
   fetch_html_body_values: bool,
   fetch_all_body_values: bool,
) -> Result<
   Option<(
      HashMap<String, EmailBodyValue>,
      Vec<EmailBodyPart>,
      Vec<EmailBodyPart>,
      Vec<EmailBodyPart>,
   )>,
   String,
> {
   #[derive(serde::Deserialize)]
   struct Shell {
      #[serde(default, rename = "textBody")]
      text_body: Vec<EmailBodyPart>,
      #[serde(default, rename = "htmlBody")]
      html_body: Vec<EmailBodyPart>,
   }
   let row = state
      .pool()
      .get()
      .await
      .map_err(|err| format!("db pool: {err}"))?
      .query_opt(
         "SELECT headers_json, body_values_json, attachments_json FROM raw_messages WHERE \
          account_id = $1 AND msgid = $2",
         &[&account_id, &msgid],
      )
      .await
      .map_err(|err| err.to_string())?
      .map(|row| {
         (
            row.get::<_, String>(0),
            row.get::<_, String>(1),
            row.get::<_, String>(2),
         )
      });
   let Some((headers_json, body_values_json, attachments_json)) = row else {
      return Ok(None);
   };

   let mut body_values = serde_json::from_str::<HashMap<String, EmailBodyValue>>(&body_values_json)
      .unwrap_or_default();
   let shell = serde_json::from_str::<Shell>(&headers_json).unwrap_or(Shell {
      text_body: vec![],
      html_body: vec![],
   });
   let attachments =
      serde_json::from_str::<Vec<EmailBodyPart>>(&attachments_json).unwrap_or_default();

   let mut selected = HashSet::<String>::new();
   if fetch_all_body_values {
      selected.extend(body_values.keys().cloned());
   } else {
      if fetch_text_body_values {
         selected.extend(
            shell
               .text_body
               .iter()
               .filter_map(|part| part.part_id.clone()),
         );
      }
      if fetch_html_body_values {
         selected.extend(
            shell
               .html_body
               .iter()
               .filter_map(|part| part.part_id.clone()),
         );
      }
   }
   body_values.retain(|part_id, _| selected.contains(part_id));
   if let Some(cap) = max_bytes.filter(|cap| *cap > 0).map(|cap| cap as usize) {
      for value in body_values.values_mut() {
         if value.value.len() > cap {
            let mut cut = cap;
            while !value.value.is_char_boundary(cut) {
               cut -= 1;
            }
            value.value.truncate(cut);
            value.is_truncated = true;
         }
      }
   }

   Ok(Some((
      body_values,
      shell.text_body,
      shell.html_body,
      attachments,
   )))
}

/// RFC 8621 §4.4 — `Email/query`.
///
/// # Errors
///
/// Returns [`MethodError`] if the filter references an unsupported field
/// ([`MethodError::UnsupportedFilter`]), the arguments fail to deserialize,
/// the caller's account does not match `accountId`, the sort comparators are
/// unsupported ([`MethodError::UnsupportedSort`]), the requested `anchor` is
/// absent from the result set ([`MethodError::AnchorNotFound`]), or a database
/// query fails.
pub async fn query(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id:       AccountId,
      #[serde(default)]
      filter:           Option<Filter<EmailFilter>>,
      #[serde(default)]
      sort:             Option<Vec<Comparator>>,
      #[serde(default)]
      position:         Option<i64>,
      #[serde(default)]
      limit:            Option<u32>,
      #[serde(default, rename = "calculateTotal")]
      calculate_total:  bool,
      #[serde(default, rename = "collapseThreads")]
      collapse_threads: bool,
      #[serde(default)]
      anchor:           Option<Id>,
      #[serde(default, rename = "anchorOffset")]
      anchor_offset:    Option<i64>,
   }
   if args
      .get("filter")
      .is_some_and(|filter| has_unsupported_fields(filter, EMAIL_FILTER_FIELDS))
   {
      return Err(MethodError::UnsupportedFilter);
   }
   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid Email/query args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   let filter_sql = req.filter.as_ref().map(compile_filter_tree).transpose()?;

   let sort_terms = email_sort_terms(req.sort.as_deref())?;
   let tie_direction = sort_terms[0].direction;
   let inner_order = sort_terms
      .iter()
      .map(|term| format!("{} {}", term.expression, term.direction))
      .chain(iter::once(format!("m.msgid {tie_direction}")))
      .collect::<Vec<_>>()
      .join(", ");

   let (where_sql, binds) = match filter_sql {
      Some(compiled) => (compiled.where_clause, compiled.binds),
      None => (String::new(), Vec::new()),
   };
   let and_filter = if where_sql.is_empty() {
      String::new()
   } else {
      format!(" AND ({where_sql})")
   };
   // collapseThreads (RFC 8621 §4.4.1): one email per thread — the member
   // that sorts first under the requested order stands in for the thread.
   let mut sql = if req.collapse_threads {
      let sort_columns = sort_terms
         .iter()
         .enumerate()
         .map(|(index, term)| format!("{} AS sort_key_{index}", term.expression))
         .collect::<Vec<_>>()
         .join(", ");
      let outer_order = sort_terms
         .iter()
         .enumerate()
         .map(|(index, term)| format!("sort_key_{index} {}", term.direction))
         .chain(iter::once(format!("msgid {tie_direction}")))
         .collect::<Vec<_>>()
         .join(", ");
      format!(
         "SELECT msgid FROM (SELECT m.msgid AS msgid, {sort_columns}, ROW_NUMBER() OVER \
          (PARTITION BY m.thrid ORDER BY {inner_order}) AS rn FROM messages m WHERE m.account_id \
          = ?{and_filter}) WHERE rn = 1 ORDER BY {outer_order}"
      )
   } else {
      format!(
         "SELECT m.msgid FROM messages m WHERE m.account_id = ?{and_filter} ORDER BY {inner_order}"
      )
   };
   let needs_count = req.calculate_total
      || (req.anchor.is_none() && req.position.is_some_and(|position| position < 0));
   let count = if needs_count {
      let count_expr = if req.collapse_threads {
         "COUNT(DISTINCT m.thrid)"
      } else {
         "COUNT(*)"
      };
      let count_sql =
         format!("SELECT {count_expr} FROM messages m WHERE m.account_id = ?{and_filter}");
      let mut params = vec![&account_id as &(dyn ToSql + Sync)];
      params.extend(binds.iter().map(SqlParam::as_dyn));
      Some(
         pg(state)
            .await?
            .query_one(&pg_numbered(&count_sql), &params)
            .await
            .map_err(|err| server_fail(format!("Email/query count: {err}")))?
            .get::<_, i64>(0),
      )
   } else {
      None
   };
   let (limit, response_limit) = query_limit(req.limit, 500);

   let (ids, position) = if let Some(anchor) = req.anchor.as_ref() {
      // Anchor pagination (RFC 8620 §5.5): position is derived from where
      // the anchor id lands in the full ordering. Fetching the whole id
      // column is fine at the corpus sizes this proxy targets, and it's
      // the only approach that stays correct under collapseThreads.
      let mut params = vec![&account_id as &(dyn ToSql + Sync)];
      params.extend(binds.iter().map(SqlParam::as_dyn));
      let all = pg(state)
         .await?
         .query(&pg_numbered(&sql), &params)
         .await
         .map_err(|err| server_fail(format!("Email/query: {err}")))?
         .into_iter()
         .map(|row| row.get::<_, String>(0))
         .collect::<Vec<String>>();
      let idx = all
         .iter()
         .position(|msgid| msgid == anchor.as_ref())
         .ok_or(MethodError::AnchorNotFound)?;
      let start = query_anchor_position(idx, req.anchor_offset);
      let ids = all
         .into_iter()
         .skip(start)
         .take(limit)
         .map(Id)
         .collect::<Vec<Id>>();
      (ids, i64::try_from(start).unwrap_or(i64::MAX))
   } else {
      let position = query_position(
         req.position,
         count
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default(),
      );
      let sql_position = i64::try_from(position).unwrap_or(i64::MAX);
      sql.push_str(" LIMIT ? OFFSET ?");
      let sql_limit = i64::try_from(limit).unwrap_or(i64::MAX);
      let mut params = vec![&account_id as &(dyn ToSql + Sync)];
      params.extend(binds.iter().map(SqlParam::as_dyn));
      params.push(&sql_limit);
      params.push(&sql_position);
      let ids = pg(state)
         .await?
         .query(&pg_numbered(&sql), &params)
         .await
         .map_err(|err| server_fail(format!("Email/query: {err}")))?
         .into_iter()
         .map(|row| Id(row.get(0)))
         .collect::<Vec<Id>>();
      (ids, sql_position)
   };

   let total = if req.calculate_total {
      count.and_then(|count| u64::try_from(count).ok())
   } else {
      None
   };

   let query_state = cached_state(state, account_id, DbStateKind::Email).await?;
   let mut out = serde_json::json!({
       "accountId": account_id,
       "queryState": query_state,
       "canCalculateChanges": false,
       "position": position,
       "ids": ids,
   });
   if let Some(total) = total {
      out["total"] = serde_json::Value::from(total);
   }
   if let Some(limit) = response_limit {
      out["limit"] = serde_json::Value::from(limit);
   }
   Ok(out)
}

pub(crate) const EMAIL_FILTER_FIELDS: &[&str] = &[
   "inMailbox",
   "inMailboxOtherThan",
   "before",
   "after",
   "minSize",
   "maxSize",
   "allInThreadHaveKeyword",
   "someInThreadHaveKeyword",
   "noneInThreadHaveKeyword",
   "hasKeyword",
   "notKeyword",
   "hasAttachment",
   "text",
   "from",
   "to",
   "cc",
   "bcc",
   "subject",
   "body",
   "header",
];

/// RFC 8621 §4.2 — `Email/changes`.
///
/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, `sinceState`
/// is not a valid `u64`, the caller's account does not match `accountId`,
/// reading the current state fails, or `sinceState` predates the current
/// state — since expunged rows are physically deleted, an accurate delta
/// cannot be reconstructed and [`MethodError::CannotCalculateChanges`] is
/// returned to force a refetch.
pub async fn changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id:  AccountId,
      #[serde(rename = "sinceState")]
      since_state: String,
      #[serde(rename = "maxChanges")]
      max_changes: Option<u32>,
   }
   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid Email/changes args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   let since = req.since_state.parse::<u64>().map_err(|_| {
      bad_args(format!(
         "sinceState {:?} is not a valid u64",
         req.since_state
      ))
   })?;
   let cur = db::get_state(state.pool(), account_id)
      .await
      .map_err(|err| server_fail(format!("state: {err}")))?;
   let cur_mod = cur.email_modseq as u64;
   let _ = req.max_changes;
   if since != cur_mod {
      // The cache physically deletes expunged rows, so reconstructing a
      // complete delta from the live messages table would omit destroyed
      // ids. Force a refetch until durable change history is available.
      return Err(MethodError::CannotCalculateChanges);
   }

   serde_json::to_value(ChangesResponse {
      account_id:       AccountId(account_id.to_owned()),
      old_state:        State(since.to_string()),
      new_state:        State(cur_mod.to_string()),
      has_more_changes: false,
      created:          vec![],
      updated:          vec![],
      destroyed:        vec![],
   })
   .map_err(|err| server_fail(err.to_string()))
}

/// RFC 8621 §4.5 — `Email/queryChanges`.
///
/// # Errors
///
/// Returns [`MethodError`] if the filter references an unsupported field
/// ([`MethodError::UnsupportedFilter`]), the arguments fail to deserialize,
/// the caller's account does not match `accountId`, the sort comparators are
/// unsupported, or the query state has advanced past `sinceQueryState` (or a
/// total was requested) — neither can be served without durable query
/// snapshots, so [`MethodError::CannotCalculateChanges`] is returned.
pub async fn query_changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   if args
      .get("filter")
      .is_some_and(|filter| has_unsupported_fields(filter, EMAIL_FILTER_FIELDS))
   {
      return Err(MethodError::UnsupportedFilter);
   }

   let req = serde_json::from_value::<QueryChangesRequest<EmailFilter>>(args)
      .map_err(|err| bad_args(format!("invalid Email/queryChanges args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   req.filter.as_ref().map(compile_filter_tree).transpose()?;
   email_sort_terms(req.sort.as_deref())?;
   let current = cached_state(state, account_id, DbStateKind::Email).await?;
   if req.since_query_state != current || req.calculate_total {
      // Query snapshots are not durable yet, so an older ordered result set
      // cannot be reconstructed without inventing removals or indices. A
      // requested total also needs the full query execution path.
      return Err(MethodError::CannotCalculateChanges);
   }

   serde_json::to_value(QueryChangesResponse {
      account_id:      req.account_id,
      old_query_state: req.since_query_state,
      new_query_state: current,
      total:           None,
      removed:         vec![],
      added:           vec![],
   })
   .map_err(|err| server_fail(err.to_string()))
}

/// SQL expression for a JMAP sort property. `from`/`to` order by the first
/// address's display name (fallback address) so the result matches what a
/// client renders in its list column.
struct SortTerm {
   expression: String,
   direction:  &'static str,
}

fn email_sort_terms(comparators: Option<&[Comparator]>) -> Result<Vec<SortTerm>, MethodError> {
   let Some(comparators) = comparators.filter(|comparators| !comparators.is_empty()) else {
      return Ok(vec![SortTerm {
         expression: "m.received_at".into(),
         direction:  "DESC",
      }]);
   };

   comparators
      .iter()
      .map(|comparator| {
         if comparator.collation.is_some() {
            return Err(MethodError::UnsupportedSort);
         }
         let direction = if comparator.is_ascending { "ASC" } else { "DESC" };
         // hasKeyword (RFC 8621 §4.4.2) carries the keyword as a comparator
         // argument; the expression mirrors the hasKeyword filter's substring
         // probe of the JSON flag array. Boolean sort, so DESC puts emails
         // carrying the keyword first. The needle is inlined as an escaped
         // literal because sort expressions cannot carry binds: they are
         // repeated in the collapseThreads projection ahead of the WHERE
         // binds, which would break the placeholder numbering.
         if comparator.property == "hasKeyword" {
            let Some(serde_json::Value::String(keyword)) = comparator.extra.get("keyword") else {
               return Err(MethodError::UnsupportedSort);
            };
            if comparator.extra.len() > 1 {
               return Err(MethodError::UnsupportedSort);
            }
            let needle = format!("\"{}\"", keyword_to_flag(keyword)).replace('\'', "''");
            return Ok(SortTerm {
               expression: format!("(strpos(m.flags_json, '{needle}') > 0)"),
               direction,
            });
         }
         if !comparator.extra.is_empty() {
            return Err(MethodError::UnsupportedSort);
         }
         Ok(SortTerm {
            expression: sort_expr(&comparator.property)
               .ok_or(MethodError::UnsupportedSort)?
               .into(),
            direction,
         })
      })
      .collect::<Result<Vec<_>, _>>()
}

fn sort_expr(prop: &str) -> Option<&'static str> {
   Some(match prop {
      "receivedAt" => "m.received_at",
      "sentAt" => "m.sent_at",
      "size" => "m.size",
      "subject" => "m.subject",
      "from" => {
         "COALESCE(m.from_json::jsonb #>> '{0,name}', m.from_json::jsonb #>> '{0,email}', '')"
      },
      "to" => "COALESCE(m.to_json::jsonb #>> '{0,name}', m.to_json::jsonb #>> '{0,email}', '')",
      _ => return None,
   })
}

struct CompiledFilter {
   where_clause: String,
   binds:        Vec<SqlParam>,
}

use SqlParam as Bind;

/// Compile a full RFC 8620 filter tree to one SQL boolean expression.
///
/// Empty operators follow the spec's identity semantics: AND of nothing is
/// true, OR of nothing is false, and NOT (defined as "none of the conditions
/// match") of nothing is true.
fn compile_filter_tree(filter: &Filter<EmailFilter>) -> Result<CompiledFilter, MethodError> {
   use jmap_protocol::filter::FilterOp;
   match filter {
      Filter::Condition(condition) => compile_filter(condition),
      Filter::Operator {
         operator,
         conditions,
      } => {
         let mut parts = Vec::<String>::with_capacity(conditions.len());
         let mut binds = Vec::<Bind>::new();
         for child in conditions {
            let cf = compile_filter_tree(child)?;
            parts.push(format!("({})", cf.where_clause));
            binds.extend(cf.binds);
         }
         let where_clause = match operator {
            FilterOp::And | FilterOp::Not if parts.is_empty() => "1".to_owned(),
            FilterOp::Or if parts.is_empty() => "0".to_owned(),
            FilterOp::And => parts.join(" AND "),
            FilterOp::Or => parts.join(" OR "),
            FilterOp::Not => format!("NOT ({})", parts.join(" OR ")),
         };
         Ok(CompiledFilter {
            where_clause,
            binds,
         })
      },
   }
}

fn compile_filter(condition: &EmailFilter) -> Result<CompiledFilter, MethodError> {
   let mut wheres = Vec::<String>::new();
   let mut binds = Vec::<Bind>::new();

   if let Some(mb) = &condition.in_mailbox {
      // EXISTS instead of a JOIN so the predicate stays a self-contained
      // boolean and composes under OR/NOT in operator trees.
      wheres.push(
         "EXISTS (SELECT 1 FROM message_mailboxes mm WHERE mm.account_id = m.account_id AND \
          mm.msgid = m.msgid AND mm.mailbox_id = ?)"
            .to_owned(),
      );
      binds.push(Bind::Str(mb.0.clone()));
   }
   if let Some(dt) = condition.before.as_ref() {
      wheres.push("m.received_at < ?".into());
      binds.push(Bind::Int(dt.timestamp()));
   }
   if let Some(dt) = condition.after.as_ref() {
      wheres.push("m.received_at >= ?".into());
      binds.push(Bind::Int(dt.timestamp()));
   }
   if let Some(sz) = condition.min_size {
      wheres.push("m.size >= ?".into());
      binds.push(Bind::Int(sz as i64));
   }
   if let Some(sz) = condition.max_size {
      wheres.push("m.size <= ?".into());
      binds.push(Bind::Int(sz as i64));
   }
   if let Some(query) = condition.subject.as_ref() {
      wheres.push("m.subject ILIKE ? ESCAPE '\\'".into());
      binds.push(Bind::Str(format!("%{}%", escape_like(query))));
   }
   if let Some(query) = condition.from.as_ref() {
      wheres.push("m.from_json ILIKE ? ESCAPE '\\'".into());
      binds.push(Bind::Str(format!("%{}%", escape_like(query))));
   }
   if let Some(query) = condition.to.as_ref() {
      wheres.push("m.to_json ILIKE ? ESCAPE '\\'".into());
      binds.push(Bind::Str(format!("%{}%", escape_like(query))));
   }
   if let Some(query) = condition.text.as_ref() {
      // Coarse "text" filter — matches subject, any address JSON, preview.
      wheres.push(
         "(m.subject ILIKE ? ESCAPE '\\' OR m.from_json ILIKE ? ESCAPE '\\' OR m.to_json ILIKE ? \
          ESCAPE '\\' OR m.preview ILIKE ? ESCAPE '\\')"
            .into(),
      );
      let needle = format!("%{}%", escape_like(query));
      binds.push(Bind::Str(needle.clone()));
      binds.push(Bind::Str(needle.clone()));
      binds.push(Bind::Str(needle.clone()));
      binds.push(Bind::Str(needle));
   }
   if let Some(has) = condition.has_keyword.as_ref() {
      let flag = format!("\"{}\"", keyword_to_flag(has));
      wheres.push("strpos(m.flags_json, ?) > 0".into());
      binds.push(Bind::Str(flag));
   }
   if let Some(has) = condition.not_keyword.as_ref() {
      let flag = format!("\"{}\"", keyword_to_flag(has));
      wheres.push("strpos(m.flags_json, ?) = 0".into());
      binds.push(Bind::Str(flag));
   }
   if let Some(has) = condition.has_attachment {
      wheres.push("m.has_attachment = ?".into());
      binds.push(Bind::Int(i64::from(has)));
   }

   if let Some(mbs) = condition.in_mailbox_other_than.as_ref() {
      // Empty exclusion list degenerates to "member of at least one
      // mailbox"; an empty SQL `NOT IN ()` list is invalid.
      let not_in = if mbs.is_empty() {
         String::new()
      } else {
         format!(
            " AND mm.mailbox_id NOT IN ({})",
            vec!["?"; mbs.len()].join(",")
         )
      };
      wheres.push(format!(
         "EXISTS (SELECT 1 FROM message_mailboxes mm WHERE mm.account_id = m.account_id AND \
          mm.msgid = m.msgid{not_in})"
      ));
      for mb in mbs {
         binds.push(Bind::Str(mb.0.clone()));
      }
   }

   for (field, col) in [(&condition.cc, "cc_json"), (&condition.bcc, "bcc_json")] {
      for query in field.iter().flatten() {
         wheres.push(format!("m.{col} ILIKE ? ESCAPE '\\'"));
         binds.push(Bind::Str(format!("%{}%", escape_like(query))));
      }
   }
   if let Some(query) = condition.body.as_ref() {
      // Body search runs over *cached* parsed bodies plus the preview. The
      // cache fills lazily (Email/get bodyValues, blob downloads), so this
      // is best-effort by design — mirroring the whole account's raw mail
      // just to serve a rare filter would defeat the lazy body cache.
      wheres.push(
         "(m.preview ILIKE ? ESCAPE '\\' OR EXISTS (SELECT 1 FROM raw_messages r WHERE \
          r.account_id = m.account_id AND r.msgid = m.msgid AND r.body_values_json ILIKE ? ESCAPE \
          '\\'))"
            .into(),
      );
      let needle = format!("%{}%", escape_like(query));
      binds.push(Bind::Str(needle.clone()));
      binds.push(Bind::Str(needle));
   }

   // Thread-scoped keyword predicates (RFC 8621 §4.4.1) quantify over every
   // message sharing the candidate's thrid.
   if let Some(kw) = condition.some_in_thread_have_keyword.as_ref() {
      wheres.push(
         "EXISTS (SELECT 1 FROM messages m2 WHERE m2.account_id = m.account_id AND m2.thrid = \
          m.thrid AND strpos(m2.flags_json, ?) > 0)"
            .into(),
      );
      binds.push(Bind::Str(format!("\"{}\"", keyword_to_flag(kw))));
   }
   if let Some(kw) = condition.all_in_thread_have_keyword.as_ref() {
      wheres.push(
         "NOT EXISTS (SELECT 1 FROM messages m2 WHERE m2.account_id = m.account_id AND m2.thrid = \
          m.thrid AND strpos(m2.flags_json, ?) = 0)"
            .into(),
      );
      binds.push(Bind::Str(format!("\"{}\"", keyword_to_flag(kw))));
   }
   if let Some(kw) = condition.none_in_thread_have_keyword.as_ref() {
      wheres.push(
         "NOT EXISTS (SELECT 1 FROM messages m2 WHERE m2.account_id = m.account_id AND m2.thrid = \
          m.thrid AND strpos(m2.flags_json, ?) > 0)"
            .into(),
      );
      binds.push(Bind::Str(format!("\"{}\"", keyword_to_flag(kw))));
   }

   // `header` stays unsupported: arbitrary headers are neither cached nor
   // indexed, and matching raw bytes would false-positive on body content.
   if condition.header.is_some() {
      return Err(MethodError::UnsupportedFilter);
   }

   Ok(CompiledFilter {
      // An all-defaults condition constrains nothing; emit a literal true so
      // the clause stays a valid boolean inside operator trees.
      where_clause: if wheres.is_empty() {
         "1".to_owned()
      } else {
         wheres.join(" AND ")
      },
      binds,
   })
}

/// JMAP keywords use dollar-prefixed well-known names (`$seen`, `$flagged`).
/// Our cache stores them as-is.
fn keyword_to_flag(key: &str) -> String {
   key.to_owned()
}

fn email_from_row(pg_row: &tokio_postgres::Row) -> Option<Email> {
   let row = db::MessageRow::from_pg_row(pg_row).ok()?;
   let mb_ids_csv = pg_row.try_get::<_, Option<String>>("mb_ids").ok().flatten();
   let mut mailbox_ids = HashMap::new();
   if let Some(csv) = mb_ids_csv {
      for id in csv.split(',').filter(|segment| !segment.is_empty()) {
         mailbox_ids.insert(Id(id.to_owned()), true);
      }
   }
   let flags = serde_json::from_str::<Vec<String>>(&row.flags_json).unwrap_or_default();
   let mut keywords = HashMap::new();
   for flag in &flags {
      keywords.insert(flag.clone(), true);
   }
   let parse_addr = |j: Option<&str>| -> Option<Vec<EmailAddress>> {
      let json = j?;
      serde_json::from_str::<Vec<EmailAddress>>(json).ok()
   };

   // RFC 8621 §4.1.2: messageId/inReplyTo/references are Lists of message-id
   // tokens (unbracketed). References is a whitespace-separated list in the
   // header; split it into tokens.
   let one_token = |header: &Option<String>| -> Option<Vec<String>> {
      let value = header.as_ref()?;
      let token = value
         .trim()
         .trim_matches(|ch| ch == '<' || ch == '>')
         .trim();
      if token.is_empty() {
         None
      } else {
         Some(vec![token.to_owned()])
      }
   };
   let refs = row.references_header.as_ref().and_then(|value| {
      let tokens = value
         .split_whitespace()
         .map(|token| token.trim_matches(|ch| ch == '<' || ch == '>').to_owned())
         .filter(|token| !token.is_empty())
         .collect::<Vec<String>>();
      if tokens.is_empty() {
         None
      } else {
         Some(tokens)
      }
   });

   Some(Email {
      id: Id(row.msgid.clone()),
      blob_id: Id(format!("blob-{}", row.msgid)),
      thread_id: Id(row.thrid.clone()),
      mailbox_ids,
      keywords,
      size: row.size as u64,
      received_at: DateTime::<Utc>::from_timestamp(row.received_at, 0).unwrap_or_else(Utc::now),
      message_id: one_token(&row.message_id_header),
      in_reply_to: one_token(&row.in_reply_to_header),
      references: refs,
      sender: None,
      from: parse_addr(row.from_json.as_deref()),
      to: parse_addr(row.to_json.as_deref()),
      cc: parse_addr(row.cc_json.as_deref()),
      bcc: parse_addr(row.bcc_json.as_deref()),
      reply_to: parse_addr(row.reply_to_json.as_deref()),
      subject: row.subject,
      sent_at: row
         .sent_at
         .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0)),
      has_attachment: row.has_attachment != 0,
      preview: row.preview,
      body_values: None,
      text_body: None,
      html_body: None,
      attachments: None,
      extra: serde_json::Map::default(),
   })
}

#[cfg(test)]
mod filter_tests {
   use super::*;

   fn parse(json: &str) -> Filter<EmailFilter> {
      serde_json::from_str(json).expect("filter json")
   }

   #[test]
   fn flat_condition_compiles() {
      let filter = parse(r#"{"inMailbox":"mb-1","hasAttachment":true}"#);
      let compiled = compile_filter_tree(&filter).unwrap();
      assert!(compiled.where_clause.contains("EXISTS"));
      assert!(compiled.where_clause.contains("m.has_attachment = ?"));
      assert_eq!(compiled.binds.len(), 2);
   }

   #[test]
   fn and_tree_compiles() {
      let filter = parse(
         r#"{"operator":"AND","conditions":[
                {"inMailbox":"mb-1"},
                {"operator":"NOT","conditions":[{"hasKeyword":"$seen"}]}
            ]}"#,
      );
      let compiled = compile_filter_tree(&filter).unwrap();
      assert!(compiled.where_clause.contains(") AND (NOT ("));
      assert_eq!(compiled.binds.len(), 2);
   }

   #[test]
   fn or_of_froms_compiles() {
      let filter = parse(r#"{"operator":"OR","conditions":[{"from":"alice"},{"from":"bob"}]}"#);
      let compiled = compile_filter_tree(&filter).unwrap();
      assert!(compiled.where_clause.contains(") OR ("));
      assert_eq!(compiled.binds.len(), 2);
   }

   #[test]
   fn empty_operators_have_identity_semantics() {
      for (json, expect) in [
         (r#"{"operator":"AND","conditions":[]}"#, "1"),
         (r#"{"operator":"OR","conditions":[]}"#, "0"),
         (r#"{"operator":"NOT","conditions":[]}"#, "1"),
      ] {
         let compiled = compile_filter_tree(&parse(json)).unwrap();
         assert_eq!(compiled.where_clause, expect, "for {json}");
         assert!(compiled.binds.is_empty());
      }
   }

   #[test]
   fn unsupported_leaf_propagates_from_tree() {
      let filter = parse(r#"{"operator":"AND","conditions":[{"header":["X-Spam"]}]}"#);
      assert!(matches!(
         compile_filter_tree(&filter),
         Err(MethodError::UnsupportedFilter)
      ));
   }

   #[test]
   fn thread_keyword_filters_compile() {
      let compiled =
         compile_filter_tree(&parse(r#"{"someInThreadHaveKeyword":"$flagged"}"#)).unwrap();
      assert!(
         compiled.where_clause.starts_with("EXISTS"),
         "{}",
         compiled.where_clause
      );
      assert!(compiled.where_clause.contains("m2.thrid = m.thrid"));

      let compiled = compile_filter_tree(&parse(r#"{"allInThreadHaveKeyword":"$seen"}"#)).unwrap();
      assert!(compiled.where_clause.starts_with("NOT EXISTS"));
      assert!(compiled.where_clause.contains("= 0"));

      let compiled = compile_filter_tree(&parse(r#"{"noneInThreadHaveKeyword":"$seen"}"#)).unwrap();
      assert!(compiled.where_clause.starts_with("NOT EXISTS"));
      assert!(compiled.where_clause.contains("> 0"));
   }

   #[test]
   fn body_and_cc_filters_compile() {
      let compiled = compile_filter_tree(&parse(r#"{"body":"needle"}"#)).unwrap();
      assert!(compiled.where_clause.contains("raw_messages"));
      assert!(compiled.where_clause.contains("m.preview ILIKE ?"));
      assert_eq!(compiled.binds.len(), 2);

      let compiled =
         compile_filter_tree(&parse(r#"{"cc":["alice","bob"],"bcc":["carol"]}"#)).unwrap();
      assert!(compiled.where_clause.contains("m.cc_json ILIKE ?"));
      assert!(compiled.where_clause.contains("m.bcc_json ILIKE ?"));
      assert_eq!(compiled.binds.len(), 3);
   }

   #[test]
   fn in_mailbox_other_than_compiles() {
      let filter = parse(r#"{"inMailboxOtherThan":["mb-1","mb-2"]}"#);
      let compiled = compile_filter_tree(&filter).unwrap();
      assert!(compiled.where_clause.contains("NOT IN (?,?)"));
      assert_eq!(compiled.binds.len(), 2);

      let filter = parse(r#"{"inMailboxOtherThan":[]}"#);
      let compiled = compile_filter_tree(&filter).unwrap();
      assert!(!compiled.where_clause.contains("NOT IN"));
      assert!(compiled.binds.is_empty());
   }

   #[test]
   fn every_email_sort_comparator_is_preserved() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(serde_json::json!([
          {"property": "subject", "isAscending": true},
          {"property": "receivedAt", "isAscending": false}
      ]))
      .unwrap();
      let terms = email_sort_terms(Some(&comparators)).unwrap();

      assert_eq!(terms.len(), 2);
      assert_eq!(terms[0].expression, "m.subject");
      assert_eq!(terms[0].direction, "ASC");
      assert_eq!(terms[1].expression, "m.received_at");
      assert_eq!(terms[1].direction, "DESC");
   }

   #[test]
   fn has_keyword_sort_compiles() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(serde_json::json!([
          {"property": "hasKeyword", "keyword": "$pinned", "isAscending": false},
          {"property": "receivedAt", "isAscending": false}
      ]))
      .unwrap();
      let terms = email_sort_terms(Some(&comparators)).unwrap();

      assert_eq!(terms.len(), 2);
      assert_eq!(terms[0].expression, r#"(strpos(m.flags_json, '"$pinned"') > 0)"#);
      assert_eq!(terms[0].direction, "DESC");
      assert_eq!(terms[1].expression, "m.received_at");
   }

   #[test]
   fn unsupported_email_sort_arguments_are_rejected() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(
         serde_json::json!([{"property": "subject", "collation": "i;unicode-casemap"}]),
      )
      .unwrap();
      assert!(matches!(
         email_sort_terms(Some(&comparators)),
         Err(MethodError::UnsupportedSort)
      ));
   }
}
