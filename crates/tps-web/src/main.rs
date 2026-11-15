//! TPS web HTTP API (S5 subset) — parity with tps59 `web_server` API surface.
//!
//! - `POST /api/<cmd>` JSON body → `{ "kq": ... }` or `{ "loi": "..." }`
//! - `Authorization: Bearer <token>` after `auth.login` (returns `{token,user,...}`)
//! - `POST /pay2s/push` webhook when `ck_push` + `pay2s_token` configured
//! - Optional read-only mode via `TPS_WEB_CHI_XEM=1`
//!
//! Run: `tps-web` (env `TPS_WEB_PORT` default 8787; DB via existing QLTH_*/config.json).

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use rand::RngCore;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tps_api::{Api, AppState};
use tps_db::{connect, load_config, query_one_json};
use tps_migrate::{ensure_migrated, MIGRATE_VERSION};
use tps_types::{AppError, Args, Ctx};

const BUILD: i32 = 1;
const VERSION: &str = "0.1.0";

#[derive(Clone)]
struct Phien {
    ctx: Ctx,
    ip: String,
    nho: bool,
    bat_dau: chrono::DateTime<Utc>,
    dung_cuoi: chrono::DateTime<Utc>,
}

impl Phien {
    fn het_han(&self) -> bool {
        let lim = if self.nho {
            Duration::days(30)
        } else {
            Duration::hours(12)
        };
        Utc::now() - self.dung_cuoi > lim
    }
}

struct WebInner {
    api: Api,
    state: AppState,
    phien: Mutex<HashMap<String, Phien>>,
    ten_trung_tam: Mutex<String>,
    chi_xem: bool,
    port: u16,
    bat_dau: chrono::DateTime<Utc>,
}

type Shared = Arc<WebInner>;

fn bam_token(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    hex::encode(h.finalize())
}

fn ma_phien() -> String {
    let mut b = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut b);
    hex::encode(b)
}

fn loi_msg(e: &AppError) -> String {
    e.to_string()
}

/// Read-only command heuristic (ported from Dart `laLenhDoc`).
fn la_lenh_doc(ten: &str) -> bool {
    const KHONG_DOC: &[&str] = &["vh.baocao.gui", "nhatky.daxem"];
    if KHONG_DOC.contains(&ten) {
        return false;
    }
    const LENH_DOC: &[&str] = &[
        "auth.login",
        "auth.logout",
        "web.thongtin",
        "bl.gv",
        "ck.bankInfo",
        "ck.getSettings",
        "ck.coChe",
    ];
    if LENH_DOC.contains(&ten) {
        return true;
    }
    const DUOI: &[&str] = &[
        "list", "get", "ds", "lichsu", "find", "xem", "preview", "me", "perms", "dem", "receipt",
        "bankInfo", "transactions", "pending", "thang", "ngay", "tonghop",
    ];
    ten.rsplit_once('.')
        .map(|(_, s)| DUOI.contains(&s))
        .unwrap_or(false)
}

const LENH_CAM: &[&str] = &["app.init", "config.get", "config.save"];

async fn with_ctx<F, Fut>(inner: &WebInner, ctx: Ctx, f: F) -> (Ctx, Result<Value, AppError>)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Value, AppError>>,
{
    {
        let mut g = inner.state.ctx.lock().await;
        *g = ctx;
    }
    let result = f().await;
    let out_ctx = inner.state.ctx.lock().await.clone();
    (out_ctx, result)
}

