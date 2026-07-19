//! `jmapper` — the JMAP server daemon.

use std::{
   collections::HashMap,
   future::IntoFuture as _,
   path::PathBuf,
   sync::Arc,
   time::Duration,
};

use anyhow::{
   Context as _,
   Result,
};
use clap::{
   Parser,
   Subcommand,
};
use dav_sync::{
   engine::SyncStats,
   service::DavHandle,
   store::{
      self as dav_store,
      DavKind,
   },
};
use tokio::{
   net::TcpListener,
   signal,
   sync::oneshot,
   task::JoinSet,
   time,
};
use tracing::info;

const HTTP_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

mod bootstrap;
mod config;
mod reload;

use crate::config::Config;

#[derive(Parser, Debug)]
#[command(version, about = "Rust replacement for jmap-perl")]
struct Cli {
   /// Path to the TOML config file.
   #[arg(long, short, env = "JMAPPER_CONFIG", default_value = "jmapper.toml")]
   config: PathBuf,

   #[command(subcommand)]
   command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
   /// Run the server (default action if no subcommand given).
   Run,
   /// One-time OAuth bootstrap for an OAuth-backed Gmail account.
   Bootstrap {
      /// The account id (matches `[[accounts]].id` in config).
      #[arg(long)]
      account: String,
   },
}

fn init_tracing() {
   use tracing_subscriber::{
      EnvFilter,
      fmt,
      prelude::*,
   };
   let filter = EnvFilter::try_from_env("JMAPPER_LOG")
      .or_else(|_| EnvFilter::try_new("info,jmapper=debug,imap_sync=debug,jmap_server=debug"))
      .expect("hardcoded filter must parse");
   tracing_subscriber::registry()
      .with(filter)
      .with(fmt::layer().with_target(true))
      .init();
}

#[tokio::main]
async fn main() -> Result<()> {
   init_tracing();

   let cli = Cli::parse();
   let config = Config::load(&cli.config)
      .with_context(|| format!("loading config from {}", cli.config.display()))?;
   info!(
       accounts = config.accounts.len(),
       bind = %config.server.bind,
       "loaded config",
   );

   match cli.command.unwrap_or(Command::Run) {
      Command::Run => run_server(config, cli.config).await?,
      Command::Bootstrap { account } => {
         bootstrap::run(&config, &account).await?;
      },
   }

   Ok(())
}

