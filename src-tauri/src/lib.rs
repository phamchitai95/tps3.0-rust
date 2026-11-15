use serde_json::{Map, Value};
use std::collections::HashMap;
use tps_api::Api;
use tps_types::{Args, Ctx};

/// Frontend → Rust RPC bridge. Command names match tps59 `Api.call`.
#[tauri::command]
async fn api_call(name: String, args: Option<Value>) -> Result<Value, String> {
    let api = Api::new();
    let ctx = Ctx::default();
    let args: Args = match args {
        Some(Value::Object(map)) => map.into_iter().collect(),
        _ => HashMap::new(),
    };
    api.call(&ctx, &name, args)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("TPS 3.0 — xin chào, {name}!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, api_call])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// silence unused in lib-only checks
#[allow(dead_code)]
fn _keep(map: Map<String, Value>) {
    let _ = map;
}