async fn boot() -> Result<Shared, Box<dyn std::error::Error + Send + Sync>> {
    let cfg = load_config()?;
    let pool = connect(&cfg).await?;
    {
        let c = pool.get().await?;
        let _ = c.execute("SET TIME ZONE 'Asia/Ho_Chi_Minh'", &[]).await;
        ensure_migrated(c.as_ref()).await.map_err(|e| e.to_string())?;
    }
    let state = AppState::new(pool.clone());
    let api = Api::new();
    // migrate / bootstrap via app.init
    let _ = api.call(&state, "app.init", Args::new()).await;

    let ten = {
        let c = pool.get().await?;
        query_one_json(&c, "SELECT tentrungtam FROM thongtintt LIMIT 1", &[])
            .await
            .ok()
            .flatten()
            .and_then(|m| {
                m.get("tentrungtam")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "Trung tâm".into())
    };

    let chi_xem = std::env::var("TPS_WEB_CHI_XEM")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let port: u16 = std::env::var("TPS_WEB_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8787);

    Ok(Arc::new(WebInner {
        api,
        state,
        phien: Mutex::new(HashMap::new()),
        ten_trung_tam: Mutex::new(ten),
        chi_xem,
        port,
        bat_dau: Utc::now(),
    }))
}

async fn root(State(inner): State<Shared>) -> impl IntoResponse {
    let ten = inner.ten_trung_tam.lock().await.clone();
    Json(json!({
        "ok": true,
        "name": "tps-web",
        "version": VERSION,
        "build": BUILD,
        "migrateVersion": MIGRATE_VERSION,
        "tentrungtam": ten,
        "chixem": inner.chi_xem,
        "api": "POST /api/<cmd>",
    }))
}

fn bearer(headers: &HeaderMap) -> String {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .trim()
        .strip_prefix("Bearer ")
        .unwrap_or("")
        .trim()
        .to_string()
}

async fn api_post(
    State(inner): State<Shared>,
    Path(ten): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("?")
        .split(',')
        .next()
        .unwrap_or("?")
        .trim()
        .to_string();

    let mut args: Args = if body.is_empty() {
        Map::new()
    } else {
        match serde_json::from_slice::<Value>(&body) {
            Ok(Value::Object(m)) => m,
            Ok(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "loi": "Dữ liệu gửi lên không hợp lệ." })),
                )
                    .into_response();
            }
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "loi": "Dữ liệu gửi lên không hợp lệ." })),
                )
                    .into_response();
            }
        }
    };

    // Accept both tendangnhap (UI) and username (Dart web) for login.
    if ten == "auth.login" {
        if !args.contains_key("username") {
            if let Some(u) = args.get("tendangnhap").cloned() {
                args.insert("username".into(), u);
            }
        }
        if !args.contains_key("password") {
            if let Some(p) = args.get("matkhau").cloned() {
                args.insert("password".into(), p);
            }
        }
    }

    if ten == "web.thongtin" {
        let ten_tt = inner.ten_trung_tam.lock().await.clone();
        return (
            StatusCode::OK,
            Json(json!({
                "kq": {
                    "tentrungtam": ten_tt,
                    "chixem": inner.chi_xem,
                    "phienban": VERSION,
                    "build": BUILD,
                    "schema": MIGRATE_VERSION,
                }
            })),
        )
            .into_response();
    }

    if ten == "auth.login" {
        let nho = args.remove("nho").and_then(|v| v.as_bool()).unwrap_or(false);
        let (ctx, res) = with_ctx(&inner, Ctx::default(), || {
            let api = &inner.api;
            let state = &inner.state;
            let a = args.clone();
            async move { api.call(state, "auth.login", a).await }
        })
        .await;
        return match res {
            Ok(u) => {
                let ma = ma_phien();
                let now = Utc::now();
                inner.phien.lock().await.insert(
                    bam_token(&ma),
                    Phien {
                        ctx,
                        ip: ip.clone(),
                        nho,
                        bat_dau: now,
                        dung_cuoi: now,
                    },
                );
                let ten_tt = inner.ten_trung_tam.lock().await.clone();
                (
                    StatusCode::OK,
                    Json(json!({
                        "kq": {
                            "token": ma,
                            "user": u,
                            "tentrungtam": ten_tt,
                        }
                    })),
                )
                    .into_response()
            }
            Err(e) => (StatusCode::OK, Json(json!({ "loi": loi_msg(&e) }))).into_response(),
        };
    }

    let raw = bearer(&headers);
    let key = bam_token(&raw);
    let mut phien_guard = inner.phien.lock().await;
    phien_guard.retain(|_, p| !p.het_han());
    let Some(ph) = phien_guard.get_mut(&key) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "loi": "Phiên đăng nhập đã hết hạn. Vui lòng đăng nhập lại." })),
        )
            .into_response();
    };
    ph.dung_cuoi = Utc::now();
    let ctx = ph.ctx.clone();
    drop(phien_guard);

    if ten == "auth.logout" {
        inner.phien.lock().await.remove(&key);
        return (StatusCode::OK, Json(json!({ "kq": true }))).into_response();
    }

    if ten.starts_with("web.") {
        return match web_admin(&inner, &ten, &args, &ctx).await {
            Ok(kq) => (StatusCode::OK, Json(json!({ "kq": kq }))).into_response(),
            Err(e) => (StatusCode::OK, Json(json!({ "loi": loi_msg(&e) }))).into_response(),
        };
    }

    if LENH_CAM.contains(&ten.as_str()) {
        return (
            StatusCode::OK,
            Json(json!({ "loi": "Chức năng này chỉ có trên app TPS Windows." })),
        )
            .into_response();
    }
    if inner.chi_xem && !la_lenh_doc(&ten) {
        return (
            StatusCode::OK,
            Json(json!({
                "loi": "Bản web đang ở chế độ chỉ xem – thao tác này hãy làm trên app TPS Windows."
            })),
        )
            .into_response();
    }

    let (new_ctx, res) = with_ctx(&inner, ctx, || {
        let api = &inner.api;
        let state = &inner.state;
        let name = ten.clone();
        let a = args.clone();
        async move { api.call(state, &name, a).await }
    })
    .await;

    // Persist ctx updates (e.g. rare handlers mutating user).
    if let Some(ph) = inner.phien.lock().await.get_mut(&key) {
        ph.ctx = new_ctx;
    }

    match res {
        Ok(kq) => (StatusCode::OK, Json(json!({ "kq": kq }))).into_response(),
        Err(e) => (StatusCode::OK, Json(json!({ "loi": loi_msg(&e) }))).into_response(),
    }
}

