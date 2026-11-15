use serde_json::Value;
use std::sync::Arc;
use tauri::State;
use tps_api::{Api, AppState};
use tps_db::{connect, load_config, DbConfig, DEFAULT_TIMEZONE};
use tps_types::Args;

struct Shared {
    api: Api,
    state: AppState,
}

/// Frontend → Rust RPC. Command names match tps59 `Api.call`.
#[tauri::command]
async fn api_call(
    shared: State<'_, Arc<Shared>>,
    name: String,
    args: Option<Value>,
) -> Result<Value, String> {
    let args: Args = match args {
        Some(Value::Object(map)) => map,
        _ => Args::new(),
    };
    shared
        .api
        .call(&shared.state, &name, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("TPS 3.0 — xin chào, {name}!")
}

fn boot_config() -> DbConfig {
    load_config().unwrap_or_else(|_| DbConfig {
        host: std::env::var("PGHOST").unwrap_or_else(|_| "127.0.0.1".into()),
        port: 5432,
        user: std::env::var("PGUSER").unwrap_or_else(|_| "tps".into()),
        password: std::env::var("PGPASSWORD").unwrap_or_else(|_| "tps_dev".into()),
        database: std::env::var("QLTH_DATABASE").unwrap_or_else(|_| "qlth_test".into()),
        time_zone: DEFAULT_TIMEZONE.into(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = boot_config();
    let runtime = tokio::runtime::Runtime::new().expect("tokio");
    let pool = runtime
        .block_on(connect(&cfg))
        .unwrap_or_else(|e| panic!("Không kết nối được Postgres: {e}"));
    let shared = Arc::new(Shared {
        api: Api::new(),
        state: AppState::new(pool),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(shared)
        .invoke_handler(tauri::generate_handler![greet, api_call])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
