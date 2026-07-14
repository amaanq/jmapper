//! Mailbox/* method handlers.

use std::{
   cmp::Ordering,
   collections::{
      HashMap,
      HashSet,
   },
   iter,
};

use imap_sync::db::{
   self,
   StateKind as DbStateKind,
};
use jmap_protocol::{
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
      State,
   },
   mailbox::{
      Mailbox,
      MailboxFilter,
      MailboxRights,
   },
   methods::{
      ChangesRequest,
      ChangesResponse,
      GetRequest,
      GetResponse,
   },
};
use tokio_postgres::types::ToSql;

use super::{
   MethodResult,
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
use crate::state::{
   AccountInfo,
   AppState,
};

#[derive(serde::Deserialize)]
struct MailboxQueryArgs {
   #[serde(rename = "accountId")]
   account_id:      AccountId,
   #[serde(default)]
   filter:          Option<Filter<MailboxFilter>>,
   #[serde(default)]
   sort:            Option<Vec<Comparator>>,
   #[serde(default)]
   position:        Option<i64>,
   #[serde(default)]
   limit:           Option<u32>,
   #[serde(default, rename = "calculateTotal")]
   calculate_total: bool,
   #[serde(default)]
   anchor:          Option<Id>,
   #[serde(default, rename = "anchorOffset")]
   anchor_offset:   Option<i64>,
   #[serde(default, rename = "sortAsTree")]
   sort_as_tree:    bool,
   #[serde(default, rename = "filterAsTree")]
   filter_as_tree:  bool,
}

#[derive(serde::Deserialize)]
struct MailboxQueryChangesArgs {
   #[serde(rename = "accountId")]
   account_id:        AccountId,
   #[serde(default)]
   filter:            Option<Filter<MailboxFilter>>,
   #[serde(default)]
   sort:              Option<Vec<Comparator>>,
   #[serde(rename = "sinceQueryState")]
   since_query_state: State,
   #[serde(rename = "maxChanges", default)]
   max_changes:       Option<u32>,
   #[serde(rename = "upToId", default)]
   up_to_id:          Option<Id>,
   #[serde(rename = "calculateTotal", default)]
   calculate_total:   bool,
   #[serde(default, rename = "sortAsTree")]
   sort_as_tree:      bool,
   #[serde(default, rename = "filterAsTree")]
   filter_as_tree:    bool,
}

/// # Errors
///
/// Returns [`MethodError`] if the request arguments are malformed, the
/// authenticated account does not match `accountId`, the requested id count
/// exceeds the server limit, or loading mailboxes or the mailbox state from the
/// database fails.
pub async fn get(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let req = serde_json::from_value::<GetRequest>(args)
      .map_err(|err| bad_args(format!("invalid Mailbox/get args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   if let Some(ids) = req.ids.as_ref() {
      enforce_get_limit(ids.len())?;
   }

   let rows = db::list_mailboxes(state.pool(), account_id)
      .await
      .map_err(|err| server_fail(format!("loading mailboxes: {err}")))?;
   if req.ids.is_none() {
      enforce_get_limit(rows.len())?;
   }

   let mut out = Vec::new();
   let mut not_found = Vec::new();
   if let Some(ids) = req.ids.as_ref() {
      for id in ids {
         match rows.iter().find(|row| row.id.as_str() == id.as_ref()) {
            Some(row) => out.push(mailbox_from_row(row)),
            None => not_found.push(id.clone()),
         }
      }
   } else {
      out.extend(rows.iter().map(mailbox_from_row));
   }

   let state_val = cached_state(state, account_id, DbStateKind::Mailbox).await?;

   let mut resp_value = serde_json::to_value(GetResponse::<Mailbox> {
      account_id: AccountId(account_id.to_owned()),
      state: state_val,
      list: out,
      not_found,
   })
   .map_err(|err| server_fail(err.to_string()))?;

   // `id` is always returned even when the client selects properties.
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
   Ok(resp_value)
}

/// # Errors
///
/// Returns [`MethodError`] if the filter references unsupported fields, the
/// arguments are malformed, the authenticated account does not match
/// `accountId`, the sort comparators are unsupported, the anchor is not found,
/// or a database query fails.
pub async fn query(state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   if args.get("filter").is_some_and(|filter| {
      has_unsupported_fields(filter, &[
         "parentId",
         "name",
         "role",
         "hasAnyRole",
         "isSubscribed",
      ])
   }) {
      return Err(MethodError::UnsupportedFilter);
   }
   let req = serde_json::from_value::<MailboxQueryArgs>(args)
      .map_err(|err| bad_args(format!("invalid Mailbox/query args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;

   let order_clause = mailbox_order_clause(req.sort.as_deref())?;
   if req.sort_as_tree || req.filter_as_tree {
      return tree_query(state, account_id, &req).await;
   }
   let client = pg(state).await?;

   // Filter and paginate in SQL so memory use is independent of mailbox count.
   let mut wheres = vec!["account_id = ?".into()];
   let mut binds = vec![account_id.to_owned()];
   if let Some(filter) = req.filter.as_ref() {
      let compiled = compile_mailbox_filter_tree(filter);
      wheres.push(compiled.where_clause);
      binds.extend(compiled.binds);
   }

   let where_sql = wheres.join(" AND ");
   let needs_count = req.calculate_total
      || (req.anchor.is_none() && req.position.is_some_and(|position| position < 0));
   let count = if needs_count {
      let count_sql = format!("SELECT COUNT(*) FROM mailboxes WHERE {where_sql}");
      let params = binds
         .iter()
         .map(|value| value as &(dyn ToSql + Sync))
         .collect::<Vec<&(dyn ToSql + Sync)>>();
      Some(
         client
            .query_one(&pg_numbered(&count_sql), &params)
            .await
            .map_err(|err| server_fail(format!("Mailbox/query count: {err}")))?
            .get::<_, i64>(0),
      )
   } else {
      None
   };
   let (limit, response_limit) = query_limit(req.limit, 500);
   let sql_limit = i64::try_from(limit).unwrap_or(i64::MAX);
   let (ids, position) = if let Some(anchor) = req.anchor.as_ref() {
      // Anchor pagination (RFC 8620 §5.5) — folder counts are small, so
      // fetching the whole ordered id list is the simple correct answer.
      let sql = format!("SELECT id FROM mailboxes WHERE {where_sql} ORDER BY {order_clause}");
      let params = binds
         .iter()
         .map(|value| value as &(dyn ToSql + Sync))
         .collect::<Vec<&(dyn ToSql + Sync)>>();
      let all = client
         .query(&pg_numbered(&sql), &params)
         .await
         .map_err(|err| server_fail(format!("Mailbox/query: {err}")))?
         .into_iter()
         .map(|row| row.get::<_, String>(0))
         .collect::<Vec<String>>();
      let idx = all
         .iter()
         .position(|id| id == anchor.as_ref())
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
      let sql = format!(
         "SELECT id FROM mailboxes WHERE {where_sql} ORDER BY {order_clause} LIMIT ? OFFSET ?",
      );
      let mut params = binds
         .iter()
         .map(|value| value as &(dyn ToSql + Sync))
         .collect::<Vec<&(dyn ToSql + Sync)>>();
      params.push(&sql_limit);
      params.push(&sql_position);
      let ids = client
         .query(&pg_numbered(&sql), &params)
         .await
         .map_err(|err| server_fail(format!("Mailbox/query: {err}")))?
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

   let state_val = cached_state(state, account_id, DbStateKind::Mailbox).await?;
   let mut out = serde_json::json!({
       "accountId": account_id,
       "queryState": state_val,
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

/// # Errors
///
/// Returns [`MethodError`] if the filter references unsupported fields, the
/// arguments are malformed, the authenticated account does not match
/// `accountId`, or the sort comparators are unsupported. Otherwise always
/// returns [`MethodError::CannotCalculateChanges`], since query-change tracking
/// is not implemented.
pub fn query_changes(
   _state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   if args.get("filter").is_some_and(|filter| {
      has_unsupported_fields(filter, &[
         "parentId",
         "name",
         "role",
         "hasAnyRole",
         "isSubscribed",
      ])
   }) {
      return Err(MethodError::UnsupportedFilter);
   }
   let req = serde_json::from_value::<MailboxQueryChangesArgs>(args)
      .map_err(|error| bad_args(format!("invalid Mailbox/queryChanges args: {error}")))?;
   require_auth_match(auth, req.account_id.as_ref())?;
   mailbox_order_clause(req.sort.as_deref())?;
   let _ = (
      req.filter,
      req.since_query_state,
      req.max_changes,
      req.up_to_id,
      req.calculate_total,
      req.sort_as_tree,
      req.filter_as_tree,
   );
   Err(MethodError::CannotCalculateChanges)
}

struct CompiledMailboxFilter {
   where_clause: String,
   binds:        Vec<String>,
}

fn compile_mailbox_filter_tree(filter: &Filter<MailboxFilter>) -> CompiledMailboxFilter {
   match filter {
      Filter::Condition(condition) => {
         let mut wheres = Vec::<String>::new();
         let mut binds = Vec::<String>::new();
         compile_mailbox_filter(condition, &mut wheres, &mut binds);
         CompiledMailboxFilter {
            where_clause: if wheres.is_empty() {
               "1".into()
            } else {
               wheres.join(" AND ")
            },
            binds,
         }
      },
      Filter::Operator {
         operator,
         conditions,
      } => {
         let mut parts = Vec::<String>::with_capacity(conditions.len());
         let mut binds = Vec::<String>::new();
         for condition in conditions {
            let compiled = compile_mailbox_filter_tree(condition);
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
         CompiledMailboxFilter {
            where_clause,
            binds,
         }
      },
   }
}

async fn tree_query(state: &AppState, account_id: &str, req: &MailboxQueryArgs) -> MethodResult {
   let all_rows = db::list_mailboxes(state.pool(), account_id)
      .await
      .map_err(|error| server_fail(format!("loading mailboxes: {error}")))?;
   let by_id = all_rows
      .iter()
      .map(|row| (row.id.as_str(), row))
      .collect::<HashMap<_, _>>();
   let mut rows = all_rows
      .iter()
      .filter(|row| {
         req.filter.as_ref().is_none_or(|filter| {
            if req.filter_as_tree {
               mailbox_path(row, &by_id)
                  .iter()
                  .all(|ancestor| mailbox_matches(ancestor, filter))
            } else {
               mailbox_matches(row, filter)
            }
         })
      })
      .cloned()
      .collect::<Vec<_>>();
   if req.sort_as_tree {
      rows.sort_by(|left, right| mailbox_tree_compare(left, right, &by_id, req.sort.as_deref()));
   } else {
      rows.sort_by(|left, right| mailbox_row_compare(left, right, &by_id, req.sort.as_deref()));
   }

   let total = rows.len();
   let position = if let Some(anchor) = req.anchor.as_ref() {
      let index = rows
         .iter()
         .position(|row| row.id == anchor.as_ref())
         .ok_or(MethodError::AnchorNotFound)?;
      query_anchor_position(index, req.anchor_offset)
   } else {
      query_position(req.position, total)
   };
   let (limit, response_limit) = query_limit(req.limit, 500);
   let ids = rows
      .into_iter()
      .skip(position)
      .take(limit)
      .map(|row| Id(row.id))
      .collect::<Vec<_>>();
   let mut output = serde_json::json!({
       "accountId": account_id,
       "queryState": cached_state(state, account_id, DbStateKind::Mailbox).await?,
       "canCalculateChanges": false,
       "position": i64::try_from(position).unwrap_or(i64::MAX),
       "ids": ids,
   });
   if req.calculate_total {
      output["total"] = serde_json::Value::from(u64::try_from(total).unwrap_or(u64::MAX));
   }
   if let Some(limit) = response_limit {
      output["limit"] = serde_json::Value::from(limit);
   }
   Ok(output)
}

fn mailbox_matches(row: &db::MailboxRow, filter: &Filter<MailboxFilter>) -> bool {
   match filter {
      Filter::Operator {
         operator,
         conditions,
      } => {
         match operator {
            FilterOp::And => {
               conditions
                  .iter()
                  .all(|condition| mailbox_matches(row, condition))
            },
            FilterOp::Or => {
               conditions
                  .iter()
                  .any(|condition| mailbox_matches(row, condition))
            },
            FilterOp::Not => {
               !conditions
                  .iter()
                  .any(|condition| mailbox_matches(row, condition))
            },
         }
      },
      Filter::Condition(condition) => {
         condition.parent_id.as_ref().is_none_or(|parent_id| {
            parent_id.as_ref().map(AsRef::as_ref) == row.parent_id.as_deref()
         }) && condition
            .name
            .as_ref()
            .is_none_or(|name| row.name.to_lowercase().contains(&name.to_lowercase()))
            && condition
               .role
               .as_ref()
               .is_none_or(|role| role.as_ref() == row.role.as_ref())
            && condition
               .has_any_role
               .is_none_or(|has_any_role| row.role.is_some() == has_any_role)
            && condition
               .is_subscribed
               .is_none_or(|is_subscribed| is_subscribed)
      },
   }
}

fn mailbox_path<'a>(
   row: &'a db::MailboxRow,
   by_id: &HashMap<&'a str, &'a db::MailboxRow>,
) -> Vec<&'a db::MailboxRow> {
   let mut path = Vec::new();
   let mut seen = HashSet::new();
   let mut current = Some(row);
   while let Some(mailbox) = current {
      if !seen.insert(mailbox.id.as_str()) {
         break;
      }
      path.push(mailbox);
      current = mailbox
         .parent_id
         .as_deref()
         .and_then(|parent_id| by_id.get(parent_id).copied());
   }
   path.reverse();
   path
}

fn mailbox_tree_compare(
   left: &db::MailboxRow,
   right: &db::MailboxRow,
   by_id: &HashMap<&str, &db::MailboxRow>,
   sort: Option<&[Comparator]>,
) -> Ordering {
   let left_path = mailbox_path(left, by_id);
   let right_path = mailbox_path(right, by_id);
   let common = left_path
      .iter()
      .zip(&right_path)
      .take_while(|(left, right)| left.id == right.id)
      .count();
   if common == left_path.len() || common == right_path.len() {
      return left_path.len().cmp(&right_path.len());
   }
   mailbox_row_compare(left_path[common], right_path[common], by_id, sort)
      .then_with(|| mailbox_row_compare(left, right, by_id, sort))
}

fn mailbox_row_compare(
   left: &db::MailboxRow,
   right: &db::MailboxRow,
   by_id: &HashMap<&str, &db::MailboxRow>,
   sort: Option<&[Comparator]>,
) -> Ordering {
   let Some(comparators) = sort.filter(|comparators| !comparators.is_empty()) else {
      return left
         .sort_order
         .cmp(&right.sort_order)
         .then_with(|| left.name.cmp(&right.name))
         .then_with(|| left.id.cmp(&right.id));
   };
   for comparator in comparators {
      let ordering = match comparator.property.as_str() {
         "sortOrder" => left.sort_order.cmp(&right.sort_order),
         "name" => left.name.cmp(&right.name),
         "parent/name" => {
            let parent_name = |row: &db::MailboxRow| {
               row.parent_id
                  .as_deref()
                  .and_then(|parent_id| by_id.get(parent_id))
                  .map_or("", |parent| parent.name.as_str())
            };
            parent_name(left).cmp(parent_name(right))
         },
         "totalEmails" => left.total_emails.cmp(&right.total_emails),
         _ => Ordering::Equal,
      };
      let ordering = if comparator.is_ascending {
         ordering
      } else {
         ordering.reverse()
      };
      if !ordering.is_eq() {
         return ordering;
      }
   }
   let ordering = left.id.cmp(&right.id);
   if comparators[0].is_ascending {
      ordering
   } else {
      ordering.reverse()
   }
}

fn mailbox_order_clause(comparators: Option<&[Comparator]>) -> Result<String, MethodError> {
   let Some(comparators) = comparators.filter(|comparators| !comparators.is_empty()) else {
      return Ok("sort_order ASC, name ASC, id ASC".into());
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
            "sortOrder" => "sort_order",
            "name" => "name",
            "parent/name" => {
               "COALESCE((SELECT parent.name FROM mailboxes parent WHERE parent.account_id = \
                mailboxes.account_id AND parent.id = mailboxes.parent_id), '')"
            },
            "totalEmails" => "total_emails",
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

fn compile_mailbox_filter(
   filter: &MailboxFilter,
   wheres: &mut Vec<String>,
   binds: &mut Vec<String>,
) {
   if let Some(pid) = filter.parent_id.as_ref() {
      match pid {
         Some(id) => {
            wheres.push("parent_id = ?".into());
            binds.push(id.0.clone());
         },
         None => {
            wheres.push("parent_id IS NULL".into());
         },
      }
   }
   if let Some(name_substr) = filter.name.as_ref() {
      wheres.push("name ILIKE ? ESCAPE '\\'".into());
      binds.push(format!("%{}%", escape_like(name_substr)));
   }
   if let Some(role) = filter.role.as_ref() {
      match role {
         Some(role_name) => {
            wheres.push("role = ?".into());
            binds.push(role_name.clone());
         },
         None => {
            wheres.push("role IS NULL".into());
         },
      }
   }
   if let Some(has_any) = filter.has_any_role {
      wheres.push(if has_any {
         "role IS NOT NULL".into()
      } else {
         "role IS NULL".into()
      });
   }
   if let Some(sub) = filter.is_subscribed
      && !sub
   {
      wheres.push("0 = 1".into());
   }
}

/// In-memory predicate that mirrors the SQL compile path in `Email/query`:
///
/// # Errors
///
/// Returns [`MethodError`] if the arguments are malformed, the authenticated
/// account does not match `accountId`, `sinceState` is not a valid `u64`, or
/// fetching the current state from the database fails. Returns
/// [`MethodError::CannotCalculateChanges`] once the state has advanced past
/// `sinceState`.
pub async fn changes(
   state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   let req = serde_json::from_value::<ChangesRequest>(args)
      .map_err(|err| bad_args(format!("invalid Mailbox/changes args: {err}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;

   let since = req.since_state.as_ref().parse::<u64>().map_err(|_| {
      bad_args(format!(
         "sinceState {:?} is not a valid u64",
         req.since_state.as_ref()
      ))
   })?;
   let cur = db::get_state(state.pool(), account_id)
      .await
      .map_err(|err| server_fail(format!("state fetch: {err}")))?;
   let cur_mod = cur.mailbox_modseq as u64;

   if since == cur_mod {
      let resp = ChangesResponse {
         account_id:       AccountId(account_id.to_owned()),
         old_state:        State(since.to_string()),
         new_state:        State(cur_mod.to_string()),
         has_more_changes: false,
         created:          vec![],
         updated:          vec![],
         destroyed:        vec![],
      };
      let mut value = serde_json::to_value(resp).map_err(|error| server_fail(error.to_string()))?;
      value["updatedProperties"] = serde_json::Value::Null;
      return Ok(value);
   }
   Err(MethodError::CannotCalculateChanges)
}

fn mailbox_from_row(row: &db::MailboxRow) -> Mailbox {
   Mailbox {
      id:             Id(row.id.clone()),
      name:           row.name.clone(),
      parent_id:      row.parent_id.clone().map(Id),
      role:           row.role.clone(),
      sort_order:     row.sort_order as u32,
      total_emails:   row.total_emails as u64,
      unread_emails:  row.unread_emails as u64,
      total_threads:  row.total_threads as u64,
      unread_threads: row.unread_threads as u64,
      my_rights:      MailboxRights::writable(row.role.is_some()),
      is_subscribed:  true,
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn mailbox_name_filter_is_case_insensitive() {
      let mut wheres = Vec::new();
      let mut binds = Vec::new();
      compile_mailbox_filter(
         &MailboxFilter {
            name: Some("Inbox".into()),
            ..MailboxFilter::default()
         },
         &mut wheres,
         &mut binds,
      );

      assert_eq!(wheres, ["name ILIKE ? ESCAPE '\\'"]);
      assert_eq!(binds.len(), 1);
   }

   #[test]
   fn mailbox_sort_keeps_secondary_comparators() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(serde_json::json!([
          {"property": "sortOrder", "isAscending": true},
          {"property": "name", "isAscending": false}
      ]))
      .unwrap();

      assert_eq!(
         mailbox_order_clause(Some(&comparators)).unwrap(),
         "sort_order ASC, name DESC, id ASC"
      );
   }

   #[test]
   fn mailbox_sort_rejects_unknown_arguments() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(serde_json::json!([
         {"property": "name", "locale": "en"}
      ]))
      .unwrap();

      assert!(matches!(
         mailbox_order_clause(Some(&comparators)),
         Err(MethodError::UnsupportedSort)
      ));
   }

   #[test]
   fn mailbox_sort_supports_parent_name() {
      let comparators = serde_json::from_value::<Vec<Comparator>>(serde_json::json!([
         {"property": "parent/name"}
      ]))
      .unwrap();

      let clause = mailbox_order_clause(Some(&comparators)).unwrap();
      assert!(clause.contains("SELECT parent.name"));
      assert!(clause.ends_with("id ASC"));
   }
}