async fn web_admin(
    inner: &WebInner,
    ten: &str,
    _args: &Args,
    ctx: &Ctx,
) -> Result<Value, AppError> {
    let chucvu = ctx
        .user
        .as_ref()
        .and_then(|u| u.get("chucvu"))
        .and_then(|v| v.as_i64())
        .unwrap_or(99);
    if chucvu > 1 {
        return Err(AppError::msg("Chỉ Admin dùng được phần Máy chủ."));
    }
    match ten {
        "web.maychu" => {
            let mut map = inner.phien.lock().await;
            map.retain(|_, p| !p.het_han());
            let sessions: Vec<Value> = map
                .values()
                .map(|p| {
                    json!({
                        "user": p.ctx.user,
                        "ip": p.ip,
                        "nho": p.nho,
                        "batdau": p.bat_dau.to_rfc3339(),
                        "dungcuoi": p.dung_cuoi.to_rfc3339(),
                    })
                })
                .collect();
            let ten_tt = inner.ten_trung_tam.lock().await.clone();
            Ok(json!({
                "batdau": inner.bat_dau.to_rfc3339(),
                "phienban": VERSION,
                "build": BUILD,
                "schema": MIGRATE_VERSION,
                "tentrungtam": ten_tt,
                "chixem": inner.chi_xem,
                "cong": inner.port,
                "phien": sessions,
                "subset": true,
                "note": "S5: chưa port sao lưu R2 / việc nền đầy đủ.",
            }))
        }
        "web.thongtin" => {
            let ten_tt = inner.ten_trung_tam.lock().await.clone();
            Ok(json!({
                "tentrungtam": ten_tt,
                "chixem": inner.chi_xem,
                "phienban": VERSION,
                "build": BUILD,
            }))
        }
        _ => Err(AppError::msg(format!("Không có lệnh máy chủ \"{ten}\"."))),
    }
}

async fn pay2s_push(
    State(inner): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    if body.len() > 1024 * 1024 {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({ "success": false })),
        );
    }
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let c = match inner.state.pool.get().await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "success": false, "message": e.to_string() })),
            );
        }
    };
    let _ = c.execute("SET TIME ZONE 'Asia/Ho_Chi_Minh'", &[]).await;

    // ck_push must be on
    let push_on = query_one_json(&c, "SELECT value FROM app_meta WHERE key = 'ck_push'", &[])
        .await
        .ok()
        .flatten()
        .and_then(|m| m.get("value").and_then(|v| v.as_str()).map(|s| s == "1"))
        .unwrap_or(false);
    if !push_on {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "success": false, "message": "TPS dang tat nhan push" })),
        );
    }

    let expect = query_one_json(&c, "SELECT value FROM app_meta WHERE key = 'pay2s_token'", &[])
        .await
        .ok()
        .flatten()
        .and_then(|m| {
            m.get("value")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();
    let got = auth
        .as_deref()
        .unwrap_or("")
        .trim()
        .strip_prefix("Bearer ")
        .unwrap_or("")
        .trim();
    if expect.is_empty() || got != expect {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "success": false, "message": "Sai token" })),
        );
    }

    let parsed: Value = if body.is_empty() {
        json!({})
    } else {
        match serde_json::from_slice(&body) {
            Ok(v) => v,
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "success": false, "message": "JSON khong hop le" })),
                );
            }
        }
    };

    match tps_pay::process_pay2s(&c, &parsed, "push").await {
        Ok(kq) => {
            let _ = c
                .execute(
                    "INSERT INTO app_meta (key, value) VALUES ('pay2s_push_luc', extract(epoch FROM now())::bigint::text)
                     ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
                    &[],
                )
                .await;
            let n = kq.as_array().map(|a| a.len()).unwrap_or(0);
            (
                StatusCode::OK,
                Json(json!({ "success": true, "nhan": n, "kq": kq })),
            )
        }
        Err(e) => (
            StatusCode::OK,
            Json(json!({ "success": false, "message": loi_msg(&e) })),
        ),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tps_web=info,tower_http=info".into()),
        )
        .init();

    let inner = match boot().await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("tps-web không khởi động được: {e}");
            std::process::exit(1);
        }
    };
    let port = inner.port;
    let app = Router::new()
        .route("/", get(root))
        .route("/api/{*ten}", post(api_post))
        .route("/pay2s/push", post(pay2s_push))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(inner);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("TPS web {VERSION} (build {BUILD}, schema {MIGRATE_VERSION}) listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_cmd_heuristic() {
        assert!(la_lenh_doc("hocsinh.list"));
        assert!(la_lenh_doc("bl.ds"));
        assert!(la_lenh_doc("luong.preview"));
        assert!(!la_lenh_doc("hocphi.collect"));
        assert!(!la_lenh_doc("bl.chot"));
        assert!(la_lenh_doc("auth.login"));
    }

    #[test]
    fn token_hash_stable() {
        assert_eq!(bam_token("abc"), bam_token("abc"));
        assert_ne!(bam_token("abc"), bam_token("abd"));
    }
}
