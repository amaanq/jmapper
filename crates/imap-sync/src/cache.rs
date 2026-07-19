//! PostgreSQL pool setup and schema migrations.

use deadpool_postgres::{
   Config,
   ManagerConfig,
   Pool,
   RecyclingMethod,
   Runtime,
};
use jmapper_codegen::queries;
use tokio_postgres::NoTls;
use tracing::info;

use crate::error::{
   Result,
   SyncError,
};

const MIGRATIONS: &[&str] = &[
   include_str!("../../../schema.sql"),
   include_str!("../migrations/0002_message_imap_uid_unique.sql"),
];

/// Build the shared pool and bring the database schema up to date.
///
/// `url` is a libpq-style connection string (`host=/run/postgresql
/// dbname=jmapper`). `NoTls` only: the supported deployments talk to postgres
/// over a local unix socket or localhost.
///
/// # Errors
///
/// Returns an error when the pool cannot be created or a schema migration
/// fails.
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

   let (mut client, connection) = tokio_postgres::connect(url, NoTls).await?;
   let conn_task = tokio::spawn(connection);
   migrate(&mut client).await?;
   drop(client);
   conn_task.abort();

   Ok(pool)
}

async fn migrate(client: &mut tokio_postgres::Client) -> Result<()> {
   // Prevent concurrent migrations.
   client
      .execute("SELECT pg_advisory_lock(7573303270)", &[])
      .await?;
   client
      .batch_execute(
         "CREATE TABLE IF NOT EXISTS schema_version (
             version    BIGINT NOT NULL PRIMARY KEY,
             applied_at BIGINT NOT NULL
          )",
      )
      .await?;
   let mut current = queries::migrations::current_schema_version()
      .bind(client)
      .one()
      .await?;

   // Pre-runner databases already have the baseline.
   if current == 0
      && queries::migrations::baseline_already_applied()
         .bind(client)
         .one()
         .await?
   {
      queries::migrations::record_schema_version()
         .bind(client, &1_i64)
         .await?;
      current = 1;
   }

   for (index, sql) in MIGRATIONS.iter().enumerate() {
      let version = index as i64 + 1;
      if version <= current {
         continue;
      }
      let tx = client.transaction().await?;
      tx.batch_execute(sql).await?;
      queries::migrations::record_schema_version()
         .bind(&tx, &version)
         .await?;
      tx.commit().await?;
      info!(version, "applied schema migration");
   }
   Ok(())
}

#[cfg(test)]
mod tests {
   use tokio_postgres::{
      Client,
      NoTls,
   };

   use super::open;
   use crate::testkit;

   async fn connect(url: &str) -> Client {
      let (client, conn) = tokio_postgres::connect(url, NoTls).await.unwrap();
      tokio::spawn(conn);
      client
   }

   #[tokio::test]
   async fn migration_dedupes_uid_slots() {
      let Some(url) = testkit::raw_test_db().await else {
         return;
      };
      let client = connect(&url).await;
      client
         .batch_execute(include_str!("../../../schema.sql"))
         .await
         .unwrap();
      client
         .batch_execute(
            "INSERT INTO accounts (id, email, provider, display_name, bearer_token_hash, \
             created_at) VALUES ('gmail', 'a@b.c', 'gmail', 'A', ''::bytea, 0);
             INSERT INTO mailboxes (id, account_id, name, modseq) VALUES
                ('mb_inbox', 'gmail', 'INBOX', 0),
                ('mb_trash', 'gmail', 'Trash', 0);
             INSERT INTO folders (account_id, imap_name, mailbox_id) VALUES
                ('gmail', 'INBOX', 'mb_inbox'),
                ('gmail', 'Trash', 'mb_trash');
             INSERT INTO messages (account_id, msgid, thrid, received_at, size, modseq) VALUES
                ('gmail', 'real', 'real', 0, 1, 1),
                ('gmail', 'fb_phantom', 'fb_phantom', 0, 0, 2),
                ('gmail', 'fb_shared', 'fb_shared', 0, 0, 3);
             INSERT INTO message_imap (account_id, msgid, folder_id, uid, uidvalidity)
             SELECT 'gmail', 'real', id, 255025, 1 FROM folders WHERE imap_name = 'INBOX';
             INSERT INTO message_imap (account_id, msgid, folder_id, uid, uidvalidity)
             SELECT 'gmail', 'fb_phantom', id, 255025, 1 FROM folders WHERE imap_name = 'INBOX';
             INSERT INTO message_imap (account_id, msgid, folder_id, uid, uidvalidity)
             SELECT 'gmail', 'fb_shared', id, 255025, 1 FROM folders WHERE imap_name = 'INBOX';
             INSERT INTO message_imap (account_id, msgid, folder_id, uid, uidvalidity)
             SELECT 'gmail', 'fb_shared', id, 7, 2 FROM folders WHERE imap_name = 'Trash';",
         )
         .await
         .unwrap();
      drop(client);

      open(&url).await.unwrap();

      let client = connect(&url).await;
      let row = client
         .query_one(
            "SELECT
                (SELECT msgid FROM message_imap WHERE uid = 255025),
                EXISTS (SELECT 1 FROM messages WHERE msgid = 'fb_phantom'),
                EXISTS (SELECT 1 FROM messages WHERE msgid = 'fb_shared')",
            &[],
         )
         .await
         .unwrap();
      assert_eq!(row.get::<_, String>(0), "real");
      assert!(!row.get::<_, bool>(1));
      assert!(row.get::<_, bool>(2));
   }
}
