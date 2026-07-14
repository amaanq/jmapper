//! Static unlimited Quota methods (RFC 9425).

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
   },
   methods::{
      QueryChangesRequest,
      QueryRequest,
      QueryResponse,
   },
};
use serde::{
   Deserialize,
   Serialize,
};

use super::{
   MethodResult,
   bad_args,
   query_anchor_position,
   query_limit,
   query_position,
   require_auth_match,
   server_fail,
   static_object,
};
use crate::state::{
   AccountInfo,
   AppState,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Quota {
   id:            Id,
   #[serde(rename = "resourceType")]
   resource_type: String,
   used:          u64,
   #[serde(rename = "warnLimit")]
   warn_limit:    Option<u64>,
   #[serde(rename = "softLimit")]
   soft_limit:    Option<u64>,
   #[serde(rename = "hardLimit")]
   hard_limit:    Option<u64>,
   name:          String,
   #[serde(skip_serializing_if = "Option::is_none")]
   description:   Option<String>,
   types:         Vec<String>,
   scope:         String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct QuotaFilter {
   name:          Option<String>,
   scope:         Option<String>,
   #[serde(rename = "resourceType")]
   resource_type: Option<String>,
   #[serde(rename = "type")]
   type_name:     Option<String>,
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account.
pub fn get(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let quota = quota(&auth.id);
   static_object::singleton_get(
      args,
      "Quota",
      static_object::state("quota", [auth.id.as_str()]),
      quota.id.clone(),
      quota,
      |account_id| require_auth_match(auth, account_id),
   )
}

/// # Errors
///
/// Returns a [`MethodError`] if the request arguments are malformed or the
/// requested `accountId` does not match the authenticated account.
pub fn changes(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   let mut response = static_object::empty_changes(
      args,
      "Quota",
      static_object::state("quota", [auth.id.as_str()]),
      |account_id| require_auth_match(auth, account_id),
   )?;
   response["updatedProperties"] = serde_json::Value::Null;
   Ok(response)
}

/// # Errors
///
/// Returns a [`MethodError`] if the filter references unsupported fields, the
/// arguments are malformed, the requested `accountId` does not match the
/// authenticated account, the sort is unsupported, or a supplied anchor is not
/// found in the result set.
pub fn query(_state: &AppState, auth: &AccountInfo, args: serde_json::Value) -> MethodResult {
   validate_filter_fields(&args)?;
   let req = serde_json::from_value::<QueryRequest<QuotaFilter>>(args)
      .map_err(|error| bad_args(format!("invalid Quota/query args: {error}")))?;
   let account_id = req.account_id.as_ref();
   require_auth_match(auth, account_id)?;
   validate_sort(req.sort.as_deref())?;
   let quota = quota(account_id);
   let mut ids = req
      .filter
      .as_ref()
      .is_none_or(|filter| quota_matches(&quota, filter))
      .then(|| quota.id.clone())
      .into_iter()
      .collect::<Vec<Id>>();
   let total = ids.len();
   let position = if let Some(anchor) = req.anchor.as_ref() {
      let index = ids
         .iter()
         .position(|id| id == anchor)
         .ok_or(MethodError::AnchorNotFound)?;
      query_anchor_position(index, req.anchor_offset)
   } else {
      query_position(req.position, total)
   };
   let (limit, response_limit) = query_limit(req.limit, 500);
   ids = ids.into_iter().skip(position).take(limit).collect();
   serde_json::to_value(QueryResponse {
      account_id: AccountId(account_id.to_owned()),
      query_state: static_object::state("quota", [account_id]),
      can_calculate_changes: false,
      position: i64::try_from(position).unwrap_or(i64::MAX),
      ids,
      total: req
         .calculate_total
         .then(|| u64::try_from(total).unwrap_or(u64::MAX)),
      limit: response_limit,
   })
   .map_err(|error| server_fail(error.to_string()))
}

/// # Errors
///
/// Returns a [`MethodError`] if the filter references unsupported fields, the
/// arguments are malformed, the requested `accountId` does not match the
/// authenticated account, or the sort is unsupported. Quota state cannot be
/// diffed, so a well-formed request still returns
/// [`MethodError::CannotCalculateChanges`].
pub fn query_changes(
   _state: &AppState,
   auth: &AccountInfo,
   args: serde_json::Value,
) -> MethodResult {
   validate_filter_fields(&args)?;
   let req = serde_json::from_value::<QueryChangesRequest<QuotaFilter>>(args)
      .map_err(|error| bad_args(format!("invalid Quota/queryChanges args: {error}")))?;
   require_auth_match(auth, req.account_id.as_ref())?;
   validate_sort(req.sort.as_deref())?;
   Err(MethodError::CannotCalculateChanges)
}

fn quota(account_id: &str) -> Quota {
   Quota {
      id:            Id(format!("quota-{account_id}")),
      resource_type: "octets".into(),
      used:          0,
      warn_limit:    None,
      soft_limit:    None,
      hard_limit:    None,
      name:          "Mail".into(),
      description:   Some("Aggregate mailbox storage; server limit unknown".into()),
      types:         vec!["Email".into()],
      scope:         "account".into(),
   }
}

fn validate_filter_fields(args: &serde_json::Value) -> Result<(), MethodError> {
   if args.get("filter").is_some_and(|filter| {
      has_unsupported_fields(filter, &["name", "scope", "resourceType", "type"])
   }) {
      Err(MethodError::UnsupportedFilter)
   } else {
      Ok(())
   }
}

fn validate_sort(sort: Option<&[Comparator]>) -> Result<(), MethodError> {
   if sort.is_some_and(|comparators| {
      comparators.iter().any(|comparator| {
         !matches!(comparator.property.as_str(), "name" | "used")
            || comparator.collation.is_some()
            || !comparator.extra.is_empty()
      })
   }) {
      Err(MethodError::UnsupportedSort)
   } else {
      Ok(())
   }
}

fn quota_matches(quota: &Quota, filter: &Filter<QuotaFilter>) -> bool {
   match filter {
      Filter::Operator {
         operator,
         conditions,
      } => {
         match operator {
            FilterOp::And => conditions.iter().all(|child| quota_matches(quota, child)),
            FilterOp::Or => conditions.iter().any(|child| quota_matches(quota, child)),
            FilterOp::Not => !conditions.iter().any(|child| quota_matches(quota, child)),
         }
      },
      Filter::Condition(condition) => {
         condition
            .name
            .as_ref()
            .is_none_or(|name| quota.name.contains(name))
            && condition
               .scope
               .as_ref()
               .is_none_or(|scope| &quota.scope == scope)
            && condition
               .resource_type
               .as_ref()
               .is_none_or(|resource_type| &quota.resource_type == resource_type)
            && condition
               .type_name
               .as_ref()
               .is_none_or(|type_name| quota.types.contains(type_name))
      },
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn quota_filter_operators_follow_rfc_identity_rules() {
      let quota = quota("account");
      let matching = serde_json::from_value::<Filter<QuotaFilter>>(serde_json::json!({
          "operator": "AND",
          "conditions": [
              {"name": "ai"},
              {"operator": "NOT", "conditions": [{"scope": "domain"}]}
          ]
      }))
      .unwrap();
      assert!(quota_matches(&quota, &matching));
      assert!(!quota_matches(&quota, &Filter::Operator {
         operator:   FilterOp::Or,
         conditions: vec![],
      }));
   }
}
