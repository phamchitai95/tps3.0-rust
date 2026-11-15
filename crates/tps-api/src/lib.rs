//! String-keyed RPC — same names as tps59 `Api.call`.

use async_trait::async_trait;
use deadpool_postgres::Pool;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tps_auth::{hash_password, need, perms_map, role, session_user, verify_password};
use tps_db::{query_one_json, DbError};
use tps_logic::so_build_hien_thi;
use tps_migrate::{ensure_migrated, MIGRATE_VERSION};
use tps_types::{AppError, Args, Ctx};

pub type HandlerResult = Result<Value, AppError>;

pub struct AppState {
    pub pool: Pool,
    pub ctx: Mutex<Ctx>,
}

impl AppState {
    pub fn new(pool: Pool) -> Self {
        Self {
            pool,
            ctx: Mutex::new(Ctx::default()),
        }
    }
}

#[async_trait]
pub trait Handler: Send + Sync {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult;
}

pub struct Api {
    handlers: HashMap<String, Arc<dyn Handler>>,
}

impl Default for Api {
    fn default() -> Self {
        Self::new()
    }
}

impl Api {
    pub fn new() -> Self {
        let mut api = Self {
            handlers: HashMap::new(),
        };
        api.register("app.ping", Arc::new(Ping));
        api.register("app.version", Arc::new(Version));
        api.register("app.checkUpdate", Arc::new(AppCheckUpdate));
        api.register("app.init", Arc::new(AppInit));
        api.register("auth.login", Arc::new(AuthLogin));
        api.register("auth.logout", Arc::new(AuthLogout));
        api.register("auth.me", Arc::new(AuthMe));
        api.register("auth.perms", Arc::new(AuthPerms));
        api.register("auth.changePassword", Arc::new(AuthChangePassword));
        api.register("monhoc.list", Arc::new(MonhocList));
        api.register("giaovien.list", Arc::new(GiaovienList));
        api.register("hocsinh.list", Arc::new(HocsinhList));
        api.register("hocsinh.get", Arc::new(HocsinhGet));
        api.register("lophoc.list", Arc::new(LophocList));
        api.register("lophoc.get", Arc::new(LophocGet));
        api.register("nhanvien.list", Arc::new(NhanvienList));
        api.register("chucvu.list", Arc::new(ChucvuList));
        api.register("caidat.get", Arc::new(CaidatGet));
        // tuition / finance (S2)
        api.register("kyhp.sync", Arc::new(KyhpSync));
        api.register("kyhp.list", Arc::new(KyhpList));
        api.register("hocphi.collect", Arc::new(HocphiCollect));
        api.register("hocphi.receipt", Arc::new(HocphiReceipt));
        api.register("congno.list", Arc::new(CongnoList));
        api.register("quy.get", Arc::new(QuyGet));
        // pay / VietQR / Pay2S (S3)
        api.register("ck.bankInfo", Arc::new(CkBankInfo));
        api.register("ck.getSettings", Arc::new(CkGetSettings));
        api.register("ck.saveSettings", Arc::new(CkSaveSettings));
        api.register("ck.qrPreview", Arc::new(CkQrPreview));
        api.register("ck.processPay2s", Arc::new(CkProcessPay2s));
        api.register("ck.gdList", Arc::new(CkGdList));
        // payroll (S4 + S4b)
        api.register("luong.preview", Arc::new(LuongPreview));
        api.register("luong.lichsu", Arc::new(LuongLichsu));
        api.register("bl.ds", Arc::new(BlDs));
        api.register("bl.gv", Arc::new(BlGv));
        api.register("phieuchi.list", Arc::new(PhieuchiList));
        api.register("bl.khautru", Arc::new(BlKhautru));
        api.register("bl.khautruXoa", Arc::new(BlKhautruXoa));
        api.register("bl.chot", Arc::new(BlChot));
        api.register("bl.mo", Arc::new(BlMo));
        api.register("bl.ck.ds", Arc::new(BlCkDs));
        api.register("bl.ck.xuat", Arc::new(BlCkXuat));
        api.register("bl.ck.tai", Arc::new(BlCkTai));
        api.register("bl.ck.huy", Arc::new(BlCkHuy));
        api
    }

    pub fn register(&mut self, name: impl Into<String>, h: Arc<dyn Handler>) {
        self.handlers.insert(name.into(), h);
    }

