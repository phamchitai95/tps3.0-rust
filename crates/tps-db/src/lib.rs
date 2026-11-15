//! Database connection + config for TPS 3.0.
//! Config path mirrors tps59: `%APPDATA%/TC59/config.json` or `TPS_DATA_DIR` / `QLTH_CONFIG`.

use deadpool_postgres::{Config as PoolConfig, Pool, Runtime};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio_postgres::NoTls;

pub const DEFAULT_TIMEZONE: &str = "Asia/Ho_Chi_Minh";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_user")]
    pub user: String,
    #[serde(default)]
    pub password: String,
    #[serde(default = "default_database")]
    pub database: String,
    #[serde(default = "default_tz", rename = "timeZone")]
    pub time_zone: String,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    5432
}
fn default_user() -> String {
    "tps".into()
}
fn default_database() -> String {
    "qlth".into()
}
fn default_tz() -> String {
    DEFAULT_TIMEZONE.into()
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            port: default_port(),
            user: default_user(),
            password: String::new(),
            database: default_database(),
            time_zone: default_tz(),
        }
    }
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Pool(#[from] deadpool_postgres::CreatePoolError),
    #[error(transparent)]
    PoolRun(#[from] deadpool_postgres::PoolError),
    #[error(transparent)]
    Pg(#[from] tokio_postgres::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn data_dir() -> PathBuf {
    if let Ok(ep) = env::var("TPS_DATA_DIR") {
        if !ep.is_empty() {
            return PathBuf::from(ep);
        }
    }
    #[cfg(windows)]
    {
        let appdata = env::var("APPDATA").unwrap_or_else(|_| ".".into());
        return PathBuf::from(appdata).join("TC59");
    }
    #[cfg(not(windows))]
    {
        let home = env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(".local/share/TC59")
    }
}

pub fn config_path() -> PathBuf {
    env::var("QLTH_CONFIG")
        .map(PathBuf::from)
        .unwrap_or_else(|_| data_dir().join("config.json"))
}

pub fn load_config() -> Result<DbConfig, DbError> {
    let path = config_path();
    let mut cfg = if path.exists() {
        let text = std::fs::read_to_string(&path)?;
        serde_json::from_str(&text)?
    } else {
        DbConfig::default()
    };
    if let Ok(db) = env::var("QLTH_DATABASE") {
        if !db.is_empty() {
            cfg.database = db;
        }
    }
    Ok(cfg)
}

pub fn save_config(cfg: &DbConfig) -> Result<(), DbError> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(cfg)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn write_dev_config(dir: &Path, cfg: &DbConfig) -> Result<PathBuf, DbError> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join("config.json");
    std::fs::write(&path, serde_json::to_string_pretty(cfg)?)?;
    Ok(path)
}

/// Build a deadpool connection pool. Sets session TimeZone on each checkout.
pub async fn connect(cfg: &DbConfig) -> Result<Pool, DbError> {
    let mut pc = PoolConfig::new();
    pc.host = Some(cfg.host.clone());
    pc.port = Some(cfg.port);
    pc.user = Some(cfg.user.clone());
    pc.password = Some(cfg.password.clone());
    pc.dbname = Some(cfg.database.clone());
    let pool = pc.create_pool(Some(Runtime::Tokio1), NoTls)?;
    // Verify + set timezone
    let client = pool.get().await?;
    client
        .execute(
            &format!("SET TIME ZONE '{}'", cfg.time_zone.replace('\'', "''")),
            &[],
        )
        .await?;
    Ok(pool)
}

/// Monotonic ID via `app_meta` key `seq:<table>` — same contract as Dart `nextId`.
pub async fn next_id(
    client: &tokio_postgres::Client,
    table: &str,
    col: &str,
) -> Result<i64, DbError> {
    let key = format!("seq:{table}");
    let row = client
        .query_opt(
            "SELECT value FROM app_meta WHERE key = $1 FOR UPDATE",
            &[&key],
        )
        .await?;
    let next = if let Some(r) = row {
        let cur: String = r.get(0);
        cur.parse::<i64>().unwrap_or(0) + 1
    } else {
        let max: i64 = client
            .query_one(
                &format!("SELECT COALESCE(MAX({col}), 0) FROM {table}"),
                &[],
            )
            .await?
            .get(0);
        max + 1
    };
    client
        .execute(
            "INSERT INTO app_meta (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
            &[&key, &next.to_string()],
        )
        .await?;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn connect_qlth_test() {
        let cfg = DbConfig {
            host: "127.0.0.1".into(),
            port: 5432,
            user: "tps".into(),
            password: "tps_dev".into(),
            database: "qlth_test".into(),
            time_zone: DEFAULT_TIMEZONE.into(),
        };
        let pool = connect(&cfg).await.expect("connect");
        let c = pool.get().await.expect("checkout");
        let row = c.query_one("SELECT 1::int AS n", &[]).await.unwrap();
        let n: i32 = row.get(0);
        assert_eq!(n, 1);
        let tz: String = c
            .query_one("SHOW TIME ZONE", &[])
            .await
            .unwrap()
            .get(0);
        assert!(
            tz.contains("Ho_Chi_Minh") || tz == "Asia/Ho_Chi_Minh" || tz.contains("+07"),
            "tz={tz}"
        );
    }
}
