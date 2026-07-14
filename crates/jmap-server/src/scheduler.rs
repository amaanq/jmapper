//! Delayed-send scheduler: relays `pending` `EmailSubmission` rows whose
//! `sendAt` has passed.
//!
//! One process-wide loop, polling every 15 s — plenty for minute-granularity
//! send scheduling. Rows are claimed with a compare-and-set to `'sending'`
//! so a concurrent cancel loses cleanly (the UPDATE's WHERE clause is the
//! arbiter), then relayed through the owning account task with the raw
//! bytes staged at create time.
//!
//! Failure policy: SMTP errors retry on later ticks up to [`MAX_ATTEMPTS`];
//! after that the row flips to `canceled` — the spec's undoStatus has no
//! "failed" value, and `canceled` at least tells the client the message did
//! NOT go out. Each transition bumps the submission modseq and publishes an
//! SSE state change so watching clients refresh.

use std::time::Duration;

use imap_sync::{
   account::AccountRequest,
   db,
};
use jmapper_codegen::queries;
use tokio::{
   sync::oneshot,
   time,
};
use tracing::{
   error,
   info,
   warn,
};

use crate::{
   methods::email_submission,
   state::{
      AppState,
      StateChange,
      StateKind,
   },
};

const TICK_SECS: u64 = 15;
const MAX_ATTEMPTS: i64 = 5;

pub async fn run(state: AppState) {
   // A crash between claim and result strands rows in 'sending'. Reset on
   // boot: worst case (crash after SMTP accepted but before the row was
   // marked final) risks a duplicate send, which beats silently never
   // sending.
   let recovered = async {
      let client = state.pool().get().await.map_err(|err| err.to_string())?;
      queries::submissions::recover_stranded_submissions()
         .bind(&client)
         .await
         .map_err(|err| err.to_string())
   }
   .await;
   match recovered {
      Ok(rows) if rows > 0 => {
         warn!(rows, "recovered stranded in-flight submissions");
      },
      Ok(_) => {},
      Err(err) => error!(error = %err, "scheduler boot recovery failed"),
   }

   let mut ticker = time::interval(Duration::from_secs(TICK_SECS));
   loop {
      ticker.tick().await;
      if let Err(err) = tick(&state).await {
         warn!(error = %err, "submission scheduler tick failed");
      }
   }
}

use jmapper_codegen::queries::submissions::DueSubmissionRow as DueRow;

async fn tick(state: &AppState) -> Result<(), String> {
   let now = chrono::Utc::now().timestamp();
   let due = {
      let client = state.pool().get().await.map_err(|err| err.to_string())?;
      queries::submissions::due_submissions()
         .bind(&client, &now)
         .all()
         .await
         .map_err(|err| err.to_string())?
   };

   for row in due {
      let claimed = {
         let client = state.pool().get().await.map_err(|err| err.to_string())?;
         queries::submissions::claim_submission()
            .bind(&client, &row.account_id.as_str(), &row.id.as_str())
            .await
            .map_err(|err| err.to_string())?
      };
      if claimed == 0 {
         continue; // canceled (or claimed elsewhere) since the SELECT
      }
      match relay(state, &row).await {
         Ok(delivery_status) => {
            finish(state, &row, "final", Some(&delivery_status)).await?;
            info!(account_id = %row.account_id, id = %row.id, "delayed submission sent");
         },
         Err(err) if row.attempts + 1 >= MAX_ATTEMPTS => {
            error!(
                account_id = %row.account_id,
                id = %row.id,
                attempts = row.attempts + 1,
                error = %err,
                "delayed submission failed permanently; marking canceled",
            );
            finish(state, &row, "canceled", None).await?;
         },
         Err(err) => {
            warn!(
                account_id = %row.account_id,
                id = %row.id,
                attempts = row.attempts + 1,
                error = %err,
                "delayed submission failed; will retry",
            );
            let client = state.pool().get().await.map_err(|err| err.to_string())?;
            queries::submissions::retry_submission()
               .bind(&client, &row.account_id.as_str(), &row.id.as_str())
               .await
               .map_err(|err| err.to_string())?;
         },
      }
   }
   Ok(())
}

/// Ok carries the per-rcpt deliveryStatus JSON built from the smarthost's
/// accept reply.
async fn relay(state: &AppState, row: &DueRow) -> Result<String, String> {
   #[derive(serde::Deserialize)]
   struct Envelope {
      #[serde(rename = "mailFrom")]
      mail_from: Addr,
      #[serde(rename = "rcptTo")]
      rcpt_to:   Vec<Addr>,
   }
   #[derive(serde::Deserialize)]
   struct Addr {
      email: String,
   }
   let env = serde_json::from_str::<Envelope>(&row.envelope_json)
      .map_err(|err| format!("stored envelope unparseable: {err}"))?;
   let raw = row
      .raw_rfc822
      .clone()
      .ok_or_else(|| "no staged bytes on pending submission".to_owned())?;

   let tx = state
      .account_sender(&row.account_id)
      .ok_or_else(|| "no sync task for account".to_owned())?;
   let (respond, rx) = oneshot::channel();
   let rcpts = env
      .rcpt_to
      .into_iter()
      .map(|addr| addr.email)
      .collect::<Vec<String>>();
   tx.send(AccountRequest::SubmitStaged {
      mail_from: env.mail_from.email,
      rcpt_to: rcpts.clone(),
      raw,
      respond,
   })
   .await
   .map_err(|_| "account task channel closed".to_owned())?;
   let reply = time::timeout(Duration::from_secs(120), rx)
      .await
      .map_err(|_| "submission timed out".to_owned())?
      .map_err(|_| "account task dropped submission".to_owned())?
      .map_err(|err| err.to_string())?;
   Ok(email_submission::delivery_status_json(&rcpts, &reply))
}

/// Terminal transition: drop the staged bytes (no longer needed either way),
/// stamp the new status + modseq, tell SSE subscribers.
async fn finish(
   state: &AppState,
   row: &DueRow,
   status: &str,
   delivery_status: Option<&str>,
) -> Result<(), String> {
   let modseq = db::bump_modseq(state.pool(), &row.account_id, db::StateKind::Submission)
      .await
      .map_err(|err| err.to_string())?;
   let client = state.pool().get().await.map_err(|err| err.to_string())?;
   queries::submissions::finish_submission()
      .bind(
         &client,
         &status,
         &delivery_status,
         &(modseq as i64),
         &row.account_id.as_str(),
         &row.id.as_str(),
      )
      .await
      .map_err(|err| err.to_string())?;
   state.publish_state_change(StateChange {
      account_id: row.account_id.clone(),
      kind:       StateKind::EmailSubmission,
      new_state:  modseq.to_string(),
   });
   Ok(())
}
