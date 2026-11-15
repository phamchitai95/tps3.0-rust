//! String-keyed RPC surface — same names as tps59 `Api.call('hocphi.collect', …)`.
//! S0 ships a tiny ping/health surface; domains fill in across sprints.

use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tps_logic::so_build_hien_thi;
use tps_types::{AppError, Args, Ctx};

pub type HandlerResult = Result<Value, AppError>;

#[async_trait]
pub trait Handler: Send + Sync {
    async fn call(&self, ctx: &Ctx, args: Args) -> HandlerResult;
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
        api
    }

    pub fn register(&mut self, name: impl Into<String>, h: Arc<dyn Handler>) {
        self.handlers.insert(name.into(), h);
    }

    pub async fn call(&self, ctx: &Ctx, name: &str, args: Args) -> HandlerResult {
        let h = self
            .handlers
            .get(name)
            .ok_or_else(|| AppError::UnknownCommand(name.to_string()))?;
        h.call(ctx, args).await
    }

    pub fn commands(&self) -> Vec<String> {
        let mut k: Vec<_> = self.handlers.keys().cloned().collect();
        k.sort();
        k
    }
}

struct Ping;
#[async_trait]
impl Handler for Ping {
    async fn call(&self, _ctx: &Ctx, _args: Args) -> HandlerResult {
        Ok(json!({ "ok": true }))
    }
}

struct Version;
#[async_trait]
impl Handler for Version {
    async fn call(&self, _ctx: &Ctx, _args: Args) -> HandlerResult {
        // Baseline build number for display parity experiments (tps59 was 276).
        const BUILD: i32 = 1;
        Ok(json!({
            "name": "tps3.0-rust",
            "version": "0.1.0",
            "build": BUILD,
            "buildHienThi": so_build_hien_thi(BUILD),
            "ui": "tauri2",
            "ota": "github-releases",
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ping_and_version() {
        let api = Api::new();
        let ctx = Ctx::default();
        let p = api.call(&ctx, "app.ping", Args::new()).await.unwrap();
        assert_eq!(p["ok"], true);
        let v = api.call(&ctx, "app.version", Args::new()).await.unwrap();
        assert_eq!(v["name"], "tps3.0-rust");
        assert!(api.call(&ctx, "nope", Args::new()).await.is_err());
    }
}
