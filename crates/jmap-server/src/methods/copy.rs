use std::collections::HashMap;

use jmap_protocol::{
   error::MethodError,
   ids::{
      AccountId,
      Id,
   },
};
use serde::Deserialize;

use super::{
   MethodResult,
   bad_args,
   enforce_set_limit,
};
use crate::state::AccountInfo;

#[derive(Deserialize)]
struct EmailCopyArgs {
   #[serde(rename = "fromAccountId")]
   from_account_id: AccountId,
   #[serde(rename = "accountId")]
   account_id:      AccountId,
   create:          HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct BlobCopyArgs {
   #[serde(rename = "fromAccountId")]
   from_account_id: AccountId,
   #[serde(rename = "accountId")]
   account_id:      AccountId,
   #[serde(rename = "blobIds")]
   blob_ids:        Vec<Id>,
}

/// Rejects `Email/copy` and `Blob/copy` after validating the request, since
/// cross-account copy is not supported.
///
/// # Errors
///
/// Returns a [`MethodError`] if `method` is neither `Email/copy` nor
/// `Blob/copy`, the arguments are malformed, the request exceeds the set limit,
/// `fromAccountId` equals `accountId`, or either account does not match the
/// authenticated account. A valid cross-account request still returns
/// [`MethodError::FromAccountNotSupportedByMethod`].
pub fn unavailable(auth: &AccountInfo, args: serde_json::Value, method: &str) -> MethodResult {
   let (from_account_id, account_id, count) = match method {
      "Email/copy" => {
         let req = serde_json::from_value::<EmailCopyArgs>(args)
            .map_err(|error| bad_args(format!("invalid {method} args: {error}")))?;
         (req.from_account_id, req.account_id, req.create.len())
      },
      "Blob/copy" => {
         let req = serde_json::from_value::<BlobCopyArgs>(args)
            .map_err(|error| bad_args(format!("invalid {method} args: {error}")))?;
         (req.from_account_id, req.account_id, req.blob_ids.len())
      },
      _ => return Err(MethodError::UnknownMethod),
   };
   enforce_set_limit(count, 0, 0)?;
   if from_account_id == account_id {
      return Err(bad_args("fromAccountId and accountId must differ"));
   }
   if account_id.as_ref() != auth.id {
      return Err(MethodError::AccountNotFound);
   }
   if from_account_id.as_ref() != auth.id {
      return Err(MethodError::FromAccountNotFound);
   }
   Err(MethodError::FromAccountNotSupportedByMethod)
}
