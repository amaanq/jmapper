//! Isolated PostgreSQL databases for parallel tests.

use std::{
   env,
   process,
   sync::atomic::{
      AtomicU64,
      Ordering,
   },
};

use deadpool_postgres::Pool;
use tokio_postgres::NoTls;

use crate::{
   cache,
   error::Result,
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// # Panics
///
/// Panics if `JMAPPER_REQUIRE_DB_TESTS` is set while `JMAPPER_TEST_DB_URL` is
/// missing, or if creating the isolated test database fails.
pub async fn test_pool() -> Option<Pool> {
   let Ok(admin_url) = env::var("JMAPPER_TEST_DB_URL") else {
      assert!(
         env::var("JMAPPER_REQUIRE_DB_TESTS").is_err(),
         "JMAPPER_REQUIRE_DB_TESTS is set but JMAPPER_TEST_DB_URL is missing"
      );
      eprintln!("skipping DB-backed test: JMAPPER_TEST_DB_URL not set");
      return None;
   };
   Some(
      fresh_db(&admin_url)
         .await
         .expect("test database setup failed"),
   )
}

async fn fresh_db(admin_url: &str) -> Result<Pool> {
   let name = format!(
      "jmapper_test_{}_{}",
      process::id(),
      COUNTER.fetch_add(1, Ordering::Relaxed)
   );
   let (client, conn) = tokio_postgres::connect(admin_url, NoTls).await?;
   let conn_task = tokio::spawn(conn);
   client
      .execute(&format!("CREATE DATABASE {name}"), &[])
      .await?;
   drop(client);
   conn_task.abort();
   cache::open(&rewrite_dbname(admin_url, &name)).await
}

fn rewrite_dbname(url: &str, name: &str) -> String {
   let mut parts = url
      .split_whitespace()
      .filter(|kv| !kv.starts_with("dbname="))
      .map(str::to_owned)
      .collect::<Vec<String>>();
   parts.push(format!("dbname={name}"));
   parts.join(" ")
}