async fn run_server(config: Config, config_path: PathBuf) -> Result<()> {
   use imap_sync::cache;
   use jmap_server::{
      AccountInfo,
      AppState,
      build_router,
      scheduler,
      state::DavAvailability,
   };

   let pool = cache::open(&config.server.database_url)
      .await
      .context("connecting to postgres / initializing schema")?;

   let mut http_accounts = Vec::with_capacity(config.accounts.len());
   let mut account_senders = HashMap::new();
   let mut handle_map = HashMap::new();
   for account in &config.accounts {
      let handle = reload::spawn_one(&pool, account).await?;
      http_accounts.push(AccountInfo::from_bearer_token(
         &account.id,
         &account.email,
         &account.display_name,
         &account.bearer_token,
      ));
      account_senders.insert(account.id.clone(), handle.tx.clone());
      handle_map.insert(account.id.clone(), handle);
   }

   let mut dav_task_map = HashMap::<String, reload::ManagedDav>::new();
   let mut dav_handles = HashMap::new();
   let mut dav_availability = HashMap::<String, DavAvailability>::new();
   let mut initial_dav_sync = Vec::<(String, DavHandle, DavKind, bool)>::new();
   for account in &config.accounts {
      let (managed, initial) = reload::spawn_dav_one(&pool, account)
         .await
         .with_context(|| format!("configuring DAV for {}", account.id))?;
      let Some(managed) = managed else {
         continue;
      };
      let handle = managed.service.handle.clone();
      for (kind, force) in initial {
         initial_dav_sync.push((account.id.clone(), handle.clone(), kind, force));
      }
      dav_handles.insert(account.id.clone(), handle);
      dav_availability.insert(account.id.clone(), managed.availability);
      dav_task_map.insert(account.id.clone(), managed);
   }

   let state = AppState::new_with_dav(
      pool.clone(),
      http_accounts,
      config.server.session_url.clone(),
      account_senders,
      dav_handles,
      dav_availability,
   );
   let router = build_router(state.clone(), config.server.cors_origins.clone());

   let ctx = Arc::new(reload::ReloadContext::new(config_path, pool.clone(), state));
   ctx.seed(&config.accounts, handle_map, dav_task_map).await;

   for (account_id, handle, kind, force) in initial_dav_sync {
      let state = ctx.state.clone();
      tokio::spawn(async move {
         match handle.sync_now(kind, force).await {
            Ok(stats) => publish_dav_changes(&state, &account_id, kind, &stats).await,
            Err(err) => {
               tracing::warn!(
                   account_id = %account_id,
                   kind = kind.as_str(),
                   error = %err,
                   "initial DAV sync failed"
               );
            },
         }
      });
   }

   #[cfg(unix)]
   tokio::spawn(sighup_loop(Arc::clone(&ctx)));
   tokio::spawn(supervisor_loop(Arc::clone(&ctx)));
   if config.server.dav_sync_interval_seconds > 0 {
      tokio::spawn(dav_sync_loop(
         Arc::clone(&ctx),
         Duration::from_secs(config.server.dav_sync_interval_seconds),
      ));
   }
   tokio::spawn(scheduler::run(ctx.state.clone()));

   let listener = TcpListener::bind(config.server.bind)
      .await
      .with_context(|| format!("binding {}", config.server.bind))?;
   info!(bind = %config.server.bind, "jmapper listening");

   let (shutdown_started_tx, shutdown_started_rx) = oneshot::channel();
   let server = axum::serve(listener, router)
      .with_graceful_shutdown(async move {
         shutdown_signal().await;
         let _ = shutdown_started_tx.send(());
      })
      .into_future();
   tokio::pin!(server);
   tokio::select! {
      result = &mut server => result.context("axum serve")?,
      _ = shutdown_started_rx => {
         match time::timeout(HTTP_SHUTDOWN_TIMEOUT, &mut server).await {
            Ok(result) => result.context("axum serve")?,
            Err(_) => {
               tracing::warn!(
                  timeout_secs = HTTP_SHUTDOWN_TIMEOUT.as_secs(),
                  "HTTP drain timed out; closing long-lived connections",
               );
            },
         }
      },
   }

   shutdown_accounts(&ctx).await;

   Ok(())
}

/// Respawn unexpectedly terminated account tasks.
async fn supervisor_loop(ctx: Arc<reload::ReloadContext>) {
   let mut ticker = time::interval(Duration::from_secs(30));
   ticker.tick().await;
   loop {
      ticker.tick().await;
      reload::supervise_tick(&ctx).await;
   }
}

async fn dav_sync_loop(ctx: Arc<reload::ReloadContext>, period: Duration) {
   let mut ticker = time::interval(period);
   ticker.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
   // Initial endpoint-specific rounds are queued during startup/reload.
   ticker.tick().await;
   loop {
      ticker.tick().await;
      let jobs = {
         let tasks = ctx.dav_tasks.lock().await;
         tasks
            .iter()
            .map(|(account_id, managed)| {
               (
                  account_id.clone(),
                  managed.service.handle.clone(),
                  managed.availability,
               )
            })
            .collect::<Vec<_>>()
      };
      let mut rounds = JoinSet::new();
      for (account_id, handle, availability) in jobs {
         for kind in [
            availability.calendars.then_some(DavKind::CalDav),
            availability.contacts.then_some(DavKind::CardDav),
         ]
         .into_iter()
         .flatten()
         {
            let state = ctx.state.clone();
            let handle = handle.clone();
            let account_id = account_id.clone();
            rounds.spawn(async move {
               match handle.sync_now(kind, false).await {
                  Ok(stats) => publish_dav_changes(&state, &account_id, kind, &stats).await,
                  Err(err) => {
                     tracing::warn!(
                         account_id = %account_id,
                         kind = kind.as_str(),
                         error = %err,
                         "periodic DAV sync failed"
                     );
                  },
               }
            });
         }
      }
      while rounds.join_next().await.is_some() {}
   }
}

