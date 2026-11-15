//! Database connection + config for TPS 3.0.
//! Config path mirrors tps59: `%APPDATA%/TC59/config.json` or `TPS_DATA_DIR` / `QLTH_CONFIG`.

use deadpool_postgres::{Config as PoolConfig, Object, Pool, Runtime};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::env;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio_postgres::types::Type;
use tokio_postgres::{NoTls, Row as PgRow};

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

/// Convert a Postgres row to a JSON object (Dart/Node-shaped).
pub fn row_to_json(row: &PgRow) -> Map<String, Value> {
    let mut m = Map::new();
    for (i, col) in row.columns().iter().enumerate() {
        let name = col.name().to_string();
        let v = pg_to_json(row, i, col.type_());
        m.insert(name, v);
    }
    m
}

fn pg_to_json(row: &PgRow, i: usize, ty: &Type) -> Value {
    match *ty {
        Type::BOOL => json_opt(row.try_get::<_, Option<bool>>(i).ok().flatten()),
        Type::INT2 => json_opt(row.try_get::<_, Option<i16>>(i).ok().flatten()),
        Type::INT4 => json_opt(row.try_get::<_, Option<i32>>(i).ok().flatten()),
        Type::INT8 => json_opt(row.try_get::<_, Option<i64>>(i).ok().flatten()),
        Type::FLOAT4 => json_opt(row.try_get::<_, Option<f32>>(i).ok().flatten()),
        Type::FLOAT8 => json_opt(row.try_get::<_, Option<f64>>(i).ok().flatten()),
        Type::TEXT | Type::VARCHAR | Type::BPCHAR | Type::NAME | Type::UNKNOWN => {
            json_opt(row.try_get::<_, Option<String>>(i).ok().flatten())
        }
        Type::DATE => match row.try_get::<_, Option<chrono::NaiveDate>>(i) {
            Ok(Some(d)) => json!(d.format("%Y-%m-%d").to_string()),
            _ => Value::Null,
        },
        Type::TIMESTAMP => match row.try_get::<_, Option<chrono::NaiveDateTime>>(i) {
            Ok(Some(t)) => json!(t.format("%Y-%m-%d %H:%M:%S").to_string()),
            _ => Value::Null,
        },
        Type::TIMESTAMPTZ => match row.try_get::<_, Option<chrono::DateTime<chrono::Utc>>>(i) {
            Ok(Some(t)) => json!(t.naive_local().format("%Y-%m-%d %H:%M:%S").to_string()),
            _ => Value::Null,
        },
        _ => {
            // NUMERIC and others: try string then i64/f64
            if let Ok(Some(s)) = row.try_get::<_, Option<String>>(i) {
                if let Ok(n) = s.parse::<i64>() {
                    return json!(n);
                }
                if let Ok(n) = s.parse::<f64>() {
                    return json!(n);
                }
                return json!(s);
            }
            if let Ok(v) = row.try_get::<_, Option<i64>>(i) {
                return json_opt(v);
            }
            if let Ok(v) = row.try_get::<_, Option<f64>>(i) {
                return json_opt(v);
            }
            Value::Null
        }
    }
}

fn json_opt<T: serde::Serialize>(v: Option<T>) -> Value {
    match v {
        Some(x) => json!(x),
        None => Value::Null,
    }
}

pub async fn query_json(
    client: &Object,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> Result<Vec<Map<String, Value>>, DbError> {
    let rows = client.query(sql, params).await?;
    Ok(rows.iter().map(row_to_json).collect())
}

pub async fn query_one_json(
    client: &Object,
    sql: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> Result<Option<Map<String, Value>>, DbError> {
    let row = client.query_opt(sql, params).await?;
    Ok(row.map(|r| row_to_json(&r)))
}

/// Dev helper: config pointing at local qlth_test.
pub fn dev_test_config() -> DbConfig {
    DbConfig {
        host: env::var("PGHOST").unwrap_or_else(|_| "127.0.0.1".into()),
        port: env::var("PGPORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5432),
        user: env::var("PGUSER").unwrap_or_else(|_| "tps".into()),
        password: env::var("PGPASSWORD").unwrap_or_else(|_| "tps_dev".into()),
        database: env::var("QLTH_DATABASE").unwrap_or_else(|_| "qlth_test".into()),
        time_zone: DEFAULT_TIMEZONE.into(),
    }
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
