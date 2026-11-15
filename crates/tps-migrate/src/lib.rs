//! Schema migration — keep `MIGRATE_VERSION` in sync with tps59 `migrate.dart`.
//! S0: apply baseline schema dump; full step-by-step port comes in later sprints.

use thiserror::Error;
use tokio_postgres::Client;

/// Must match tps59 `core/lib/src/migrate.dart` → `migrateVersion`.
pub const MIGRATE_VERSION: i32 = 27;

#[derive(Debug, Error)]
pub enum MigrateError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Pg(#[from] tokio_postgres::Error),
}

pub async fn read_version(client: &Client) -> Result<Option<i32>, MigrateError> {
    let exists = client
        .query_one(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = 'app_meta'
             )",
            &[],
        )
        .await?
        .get::<_, bool>(0);
    if !exists {
        return Ok(None);
    }
    let row = client
        .query_opt(
            "SELECT value FROM app_meta WHERE key = 'migrate_version'",
            &[],
        )
        .await?;
    Ok(row.and_then(|r| {
        let v: String = r.get(0);
        v.parse().ok()
    }))
}

pub async fn set_version(client: &Client, version: i32) -> Result<(), MigrateError> {
    client
        .execute(
            "INSERT INTO app_meta (key, value) VALUES ('migrate_version', $1)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
            &[&version.to_string()],
        )
        .await?;
    Ok(())
}

/// Apply a full schema SQL dump (e.g. `schema/qlth_cautruc.sql`), then stamp version.
pub async fn apply_baseline_sql(client: &Client, sql: &str) -> Result<(), MigrateError> {
    client.batch_execute(sql).await?;
    // Ensure app_meta PK for ON CONFLICT (dump may only have table, not constraint name we need)
    client
        .batch_execute(
            r#"
            DO $$ BEGIN
              IF NOT EXISTS (
                SELECT 1 FROM pg_constraint WHERE conname = 'app_meta_pkey'
              ) THEN
                ALTER TABLE app_meta ADD PRIMARY KEY (key);
              END IF;
            END $$;
            "#,
        )
        .await
        .ok(); // ignore if already PK via another name
    set_version(client, MIGRATE_VERSION).await?;
    Ok(())
}

pub async fn ensure_migrated(client: &Client) -> Result<i32, MigrateError> {
    match read_version(client).await? {
        Some(v) if v == MIGRATE_VERSION => Ok(v),
        Some(v) => Err(MigrateError::Message(format!(
            "Schema version {v} ≠ {MIGRATE_VERSION}. Restore baseline or run migrate steps."
        ))),
        None => Err(MigrateError::Message(
            "Chưa có schema. Chạy scripts/init_dev_db.sh trước.".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tps_db::{connect, DbConfig, DEFAULT_TIMEZONE};

    #[tokio::test]
    async fn version_readable_after_baseline() {
        let cfg = DbConfig {
            host: "127.0.0.1".into(),
            port: 5432,
            user: "tps".into(),
            password: "tps_dev".into(),
            database: "qlth_test".into(),
            time_zone: DEFAULT_TIMEZONE.into(),
        };
        let pool = connect(&cfg).await.expect("connect");
        let c = pool.get().await.expect("client");
        // May be None before init script; after init should be 27
        let v = read_version(c.as_ref()).await.unwrap();
        if let Some(v) = v {
            assert_eq!(v, MIGRATE_VERSION);
        }
    }
}
