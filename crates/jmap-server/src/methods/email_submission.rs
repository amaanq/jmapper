//! `EmailSubmission`/* methods (RFC 8621 §7).

use std::{
   collections::{
      HashMap,
      HashSet,
   },
   iter,
   time::Duration,
};

use imap_sync::{
   account::AccountRequest,
   db::{
      self,
      StateKind as DbStateKind,
   },
};
use jmap_protocol::{
   email::EmailAddress,
   error::MethodError,
   filter::{
      Comparator,
      Filter,
      FilterOp,
      has_unsupported_fields,
   },
   ids::{
      AccountId,
      Id,
   },
   methods::{
      ChangesRequest,
      ChangesResponse,
      QueryChangesRequest,
      QueryRequest,
   },
   session::SubmissionCapability,
};
use jmapper_codegen::{
   queries,
   queries::submissions::SubmissionRow,
};
use sha1::{
   Digest as _,
   Sha1,
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
   cached_state_row,
   enforce_get_limit,
   enforce_set_limit,
   ids_or_null,
   object_or_null,
   pg,
   publish_imap_state_changes,
   query_anchor_position,
   query_limit,
   query_position,
   require_auth_match,
   server_fail,
   state_value,
};
use crate::{
   methods::email_set,
   state::{
      AccountInfo,
      AppState,
   },
};

#[derive(Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
struct SubmissionFilter {
   undo_status:  Option<String>,
   email_ids:    Option<Vec<String>>,
   identity_ids: Option<Vec<String>>,
   thread_ids:   Option<Vec<String>>,
   before:       Option<chrono::DateTime<chrono::Utc>>,
   after:        Option<chrono::DateTime<chrono::Utc>>,
}

struct CompiledSubmissionFilter {
   where_clause: String,
   binds:        Vec<SqlParam>,
}

/// Per-rcpt deliveryStatus from the smarthost's accept line. `queued` is the
/// honest value: the smarthost took responsibility; final delivery is out of
/// our sight.
pub(crate) fn delivery_status_json(rcpt_to: &[String], smtp_reply: &str) -> String {
   let mut map = serde_json::Map::new();
   for rcpt in rcpt_to {
      map.insert(
         rcpt.clone(),
         serde_json::json!({
             "smtpReply": smtp_reply,
             "delivered": "queued",
             "displayed": "unknown",
         }),
      );
   }
   serde_json::Value::Object(map).to_string()
}

fn submission_to_json(row: &SubmissionRow) -> serde_json::Value {
   serde_json::json!({
       "id": row.id,
       "identityId": row.identity_id,
       "emailId": row.email_id,
       "threadId": row.thread_id,
       "envelope": serde_json::from_str::<serde_json::Value>(&row.envelope_json)
           .unwrap_or(serde_json::Value::Null),
       "sendAt": chrono::DateTime::<chrono::Utc>::from_timestamp(row.send_at, 0)
           .unwrap_or_else(chrono::Utc::now)
           .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
       // 'sending' is a scheduler-internal claim marker, not a spec value.
       "undoStatus": if row.undo_status == "sending" { "pending" } else { row.undo_status.as_str() },
       "deliveryStatus": row
           .delivery_status_json
           .as_deref()
           .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
           .unwrap_or(serde_json::Value::Null),
       "dsnBlobIds": [],
       "mdnBlobIds": [],
   })
}

/// RFC 8621 §7.2 — `EmailSubmission/get`.
///
/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, the
/// authenticated account does not match `accountId`, the requested id set
/// exceeds the get limit, or a database query fails.
pub async fn get(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id: AccountId,
      #[serde(default)]
      ids:        Option<Vec<Id>>,
   }
   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid EmailSubmission/get args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   } else {
      let count = pg(state)
         .await?
         .query_one(
            "SELECT COUNT(*) FROM email_submissions WHERE account_id = $1 AND destroyed = 0",
            &[&account_id],
         )
         .await
         .map_err(|error| server_fail(format!("EmailSubmission/get count: {error}")))?
         .get::<_, i64>(0);
      enforce_get_limit(usize::try_from(count).unwrap_or(usize::MAX))?;
   }

   let rows = match req.ids.as_ref() {
      Some(ids) => {
         let ids = ids
            .iter()
            .map(|i| i.as_ref().to_owned())
            .collect::<Vec<String>>();
         queries::submissions::get_submissions_by_ids()
            .bind(&pg(state).await?, &account_id, &ids)
            .all()
            .await
            .map_err(|err| server_fail(format!("EmailSubmission/get: {err}")))?
      },
      None => {
         queries::submissions::list_submissions()
            .bind(&pg(state).await?, &account_id)
            .all()
            .await
            .map_err(|err| server_fail(format!("EmailSubmission/get: {err}")))?
      },
   };

   let found = rows
      .iter()
      .map(|row| row.id.as_str())
      .collect::<HashSet<&str>>();
   let not_found = req
      .ids
      .iter()
      .flatten()
      .map(AsRef::as_ref)
      .filter(|id| !found.contains(*id))
      .collect::<Vec<&str>>();

   Ok(serde_json::json!({
       "accountId": account_id,
       "state": cached_state(state, account_id, DbStateKind::Submission).await?,
       "list": rows.iter().map(submission_to_json).collect::<Vec<_>>(),
       "notFound": not_found,
   }))
}