    pub async fn call(&self, state: &AppState, name: &str, args: Args) -> HandlerResult {
        let h = self
            .handlers
            .get(name)
            .ok_or_else(|| AppError::UnknownCommand(name.to_string()))?;
        h.call(state, args).await
    }

    pub fn commands(&self) -> Vec<String> {
        let mut k: Vec<_> = self.handlers.keys().cloned().collect();
        k.sort();
        k
    }
}

fn db_err(e: DbError) -> AppError {
    AppError::msg(e.to_string())
}

fn arg_str(args: &Args, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

async fn client(state: &AppState) -> Result<deadpool_postgres::Object, AppError> {
    let c = state
        .pool
        .get()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    let _ = c.execute("SET TIME ZONE 'Asia/Ho_Chi_Minh'", &[]).await;
    Ok(c)
}

struct Ping;
#[async_trait]
impl Handler for Ping {
    async fn call(&self, _state: &AppState, _args: Args) -> HandlerResult {
        Ok(json!({ "ok": true }))
    }
}

/// App build number — bump mỗi sprint đóng gói.
pub const APP_BUILD: i32 = 2;
pub const APP_VERSION: &str = "0.1.0";
/// Repo GitHub Releases dùng cho OTA (`owner/name`). Override: `TPS_OTA_REPO`.
pub const DEFAULT_OTA_REPO: &str = "phamchitai95/tps3.0-rust";

struct Version;
#[async_trait]
impl Handler for Version {
    async fn call(&self, _state: &AppState, _args: Args) -> HandlerResult {
        Ok(json!({
            "name": "tps3.0-rust",
            "version": APP_VERSION,
            "build": APP_BUILD,
            "buildHienThi": so_build_hien_thi(APP_BUILD),
            "migrateVersion": MIGRATE_VERSION,
            "ui": "tauri2",
            "ota": "github-releases",
            "otaRepo": std::env::var("TPS_OTA_REPO").unwrap_or_else(|_| DEFAULT_OTA_REPO.into()),
        }))
    }
}

/// `app.checkUpdate` — so sánh bản hiện tại với GitHub Releases latest (S6).
/// Cài đặt updater plugin + ký gói: xem `docs/RELEASE.md` (chưa bắt buộc signing để build).
struct AppCheckUpdate;
#[async_trait]
impl Handler for AppCheckUpdate {
    async fn call(&self, _state: &AppState, args: Args) -> HandlerResult {
        let repo = args
            .get("repo")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("TPS_OTA_REPO").ok())
            .unwrap_or_else(|| DEFAULT_OTA_REPO.into());
        if !repo.contains('/') || repo.contains("..") {
            return Err(AppError::msg("repo OTA không hợp lệ (cần owner/name)."));
        }
        let url = format!("https://api.github.com/repos/{repo}/releases/latest");
        let client = reqwest::Client::builder()
            .user_agent(format!("tps3.0-rust/{APP_VERSION}"))
            .timeout(std::time::Duration::from_secs(20))
            .build()
            .map_err(|e| AppError::msg(e.to_string()))?;
        let resp = client
            .get(&url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| AppError::msg(format!("Không gọi được GitHub Releases: {e}")))?;
        let status = resp.status();
        if status.as_u16() == 404 {
            return Ok(json!({
                "ok": true,
                "update": false,
                "current": { "version": APP_VERSION, "build": APP_BUILD },
                "latest": Value::Null,
                "repo": repo,
                "message": "Chưa có release trên GitHub.",
            }));
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::msg(format!(
                "GitHub Releases HTTP {status}: {}",
                body.chars().take(200).collect::<String>()
            )));
        }
        let body: Value = resp
            .json()
            .await
            .map_err(|e| AppError::msg(format!("JSON release lỗi: {e}")))?;
        let tag = body
            .get("tag_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .trim_start_matches('v')
            .to_string();
        let name = body
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let html_url = body
            .get("html_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let published = body
            .get("published_at")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let notes = body
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .chars()
            .take(2000)
            .collect::<String>();
        let assets: Vec<Value> = body
            .get("assets")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|x| {
                        let n = x.get("name")?.as_str()?;
                        let u = x.get("browser_download_url")?.as_str()?;
                        Some(json!({ "name": n, "url": u }))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let update = !tag.is_empty() && version_is_newer(&tag, APP_VERSION);
        Ok(json!({
            "ok": true,
            "update": update,
            "current": { "version": APP_VERSION, "build": APP_BUILD },
            "latest": {
                "version": tag,
                "name": name,
                "url": html_url,
                "publishedAt": published,
                "notes": notes,
                "assets": assets,
            },
            "repo": repo,
            "message": if update {
                format!("Có bản mới {tag} (đang chạy {APP_VERSION}).")
            } else if tag.is_empty() {
                "Không đọc được tag release.".into()
            } else {
                format!("Đang ở bản mới nhất ({APP_VERSION}).")
            },
        }))
    }
}

/// SemVer đơn giản: so sánh từng số; tag không parse được → coi là khác/mới nếu ≠ current.
fn version_is_newer(latest: &str, current: &str) -> bool {
    fn parts(s: &str) -> Vec<u64> {
        s.split(|c: char| !c.is_ascii_digit())
            .filter(|p| !p.is_empty())
            .filter_map(|p| p.parse().ok())
            .collect()
    }
    let a = parts(latest);
    let b = parts(current);
    if a.is_empty() {
        return latest != current;
    }
    for i in 0..a.len().max(b.len()) {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

#[cfg(test)]
mod version_cmp_tests {
    use super::version_is_newer;

    #[test]
    fn newer_patch() {
        assert!(version_is_newer("0.1.1", "0.1.0"));
        assert!(!version_is_newer("0.1.0", "0.1.0"));
        assert!(!version_is_newer("0.1.0", "0.2.0"));
        assert!(version_is_newer("1.0.0", "0.9.9"));
    }
}

struct AppInit;
#[async_trait]
impl Handler for AppInit {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let client = client(state).await?;
        ensure_migrated(client.as_ref())
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        tps_catalog::ensure_bootstrap(&client).await?;
        let tt = query_one_json(&client, "SELECT tentrungtam FROM thongtintt LIMIT 1", &[])
            .await
            .map_err(db_err)?;
        Ok(json!({
            "ok": true,
            "tentrungtam": tt.and_then(|m| m.get("tentrungtam").cloned()),
            "migrateVersion": MIGRATE_VERSION,
        }))
    }
}

struct AuthLogin;
#[async_trait]
impl Handler for AuthLogin {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let username = arg_str(&args, "username");
        let password = args
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let client = client(state).await?;
        let user_key = username.clone();
        let nv = query_one_json(
            &client,
            r#"SELECT n.nhanvienid, n.hotennv, n.hoten, n.chucvu, n.matkhau, n.quyen, n.an, d.biendichchucvu
                 FROM nhanvien n LEFT JOIN doichieuchucvu d ON d.chucvu = n.chucvu
                WHERE LOWER(n.hotennv) = LOWER($1) AND n.daxoa IS NULL"#,
            &[&user_key],
        )
        .await
        .map_err(db_err)?;

        let Some(nv) = nv else {
            return Err(AppError::msg("Sai tên đăng nhập hoặc mật khẩu."));
        };
        let stored = nv
            .get("matkhau")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if !verify_password(&password, stored.as_deref()) {
            return Err(AppError::msg("Sai tên đăng nhập hoặc mật khẩu."));
        }
        if stored
            .as_deref()
            .map(|s| !s.starts_with("scrypt$"))
            .unwrap_or(true)
        {
            let hashed = hash_password(&password);
            let id = nv.get("nhanvienid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            client
                .execute(
                    "UPDATE nhanvien SET matkhau = $1 WHERE nhanvienid = $2",
                    &[&hashed.as_str(), &id],
                )
                .await
                .map_err(|e| AppError::msg(e.to_string()))?;
        }
        let ten = nv
            .get("biendichchucvu")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let user = session_user(&nv, ten.as_deref());
        state.ctx.lock().await.user = Some(user.clone());
        Ok(Value::Object(user))
    }
}

struct AuthLogout;
#[async_trait]
impl Handler for AuthLogout {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        state.ctx.lock().await.user = None;
        Ok(json!(true))
    }
}

struct AuthMe;
#[async_trait]
impl Handler for AuthMe {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        Ok(state
            .ctx
            .lock()
            .await
            .user
            .clone()
            .map(Value::Object)
            .unwrap_or(Value::Null))
    }
}

struct AuthPerms;
#[async_trait]
impl Handler for AuthPerms {
    async fn call(&self, _state: &AppState, _args: Args) -> HandlerResult {
        Ok(json!(perms_map()))
    }
}

struct AuthChangePassword;
#[async_trait]
impl Handler for AuthChangePassword {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let (id, _) = {
            let ctx = state.ctx.lock().await;
            need(&ctx, role::CTV)?;
            let id = ctx
                .user
                .as_ref()
                .and_then(|u| u.get("nhanvienid"))
                .and_then(|v| v.as_i64())
                .ok_or_else(|| AppError::msg("Vui lòng đăng nhập."))?;
            (id, ())
        };
        let np = arg_str(&args, "newPassword");
        if np.is_empty() {
            return Err(AppError::msg("Vui lòng nhập mật khẩu mới."));
        }
        let client = client(state).await?;
        let id32 = id as i32;
        let row = query_one_json(
            &client,
            "SELECT matkhau FROM nhanvien WHERE nhanvienid = $1",
            &[&id32],
        )
        .await
        .map_err(db_err)?;
        let old = arg_str(&args, "oldPassword");
        let stored = row
            .as_ref()
            .and_then(|m| m.get("matkhau"))
            .and_then(|v| v.as_str());
        if !verify_password(&old, stored) {
            return Err(AppError::msg("Mật khẩu cũ không đúng."));
        }
        let hashed = hash_password(&np);
        client
            .execute(
                "UPDATE nhanvien SET matkhau = $1 WHERE nhanvienid = $2",
                &[&hashed.as_str(), &id32],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        Ok(json!(true))
    }
}

struct MonhocList;
#[async_trait]
impl Handler for MonhocList {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_catalog::monhoc_list(&c).await
    }
}