async fn publish_dav_changes(
   state: &jmap_server::AppState,
   account_id: &str,
   kind: DavKind,
   stats: &SyncStats,
) {
   use jmap_server::state::{
      StateChange,
      StateKind,
   };
   let collections_changed =
      stats.collections_created + stats.collections_updated + stats.collections_removed > 0;
   let resources_changed = stats.resources_created
      + stats.resources_updated
      + stats.resources_removed
      + stats.collections_removed
      > 0;
   if !collections_changed && !resources_changed {
      return;
   }
   let Ok(client) = state.pool().get().await else {
      return;
   };
   let Ok(dav_state) = dav_store::get_state(&client, account_id).await else {
      return;
   };
   let (collection_kind, collection_state, resource_kind, resource_state) = match kind {
      DavKind::CalDav => {
         (
            StateKind::Calendar,
            dav_state.calendar_modseq,
            StateKind::CalendarEvent,
            dav_state.calendar_event_modseq,
         )
      },
      DavKind::CardDav => {
         (
            StateKind::AddressBook,
            dav_state.addressbook_modseq,
            StateKind::ContactCard,
            dav_state.contact_card_modseq,
         )
      },
   };
   if collections_changed {
      state.publish_state_change(StateChange {
         account_id: account_id.to_owned(),
         kind:       collection_kind,
         new_state:  collection_state.to_string(),
      });
   }
   if resources_changed {
      state.publish_state_change(StateChange {
         account_id: account_id.to_owned(),
         kind:       resource_kind,
         new_state:  resource_state.to_string(),
      });
   }
}

#[cfg(unix)]
async fn sighup_loop(ctx: Arc<reload::ReloadContext>) {
   use tokio::signal::unix::{
      self,
      SignalKind,
   };
   let mut stream = match unix::signal(SignalKind::hangup()) {
      Ok(stream) => stream,
      Err(err) => {
         tracing::error!(error = %err, "cannot install SIGHUP handler");
         return;
      },
   };
   while stream.recv().await.is_some() {
      if let Err(err) = reload::reload(&ctx).await {
         tracing::error!(error = %err, "reload failed; keeping previous state");
      }
   }
}

/// Drain every account task, aborting ones that ignore shutdown.
async fn shutdown_accounts(ctx: &Arc<reload::ReloadContext>) {
   let drained = {
      let mut handles = ctx.handles.lock().await;
      handles.drain().collect::<Vec<_>>()
   };
   let mut joined = JoinSet::new();
   for (id, handle) in drained {
      joined.spawn(async move {
         reload::stop_one(&id, handle).await;
      });
   }
   while joined.join_next().await.is_some() {}

   let dav = {
      let mut tasks = ctx.dav_tasks.lock().await;
      tasks.drain().collect::<Vec<_>>()
   };
   let mut joined = JoinSet::new();
   for (id, managed) in dav {
      joined.spawn(async move {
         reload::stop_dav_one(&id, managed).await;
      });
   }
   while joined.join_next().await.is_some() {}
}

async fn shutdown_signal() {
   let ctrl_c = async {
      let _ = signal::ctrl_c().await;
   };
   #[cfg(unix)]
   let terminate = async {
      use tokio::signal::unix::{
         self,
         SignalKind,
      };
      if let Ok(mut stream) = unix::signal(SignalKind::terminate()) {
         stream.recv().await;
      }
   };
   #[cfg(not(unix))]
   let terminate = std::future::pending::<()>();

   tokio::select! {
       () = ctrl_c => {},
       () = terminate => {},
   }
   info!("shutdown signal received");
}