/// RFC 8621 §7.4 — `EmailSubmission/changes`.
///
/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, the account
/// does not match `accountId`, or the requested `sinceState` differs from the
/// current state — [`MethodError::CannotCalculateChanges`], since submission
/// changes cannot be reconstructed from tombstones.
pub async fn changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let req = serde_json::from_value::<ChangesRequest>(args)
      .map_err(|err| bad_args(format!("invalid EmailSubmission/changes args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   let current = cached_state(state, account_id, DbStateKind::Submission).await?;
   if req.since_state != current {
      // Tombstones preserve destroyed ids, but rows only retain their most
      // recent modseq. Without the original creation modseq we cannot tell
      // created from updated, or omit an object created and destroyed in
      // the requested window. Refetch instead of fabricating buckets.
      return Err(MethodError::CannotCalculateChanges);
   }

   let resp = ChangesResponse {
      account_id:       AccountId(account_id.to_owned()),
      old_state:        req.since_state,
      new_state:        current,
      has_more_changes: false,
      created:          vec![],
      updated:          vec![],
      destroyed:        vec![],
   };
   serde_json::to_value(resp).map_err(|err| server_fail(err.to_string()))
}

/// RFC 8621 §7.3 — `EmailSubmission/query`.
///
/// # Errors
///
/// Returns [`MethodError`] if the filter references unsupported fields, the
/// arguments fail to deserialize, the account does not match, the sort refers
/// to an unknown property, the anchor id is not present in the result set, or a
/// database query fails.
pub async fn query(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   validate_submission_filter_fields(&args)?;
   let req = serde_json::from_value::<QueryRequest<SubmissionFilter>>(args)
      .map_err(|err| bad_args(format!("invalid EmailSubmission/query args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   let mut wheres = vec!["account_id = ?".to_owned(), "destroyed = 0".to_owned()];
   let mut binds = vec![SqlParam::Str(account_id.to_owned())];
   if let Some(filter) = req.filter.as_ref() {
      let compiled = compile_submission_filter(filter);
      wheres.push(compiled.where_clause);
      binds.extend(compiled.binds);
   }

   let order_clause = submission_order_clause(req.sort.as_deref())?;
   let where_sql = wheres.join(" AND ");
   let sql = format!("SELECT id FROM email_submissions WHERE {where_sql} ORDER BY {order_clause}");
   let params = binds
      .iter()
      .map(SqlParam::as_dyn)
      .collect::<Vec<&(dyn ToSql + Sync)>>();
   let all = pg(state)
      .await?
      .query(&super::pg_numbered(&sql), &params)
      .await
      .map_err(|err| server_fail(format!("EmailSubmission/query: {err}")))?
      .into_iter()
      .map(|row| row.get::<_, String>(0))
      .collect::<Vec<String>>();

   let total = all.len();
   let (limit, response_limit) = query_limit(req.limit, 500);
   let start = match req.anchor.as_ref() {
      Some(anchor) => {
         let idx = all
            .iter()
            .position(|id| id == anchor.as_ref())
            .ok_or(MethodError::AnchorNotFound)?;
         query_anchor_position(idx, req.anchor_offset)
      },
      None => query_position(req.position, total),
   };
   let ids = all
      .into_iter()
      .skip(start)
      .take(limit)
      .map(Id)
      .collect::<Vec<Id>>();

   let mut out = serde_json::json!({
       "accountId": account_id,
       "queryState": cached_state(state, account_id, DbStateKind::Submission).await?,
       "canCalculateChanges": false,
       "position": start as i64,
       "ids": ids,
   });
   if req.calculate_total {
      out["total"] = serde_json::Value::from(total as u64);
   }
   if let Some(limit) = response_limit {
      out["limit"] = serde_json::Value::from(limit);
   }
   Ok(out)
}

fn validate_submission_filter_fields(args: &serde_json::Value) -> Result<(), MethodError> {
   if args.get("filter").is_some_and(|filter| {
      has_unsupported_fields(filter, &[
         "undoStatus",
         "emailIds",
         "identityIds",
         "threadIds",
         "before",
         "after",
      ])
   }) {
      Err(MethodError::UnsupportedFilter)
   } else {
      Ok(())
   }
}

fn compile_submission_filter(filter: &Filter<SubmissionFilter>) -> CompiledSubmissionFilter {
   match filter {
      Filter::Condition(condition) => compile_submission_condition(condition),
      Filter::Operator {
         operator,
         conditions,
      } => {
         let mut parts = Vec::<String>::with_capacity(conditions.len());
         let mut binds = Vec::<SqlParam>::new();
         for condition in conditions {
            let compiled = compile_submission_filter(condition);
            parts.push(format!("({})", compiled.where_clause));
            binds.extend(compiled.binds);
         }
         let where_clause = match operator {
            FilterOp::And | FilterOp::Not if parts.is_empty() => "1".into(),
            FilterOp::And => parts.join(" AND "),
            FilterOp::Or if parts.is_empty() => "0".into(),
            FilterOp::Or => parts.join(" OR "),
            FilterOp::Not => format!("NOT ({})", parts.join(" OR ")),
         };
         CompiledSubmissionFilter {
            where_clause,
            binds,
         }
      },
   }
}

fn compile_submission_condition(condition: &SubmissionFilter) -> CompiledSubmissionFilter {
   let mut wheres = Vec::<String>::new();
   let mut binds = Vec::<SqlParam>::new();
   if let Some(status) = condition.undo_status.as_ref() {
      if status == "pending" {
         // `sending` is the scheduler's internal claim marker; it remains
         // `pending` on the JMAP wire until delivery finishes.
         wheres.push("undo_status IN ('pending', 'sending')".into());
      } else {
         wheres.push("undo_status = ?".into());
         binds.push(SqlParam::Str(status.clone()));
      }
   }
   for (ids, column) in [
      (&condition.email_ids, "email_id"),
      (&condition.identity_ids, "identity_id"),
      (&condition.thread_ids, "thread_id"),
   ] {
      if let Some(ids) = ids {
         if ids.is_empty() {
            wheres.push("0".into());
         } else {
            wheres.push(format!("{column} IN ({})", vec!["?"; ids.len()].join(",")));
            binds.extend(ids.iter().cloned().map(SqlParam::Str));
         }
      }
   }
   if let Some(before) = condition.before {
      wheres.push("send_at < ?".into());
      binds.push(SqlParam::Int(before.timestamp()));
   }
   if let Some(after) = condition.after {
      wheres.push("send_at >= ?".into());
      binds.push(SqlParam::Int(after.timestamp()));
   }
   CompiledSubmissionFilter {
      where_clause: if wheres.is_empty() {
         "1".into()
      } else {
         wheres.join(" AND ")
      },
      binds,
   }
}

fn submission_order_clause(comparators: Option<&[Comparator]>) -> Result<String, MethodError> {
   let Some(comparators) = comparators.filter(|comparators| !comparators.is_empty()) else {
      return Ok("send_at ASC, id ASC".into());
   };

   let tie_direction = if comparators[0].is_ascending {
      "ASC"
   } else {
      "DESC"
   };
   comparators
      .iter()
      .map(|comparator| {
         if comparator.collation.is_some() || !comparator.extra.is_empty() {
            return Err(MethodError::UnsupportedSort);
         }
         let column = match comparator.property.as_str() {
            "sentAt" => "send_at",
            "emailId" => "email_id",
            "threadId" => "thread_id",
            _ => return Err(MethodError::UnsupportedSort),
         };
         let direction = if comparator.is_ascending {
            "ASC"
         } else {
            "DESC"
         };
         Ok(format!("{column} {direction}"))
      })
      .chain(iter::once(Ok(format!("id {tie_direction}"))))
      .collect::<Result<Vec<_>, _>>()
      .map(|terms| terms.join(", "))
}

/// RFC 8621 §7.6 — `EmailSubmission/queryChanges`.
///
/// # Errors
///
/// Always fails with [`MethodError::CannotCalculateChanges`], but validates
/// first: an unsupported filter field yields
/// [`MethodError::UnsupportedFilter`], malformed arguments yield a bad-args
/// error, a mismatched account is rejected, and an unknown sort property yields
/// [`MethodError::UnsupportedSort`].
pub fn query_changes(
   _state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_submission_filter_fields(&args)?;
   let req = serde_json::from_value::<QueryChangesRequest<SubmissionFilter>>(args)
      .map_err(|err| bad_args(format!("invalid EmailSubmission/queryChanges args: {err}")))?;
   require_auth_match(auth, req.account_id.as_ref())?;
   submission_order_clause(req.sort.as_deref())?;
   Err(MethodError::CannotCalculateChanges)
}

/// `EmailSubmission/set`, returning the submission response plus the
/// implicit `Email/set` response when `onSuccess*` arguments were present
/// and at least one submission succeeded.
///
/// # Errors
///
/// Returns [`MethodError`] if the arguments fail to deserialize, the account
/// does not match, the create/update/destroy totals exceed the set limit, the
/// `ifInState` guard does not match the current state, or a database access
/// fails while reading state or applying the implicit `Email/set`.
pub async fn set_with_implicit(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> Result<(serde_json::Value, Option<serde_json::Value>), MethodError> {
   #[derive(serde::Deserialize)]
   struct Args {
      #[serde(rename = "accountId")]
      account_id:               AccountId,
      #[serde(default, rename = "ifInState")]
      if_in_state:              Option<String>,
      #[serde(default)]
      create:                   Option<HashMap<String, serde_json::Value>>,
      #[serde(default)]
      update:                   Option<HashMap<String, serde_json::Value>>,
      #[serde(default)]
      destroy:                  Option<Vec<String>>,
      #[serde(default, rename = "onSuccessUpdateEmail")]
      on_success_update_email:  Option<HashMap<String, serde_json::Value>>,
      #[serde(default, rename = "onSuccessDestroyEmail")]
      on_success_destroy_email: Option<Vec<String>>,
   }
   let req = serde_json::from_value::<Args>(args)
      .map_err(|err| bad_args(format!("invalid EmailSubmission/set args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   enforce_set_limit(
      req.create.as_ref().map_or(0, HashMap::len),
      req.update.as_ref().map_or(0, HashMap::len),
      req.destroy.as_ref().map_or(0, Vec::len),
   )?;

   let before = cached_state_row(state, account_id).await?;
   let old_state = state_value(&before, DbStateKind::Submission);
   if let Some(expected) = req.if_in_state.as_deref()
      && expected != old_state.as_ref()
   {
      return Err(MethodError::StateMismatch);
   }

   let mut created = serde_json::Map::new();
   let mut not_created = serde_json::Map::new();
   // creationId → (submissionId, emailId) for resolving `#id` references in
   // the onSuccess* arguments.
   let mut created_emails = HashMap::<String, (String, String)>::new();
   if let Some(create) = req.create {
      for (creation_id, payload) in create {
         match apply_create(state, auth, account_id, &payload).await {
            Ok((server_set, email_id)) => {
               let sub_id = server_set["id"].as_str().unwrap_or_default().to_owned();
               created_emails.insert(creation_id.clone(), (sub_id, email_id));
               created.insert(creation_id, server_set);
            },
            Err(set_error) => {
               tracing::warn!(
                  account_id,
                  creation_id,
                  error_type = set_error.get("type").and_then(serde_json::Value::as_str),
                  description = set_error
                     .get("description")
                     .and_then(serde_json::Value::as_str),
                  "email submission create failed",
               );
               not_created.insert(creation_id, set_error);
            },
         }
      }
   }

   let mut updated = serde_json::Map::new();
   let mut not_updated = serde_json::Map::new();
   for (id, patch) in req.update.into_iter().flatten() {
      match apply_cancel(state, account_id, &id, &patch).await {
         Ok(()) => {
            updated.insert(id, serde_json::Value::Null);
         },
         Err(err) => {
            not_updated.insert(id, err);
         },
      }
   }
   let mut destroyed_ids = Vec::<String>::new();
   let mut not_destroyed = serde_json::Map::new();
   for id in req.destroy.into_iter().flatten() {
      match apply_destroy(state, account_id, &id).await {
         Ok(()) => destroyed_ids.push(id),
         Err(err) => {
            not_destroyed.insert(id, err);
         },
      }
   }

   let after = cached_state_row(state, account_id).await?;
   let new_state = state_value(&after, DbStateKind::Submission);
   publish_imap_state_changes(state, account_id, &before, &after);

   let response = serde_json::json!({
       "accountId": account_id,
       "oldState": old_state,
       "newState": new_state,
       "created": object_or_null(created),
       "updated": object_or_null(updated),
       "destroyed": ids_or_null(destroyed_ids),
       "notCreated": object_or_null(not_created),
       "notUpdated": object_or_null(not_updated),
       "notDestroyed": object_or_null(not_destroyed),
   });

   let implicit = implicit_email_set(
      state,
      account_id,
      &created_emails,
      req.on_success_update_email.as_ref(),
      req.on_success_destroy_email.as_deref(),
   )
   .await?;

   Ok((response, implicit))
}

/// One create entry: validate, derive the envelope, run the SMTP
/// transaction via the account task, record the submission row. Returns the
/// server-set properties and the emailId (for onSuccess* resolution).
async fn apply_create(
   state: &AppState,
   auth: &AccountInfo,
   account_id: &str,
   payload: &serde_json::Value,
) -> Result<(serde_json::Value, String), serde_json::Value> {
   #[derive(serde::Deserialize)]
   struct CreatePayload {
      #[serde(rename = "emailId")]
      email_id:    String,
      #[serde(rename = "identityId")]
      identity_id: String,
      #[serde(default)]
      envelope:    Option<Envelope>,
      #[serde(default, rename = "undoStatus")]
      undo_status: Option<String>,
      #[serde(default, rename = "sendAt")]
      send_at:     Option<chrono::DateTime<chrono::Utc>>,
   }
   #[derive(serde::Deserialize)]
   struct Envelope {
      #[serde(rename = "mailFrom")]
      mail_from: EnvelopeAddress,
      #[serde(rename = "rcptTo")]
      rcpt_to:   Vec<EnvelopeAddress>,
   }
   #[derive(serde::Deserialize)]
   struct EnvelopeAddress {
      email: String,
   }

   let parsed = serde_json::from_value::<CreatePayload>(payload.clone()).map_err(|err| {
      serde_json::json!({
          "type": "invalidProperties",
          "description": format!("invalid EmailSubmission create: {err}"),
      })
   })?;
   let now = chrono::Utc::now();
   // A future sendAt (or an explicit undoStatus of "pending") queues the
   // submission for the scheduler instead of relaying inline. A small skew
   // allowance means "sendAt: now" from a slightly-fast client still sends
   // immediately.
   let delayed_until = match parsed.send_at {
      Some(at) if at > now + chrono::Duration::seconds(5) => {
         let max =
            chrono::Duration::seconds(i64::from(SubmissionCapability::default().max_delayed_send));
         if at > now + max {
            return Err(serde_json::json!({
                "type": "invalidProperties",
                "properties": ["sendAt"],
                "description": format!("sendAt exceeds maxDelayedSend ({}s)", max.num_seconds()),
            }));
         }
         Some(at)
      },
      _ => None,
   };
   if let Some(undo) = parsed.undo_status.as_deref() {
      let consistent = match undo {
         "final" => delayed_until.is_none(),
         "pending" => delayed_until.is_some(),
         _ => false,
      };
      if !consistent {
         return Err(serde_json::json!({
             "type": "invalidProperties",
             "properties": ["undoStatus"],
             "description": "undoStatus must be \"final\" (immediate) or \"pending\" with a future sendAt",
         }));
      }
   }
   if parsed.identity_id != format!("ident-{account_id}") {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["identityId"],
          "description": "unknown identity",
      }));
   }

   let row = state
      .pool()
      .get()
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?
      .query_opt(
         "SELECT thrid, to_json, cc_json, bcc_json FROM messages WHERE account_id = $1 AND msgid \
          = $2",
         &[&account_id, &parsed.email_id],
      )
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?
      .map(|row| {
         (
            row.get::<_, String>(0),
            row.get::<_, Option<String>>(1),
            row.get::<_, Option<String>>(2),
            row.get::<_, Option<String>>(3),
         )
      });
   let Some((thread_id, to_json, cc_json, bcc_json)) = row else {
      return Err(serde_json::json!({
          "type": "invalidProperties",
          "properties": ["emailId"],
          "description": "emailId does not reference a cached message",
      }));
   };

   let (mail_from, rcpt_to) = if let Some(env) = parsed.envelope {
      (
         env.mail_from.email,
         env.rcpt_to
            .into_iter()
            .map(|addr| addr.email)
            .collect::<Vec<_>>(),
      )
   } else {
      // RFC 8621 §7.1: a null envelope is generated from the message —
      // identity address as the return path, To+Cc+Bcc as recipients.
      let mut rcpts = Vec::<String>::new();
      for j in [&to_json, &cc_json, &bcc_json].into_iter().flatten() {
         let addrs = serde_json::from_str::<Vec<EmailAddress>>(j).unwrap_or_default();
         rcpts.extend(addrs.into_iter().map(|addr| addr.email));
      }
      rcpts.sort();
      rcpts.dedup();
      (auth.email.clone(), rcpts)
   };
   if rcpt_to.is_empty() {
      return Err(serde_json::json!({
          "type": "noRecipients",
          "description": "the message has no To, Cc or Bcc addresses and no envelope was given",
      }));
   }

   // Delayed sends carry their own copy of the raw bytes: the standard
   // client flow destroys the draft (onSuccessDestroyEmail) right after
   // create, which CASCADEs the raw_messages row the scheduler would
   // otherwise read at send time.
   let mut smtp_reply = None::<String>;
   let staged_raw = if let Some(at) = delayed_until {
      let raw = fetch_raw_for_staging(state, account_id, &parsed.email_id).await?;
      tracing::info!(
          account_id,
          email_id = %parsed.email_id,
          send_at = %at,
          bytes = raw.len(),
          "queued delayed submission",
      );
      Some(raw)
   } else {
      let tx = state.account_sender(account_id).ok_or_else(|| {
         serde_json::json!({
             "type": "serverUnavailable",
             "description": "sync task is not running for this account",
         })
      })?;
      let (respond, rx) = oneshot::channel();
      tx.send(AccountRequest::SubmitEmail {
            msgid: parsed.email_id.clone(),
            mail_from: mail_from.clone(),
            rcpt_to: rcpt_to.clone(),
            respond,
        })
        .await
        .map_err(
            |_| serde_json::json!({"type": "serverFail", "description": "account task channel closed"}),
        )?;
      // Generous timeout: the account task may need to exit IDLE, fetch the
      // raw body over IMAP, and complete a full SMTP transaction.
      let outcome = time::timeout(Duration::from_secs(120), rx)
            .await
            .map_err(|_| {
                serde_json::json!({"type": "serverFail", "description": "submission timed out"})
            })?
            .map_err(|_| {
                serde_json::json!({"type": "serverFail", "description": "account task dropped submission"})
            })?;
      let reply = outcome.map_err(
         |err| serde_json::json!({"type": "smtpProtocolError", "description": err.to_string()}),
      )?;
      smtp_reply = Some(reply);
      None
   };

   let envelope_json = serde_json::json!({
       "mailFrom": {"email": mail_from},
       "rcptTo": rcpt_to.iter().map(|rcpt| serde_json::json!({"email": rcpt})).collect::<Vec<_>>(),
   });
   let undo_status = if delayed_until.is_some() {
      "pending"
   } else {
      "final"
   };
   let send_at = delayed_until.unwrap_or(now);
   let modseq = db::bump_modseq(state.pool(), account_id, DbStateKind::Submission)
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?;

   // Deterministic id from what makes the submission unique; the modseq
   // disambiguates re-sends of the same message.
   let sub_id = {
      let mut hasher = Sha1::new();
      hasher.update(account_id.as_bytes());
      hasher.update(b"\0");
      hasher.update(parsed.email_id.as_bytes());
      hasher.update(b"\0");
      hasher.update(modseq.to_be_bytes());
      format!("sub-{}", hex::encode(&hasher.finalize()[..10]))
   };
   let delivery_status = smtp_reply
      .as_deref()
      .map(|reply| delivery_status_json(&rcpt_to, reply));
   queries::submissions::insert_submission()
      .bind(
         &state.pool().get().await.map_err(
            |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
         )?,
         &account_id,
         &sub_id.as_str(),
         &parsed.email_id.as_str(),
         &parsed.identity_id.as_str(),
         &Some(thread_id.as_str()),
         &envelope_json.to_string().as_str(),
         &send_at.timestamp(),
         &undo_status,
         &staged_raw.as_deref(),
         &delivery_status.as_deref(),
         &(modseq as i64),
      )
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?;

   let server_set = serde_json::json!({
       "id": sub_id,
       "threadId": thread_id,
       "envelope": envelope_json,
       "sendAt": send_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
       "undoStatus": undo_status,
       "deliveryStatus": delivery_status
           .as_deref()
           .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
           .unwrap_or(serde_json::Value::Null),
       "dsnBlobIds": [],
       "mdnBlobIds": [],
   });
   Ok((server_set, parsed.email_id))
}

/// Build the implicit Email/set response for onSuccessUpdateEmail /
/// onSuccessDestroyEmail (RFC 8621 §7.5). Keys are `#creationId` references
/// or plain submission ids; entries whose submission failed are skipped.
async fn implicit_email_set(
   state: &AppState,
   account_id: &str,
   created: &HashMap<String, (String, String)>,
   update: Option<&HashMap<String, serde_json::Value>>,
   destroy: Option<&[String]>,
) -> Result<Option<serde_json::Value>, MethodError> {
   if update.is_none() && destroy.is_none() {
      return Ok(None);
   }

   let resolve = |key: &str| -> Option<String> {
      let key = key.strip_prefix('#').unwrap_or(key);
      // `#creationId` first; otherwise treat the key as a submission id
      // from this batch (RFC only allows ids created in the same call).
      created
         .get(key)
         .map(|(_, email_id)| email_id.clone())
         .or_else(|| {
            created
               .values()
               .find(|(sub_id, _)| sub_id == key)
               .map(|(_, email_id)| email_id.clone())
         })
   };

   let before = cached_state_row(state, account_id).await?;
   let email_old_state = state_value(&before, DbStateKind::Email);
   let mut updated = serde_json::Map::new();
   let mut not_updated = serde_json::Map::new();
   for (key, patch) in update.into_iter().flatten() {
      let Some(email_id) = resolve(key) else {
         continue;
      };
      match email_set::apply_update(state, account_id, &email_id, patch).await {
         Ok(()) => {
            updated.insert(email_id, serde_json::Value::Null);
         },
         Err(err) => {
            not_updated.insert(email_id, err);
         },
      }
   }
   let mut destroyed = Vec::<String>::new();
   let mut not_destroyed = serde_json::Map::new();
   for key in destroy.into_iter().flatten() {
      let Some(email_id) = resolve(key) else {
         continue;
      };
      match email_set::apply_destroy(state, account_id, &email_id).await {
         Ok(()) => destroyed.push(email_id),
         Err(err) => {
            not_destroyed.insert(email_id, err);
         },
      }
   }

   let after = cached_state_row(state, account_id).await?;
   let email_new_state = state_value(&after, DbStateKind::Email);
   publish_imap_state_changes(state, account_id, &before, &after);

   Ok(Some(serde_json::json!({
       "accountId": account_id,
       "oldState": email_old_state,
       "newState": email_new_state,
       "created": serde_json::Value::Null,
       "updated": object_or_null(updated),
       "destroyed": ids_or_null(destroyed),
       "notCreated": serde_json::Value::Null,
       "notUpdated": object_or_null(not_updated),
       "notDestroyed": object_or_null(not_destroyed),
   })))
}

/// The only legal update: `undoStatus` → "canceled" while the submission is
/// still pending. The UPDATE's WHERE clause is the race guard — if the
/// scheduler claimed the row between our read and write, zero rows change
/// and the client gets `cannotUnsend`.
async fn apply_cancel(
   state: &AppState,
   account_id: &str,
   id: &str,
   patch: &serde_json::Value,
) -> Result<(), serde_json::Value> {
   let obj = patch.as_object().ok_or_else(
      || serde_json::json!({"type": "invalidPatch", "description": "patch must be an object"}),
   )?;
   for (key, val) in obj {
      if key != "undoStatus" || val.as_str() != Some("canceled") {
         return Err(serde_json::json!({
             "type": "invalidProperties",
             "properties": [key],
             "description": "only {\"undoStatus\": \"canceled\"} may be updated",
         }));
      }
   }
   let modseq = db::bump_modseq(state.pool(), account_id, DbStateKind::Submission)
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?;
   let conn = state.pool().get().await.map_err(
      |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
   )?;
   let res = queries::submissions::cancel_pending_submission()
      .bind(&conn, &(modseq as i64), &account_id, &id)
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?;
   if res == 0 {
      let exists = queries::submissions::submission_undo_status()
         .bind(&conn, &account_id, &id)
         .opt()
         .await
         .map_err(
            |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
         )?;
      return Err(match exists {
         None => serde_json::json!({"type": "notFound"}),
         Some(_) => {
            serde_json::json!({
                "type": "cannotUnsend",
                "description": "the message has already been relayed (or is being relayed now)",
            })
         },
      });
   }
   Ok(())
}

/// Destroy tombstones the row (so /changes can report it) rather than
/// deleting it. Pending submissions must be canceled first — destroying a
/// row the scheduler is about to act on would turn destroy into cancel.
async fn apply_destroy(
   state: &AppState,
   account_id: &str,
   id: &str,
) -> Result<(), serde_json::Value> {
   let modseq = db::bump_modseq(state.pool(), account_id, DbStateKind::Submission)
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?;
   let conn = state.pool().get().await.map_err(
      |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
   )?;
   let res = queries::submissions::tombstone_submission()
      .bind(&conn, &(modseq as i64), &account_id, &id)
      .await
      .map_err(
         |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
      )?;
   if res == 0 {
      let exists = queries::submissions::submission_undo_status()
         .bind(&conn, &account_id, &id)
         .opt()
         .await
         .map_err(
            |err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}),
         )?;
      return Err(match exists {
         None => serde_json::json!({"type": "notFound"}),
         Some(_) => {
            serde_json::json!({
                "type": "forbidden",
                "description": "cancel the pending submission before destroying it",
            })
         },
      });
   }
   Ok(())
}

/// Raw RFC 5322 bytes for staging a delayed send, fetching over IMAP first
/// when the lazy body cache is cold. Errors are `SetError` values because the
/// caller folds them into notCreated.
async fn fetch_raw_for_staging(
   state: &AppState,
   account_id: &str,
   email_id: &str,
) -> Result<Vec<u8>, serde_json::Value> {
   let load = || {
      async {
         Ok::<_, serde_json::Value>(
            queries::raw_messages::raw_message_bytes()
                .bind(
                    &state.pool().get().await.map_err(|err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}))?,
                    &account_id,
                    &email_id,
                )
                .opt()
                .await
                .map_err(|err| serde_json::json!({"type": "serverFail", "description": format!("db: {err}")}))?
                .map(|row| row.raw_rfc822),
        )
      }
   };
   if let Some(raw) = load().await? {
      return Ok(raw);
   }
   let tx = state.account_sender(account_id).ok_or_else(|| {
      serde_json::json!({
          "type": "serverUnavailable",
          "description": "sync task is not running for this account",
      })
   })?;
   let (respond, rx) = oneshot::channel();
   tx.send(AccountRequest::FetchBody {
      msgid: email_id.to_owned(),
      respond,
   })
   .await
   .map_err(
      |_| serde_json::json!({"type": "serverFail", "description": "account task channel closed"}),
   )?;
   time::timeout(Duration::from_secs(30), rx)
      .await
      .map_err(
         |_| serde_json::json!({"type": "serverFail", "description": "body fetch timed out"}),
      )?
      .map_err(
         |_| serde_json::json!({"type": "serverFail", "description": "account task dropped fetch"}),
      )?
      .map_err(|err| serde_json::json!({"type": "serverFail", "description": err.to_string()}))?;
   load().await?.ok_or_else(|| {
      serde_json::json!({
          "type": "serverFail",
          "description": "no raw bytes cached after fetch; cannot stage delayed send",
      })
   })
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn submission_sort_keeps_secondary_comparators() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(serde_json::json!([
          {"property": "sentAt", "isAscending": false},
          {"property": "threadId", "isAscending": true},
          {"property": "emailId", "isAscending": true}
      ]))
      .unwrap();

      assert_eq!(
         submission_order_clause(Some(&comparators)).unwrap(),
         "send_at DESC, thread_id ASC, email_id ASC, id DESC"
      );
   }

   #[test]
   fn submission_filter_compiles_boolean_operators() {
      let filter = serde_json::from_value::<Filter<SubmissionFilter>>(serde_json::json!({
          "operator": "OR",
          "conditions": [
              {"undoStatus": "pending"},
              {"emailIds": ["email-a", "email-b"]}
          ]
      }))
      .unwrap();
      let compiled = compile_submission_filter(&filter);

      assert_eq!(
         compiled.where_clause,
         "(undo_status IN ('pending', 'sending')) OR (email_id IN (?,?))"
      );
      assert_eq!(compiled.binds.len(), 2);
   }

   #[test]
   fn submission_sort_rejects_collations() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(
         serde_json::json!([{"property": "emailId", "collation": "i;ascii-casemap"}]),
      )
      .unwrap();

      assert!(matches!(
         submission_order_clause(Some(&comparators)),
         Err(MethodError::UnsupportedSort)
      ));
   }
}