struct GiaovienList;
#[async_trait]
impl Handler for GiaovienList {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_catalog::giaovien_list(&c, &ctx).await
    }
}

struct HocsinhList;
#[async_trait]
impl Handler for HocsinhList {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_catalog::hocsinh_list(&c, &args).await
    }
}

struct HocsinhGet;
#[async_trait]
impl Handler for HocsinhGet {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_catalog::hocsinh_get(&c, &args).await
    }
}

struct LophocList;
#[async_trait]
impl Handler for LophocList {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_catalog::lophoc_list(&c, &args).await
    }
}

struct LophocGet;
#[async_trait]
impl Handler for LophocGet {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_catalog::lophoc_get(&c, &args).await
    }
}

struct NhanvienList;
#[async_trait]
impl Handler for NhanvienList {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_catalog::nhanvien_list(&c, &ctx).await
    }
}

struct ChucvuList;
#[async_trait]
impl Handler for ChucvuList {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_catalog::chucvu_list(&c).await
    }
}

struct CaidatGet;
#[async_trait]
impl Handler for CaidatGet {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_catalog::caidat_get(&c).await
    }
}

struct KyhpSync;
#[async_trait]
impl Handler for KyhpSync {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let hs = args
            .get("hocsinhid")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32);
        let n = tps_tuition::ensure_periods(&c, hs, None).await?;
        Ok(json!({ "created": n }))
    }
}

