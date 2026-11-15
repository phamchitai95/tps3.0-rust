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
        api.register("congno.list", Arc::new(CongnoList));
        api.register("quy.get", Arc::new(QuyGet));
        // pay / VietQR / Pay2S (S3)
        api.register("ck.bankInfo", Arc::new(CkBankInfo));
        api.register("ck.getSettings", Arc::new(CkGetSettings));
        api.register("ck.saveSettings", Arc::new(CkSaveSettings));
        api.register("ck.qrPreview", Arc::new(CkQrPreview));
        api.register("ck.processPay2s", Arc::new(CkProcessPay2s));
        api.register("ck.gdList", Arc::new(CkGdList));
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

struct Version;
#[async_trait]
impl Handler for Version {
    async fn call(&self, _state: &AppState, _args: Args) -> HandlerResult {
        const BUILD: i32 = 1;
        Ok(json!({
            "name": "tps3.0-rust",
            "version": "0.1.0",
            "build": BUILD,
            "buildHienThi": so_build_hien_thi(BUILD),
            "migrateVersion": MIGRATE_VERSION,
            "ui": "tauri2",
            "ota": "github-releases",
        }))
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
