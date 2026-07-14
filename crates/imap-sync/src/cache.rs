//! PostgreSQL pool setup and initial schema bootstrap. Query helpers live in
//! sibling modules; typed statements come from `jmapper-codegen`.

use deadpool_postgres::{
   Config,
   ManagerConfig,
   Pool,
   RecyclingMethod,
   Runtime,
};
use tokio_postgres::NoTls;

use crate::error::{
   Result,
   SyncError,
};

/// Build the shared pool and initialize an empty database.
///
/// `url` is a libpq-style connection string (`host=/run/postgresql
/// dbname=jmapper`). `NoTls` only: the supported deployments talk to postgres
/// over a local unix socket or localhost.
///
/// # Errors
///
/// Returns an error when the pool cannot be created or the schema cannot be
/// initialized.
#[inline]
pub async fn open(url: &str) -> Result<Pool> {
   let mut cfg = Config::new();
   cfg.url = Some(url.to_owned());
   cfg.manager = Some(ManagerConfig {
      recycling_method: RecyclingMethod::Fast,
   });
   let pool = cfg
      .create_pool(Some(Runtime::Tokio1), NoTls)
      .map_err(|err| SyncError::Other(format!("building postgres pool: {err}")))?;

   let (client, connection) = tokio_postgres::connect(url, NoTls).await?;
   let conn_task = tokio::spawn(connection);
   let initialized = client
      .query_one("SELECT to_regclass('public.accounts') IS NOT NULL", &[])
      .await?
      .get::<_, bool>(0);
   if !initialized {
      client
         .batch_execute(include_str!("../../../schema.sql"))
         .await?;
   }
   drop(client);
   conn_task.abort();

   Ok(pool)
}