struct KyhpList;
#[async_trait]
impl Handler for KyhpList {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_tuition::kyhp_list(&c, &args).await
    }
}

struct HocphiCollect;
#[async_trait]
impl Handler for HocphiCollect {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let mut c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_tuition::hocphi_collect(&mut c, &ctx, &args).await
    }
}

struct HocphiReceipt;
#[async_trait]
impl Handler for HocphiReceipt {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_tuition::hocphi_receipt(&c, &ctx, &args).await
    }
}

struct CongnoList;
#[async_trait]
impl Handler for CongnoList {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_tuition::congno_list(&c, &ctx, &args).await
    }
}

struct QuyGet;
#[async_trait]
impl Handler for QuyGet {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        tps_tuition::quy_get(&c).await
    }
}

struct CkBankInfo;
#[async_trait]
impl Handler for CkBankInfo {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_pay::ck_bank_info(&c, &ctx).await
    }
}

struct CkGetSettings;
#[async_trait]
impl Handler for CkGetSettings {
    async fn call(&self, state: &AppState, _args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_pay::ck_get_settings(&c, &ctx).await
    }
}

struct CkSaveSettings;
#[async_trait]
impl Handler for CkSaveSettings {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_pay::ck_save_settings(&c, &ctx, &args).await
    }
}

struct CkQrPreview;
#[async_trait]
impl Handler for CkQrPreview {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_pay::ck_qr_preview(&c, &ctx, &args).await
    }
}

struct CkProcessPay2s;
#[async_trait]
impl Handler for CkProcessPay2s {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_pay::ck_process_pay2s(&c, &ctx, &args).await
    }
}

struct CkGdList;
#[async_trait]
impl Handler for CkGdList {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_pay::ck_gd_list(&c, &ctx, &args).await
    }
}

struct LuongPreview;
#[async_trait]
impl Handler for LuongPreview {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::luong_preview(&c, &ctx, &args).await
    }
}

struct LuongLichsu;
#[async_trait]
impl Handler for LuongLichsu {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::luong_lichsu(&c, &ctx, &args).await
    }
}

struct BlDs;
#[async_trait]
impl Handler for BlDs {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_ds(&c, &ctx, &args).await
    }
}

struct BlGv;
#[async_trait]
impl Handler for BlGv {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_gv(&c, &ctx, &args).await
    }
}

struct PhieuchiList;
#[async_trait]
impl Handler for PhieuchiList {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::phieuchi_list(&c, &ctx, &args).await
    }
}

struct BlKhautru;
#[async_trait]
impl Handler for BlKhautru {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let mut c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_khautru(&mut c, &ctx, &args).await
    }
}

struct BlKhautruXoa;
#[async_trait]
impl Handler for BlKhautruXoa {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let mut c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_khautru_xoa(&mut c, &ctx, &args).await
    }
}

struct BlChot;
#[async_trait]
impl Handler for BlChot {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let mut c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_chot(&mut c, &ctx, &args).await
    }
}

struct BlMo;
#[async_trait]
impl Handler for BlMo {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let mut c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_mo(&mut c, &ctx, &args).await
    }
}

struct BlCkDs;
#[async_trait]
impl Handler for BlCkDs {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_ck_ds(&c, &ctx, &args).await
    }
}

struct BlCkXuat;
#[async_trait]
impl Handler for BlCkXuat {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let mut c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_ck_xuat(&mut c, &ctx, &args).await
    }
}

struct BlCkTai;
#[async_trait]
impl Handler for BlCkTai {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_ck_tai(&c, &ctx, &args).await
    }
}

struct BlCkHuy;
#[async_trait]
impl Handler for BlCkHuy {
    async fn call(&self, state: &AppState, args: Args) -> HandlerResult {
        let c = client(state).await?;
        let ctx = state.ctx.lock().await;
        tps_payroll::bl_ck_huy(&c, &ctx, &args).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tps_db::{connect, dev_test_config};

    async fn setup() -> (Api, AppState) {
        let pool = connect(&dev_test_config()).await.expect("db");
        let state = AppState::new(pool);
        let api = Api::new();
        api.call(&state, "app.init", Args::new())
            .await
            .expect("init");
        (api, state)
    }

    #[tokio::test]
    async fn login_admin_and_lists() {
        let (api, state) = setup().await;
        let user = api
            .call(
                &state,
                "auth.login",
                {
                    let mut a = Args::new();
                    a.insert("username".into(), json!("admin"));
                    a.insert("password".into(), json!("admin"));
                    a
                },
            )
            .await
            .expect("login");
        assert_eq!(user["chucvu"], 1);
        assert_eq!(user["tenchucvu"], "Admin");

        let roles = api.call(&state, "chucvu.list", Args::new()).await.unwrap();
        assert!(roles.as_array().unwrap().len() >= 3);

        let mon = api.call(&state, "monhoc.list", Args::new()).await.unwrap();
        assert!(!mon.as_array().unwrap().is_empty());

        let hs = api.call(&state, "hocsinh.list", Args::new()).await.unwrap();
        assert!(!hs.as_array().unwrap().is_empty());

        let lop = api
            .call(
                &state,
                "lophoc.list",
                {
                    let mut a = Args::new();
                    a.insert("status".into(), json!("all"));
                    a
                },
            )
            .await
            .unwrap();
        assert!(!lop.as_array().unwrap().is_empty());

        assert!(api
            .call(
                &state,
                "auth.login",
                {
                    let mut a = Args::new();
                    a.insert("username".into(), json!("admin"));
                    a.insert("password".into(), json!("sai"));
                    a
                },
            )
            .await
            .is_err());
    }
}
