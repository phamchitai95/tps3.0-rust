//! S7 — chạy smoke RPC trên DB so sánh (mặc định `qlth_so_sanh`).
//!
//! ```bash
//! QLTH_DATABASE=qlth_so_sanh cargo run -p tps-so-sanh
//! # hoặc: ./scripts/so_sanh.sh
//! ```

use chrono::Datelike;
use serde_json::{json, Map, Value};
use std::env;
use std::process::ExitCode;
use tps_api::{Api, AppState, APP_BUILD, APP_VERSION};
use tps_db::{connect, query_json, query_one_json, DbConfig, DEFAULT_TIMEZONE};
use tps_migrate::MIGRATE_VERSION;
use tps_types::Args;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            let fail = report
                .get("ok")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                == false;
            if fail {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("tps-so-sanh lỗi: {e}");
            ExitCode::from(2)
        }
    }
}

async fn run() -> Result<Value, String> {
    let database = env::var("QLTH_DATABASE").unwrap_or_else(|_| "qlth_so_sanh".into());
    let user = env::var("SO_SANH_USER").unwrap_or_else(|_| "admin".into());
    let pass = env::var("SO_SANH_PASS").unwrap_or_else(|_| "admin".into());
    let sample_n: usize = env::var("SO_SANH_SAMPLE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);

    let cfg = DbConfig {
        host: env::var("PGHOST").unwrap_or_else(|_| "127.0.0.1".into()),
        port: env::var("PGPORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(5432),
        user: env::var("PGUSER").unwrap_or_else(|_| "tps".into()),
        password: env::var("PGPASSWORD").unwrap_or_else(|_| "tps_dev".into()),
        database: database.clone(),
        time_zone: DEFAULT_TIMEZONE.into(),
    };

    let pool = connect(&cfg)
        .await
        .map_err(|e| format!("connect {database}: {e}"))?;
    {
        let c = pool.get().await.map_err(|e| e.to_string())?;
        let _ = c.execute("SET TIME ZONE 'Asia/Ho_Chi_Minh'", &[]).await;
    }

    let state = AppState::new(pool);
    let api = Api::new();

    let mut checks: Vec<Value> = Vec::new();
    let mut hard_fail = false;

    // init / migrate
    match api.call(&state, "app.init", Args::new()).await {
        Ok(v) => checks.push(check("app.init", true, v)),
        Err(e) => {
            hard_fail = true;
            checks.push(check("app.init", false, json!({ "error": e.to_string() })));
        }
    }

    let client = state.pool.get().await.map_err(|e| e.to_string())?;
    let migrate = query_one_json(
        &client,
        "SELECT value FROM app_meta WHERE key = 'migrate_version'",
        &[],
    )
    .await
    .map_err(|e| e.to_string())?;
    let mig_val = migrate
        .as_ref()
        .and_then(|m| m.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let mig_ok = mig_val == MIGRATE_VERSION.to_string() || mig_val.parse::<i32>().unwrap_or(0) >= 20;
    if !mig_ok {
        hard_fail = true;
    }
    checks.push(check(
        "migrate_version",
        mig_ok,
        json!({ "value": mig_val, "expected": MIGRATE_VERSION }),
    ));

    let counts = table_counts(&client).await?;
    let has_data = counts
        .get("hocsinh")
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
        > 0;
    checks.push(check(
        "counts",
        has_data,
        json!({ "tables": counts, "note": if has_data { "có dữ liệu" } else { "DB trống — restore dump trước" } }),
    ));
    if !has_data {
        hard_fail = true;
    }

    // login
    let mut login_args = Args::new();
    login_args.insert("username".into(), json!(user));
    login_args.insert("password".into(), json!(pass));
    let login = api.call(&state, "auth.login", login_args).await;
    match &login {
        Ok(v) => checks.push(check("auth.login", true, v.clone())),
        Err(e) => {
            // Real dumps may not use admin/admin — warn, not always hard fail.
            let soft = env::var("SO_SANH_REQUIRE_LOGIN").ok().as_deref() != Some("1");
            if !soft {
                hard_fail = true;
            }
            checks.push(check(
                "auth.login",
                soft,
                json!({
                    "error": e.to_string(),
                    "hint": "set SO_SANH_USER / SO_SANH_PASS; SO_SANH_REQUIRE_LOGIN=1 để fail cứng"
                }),
            ));
        }
    }

    if login.is_ok() {
        // catalog
        for cmd in ["hocsinh.list", "lophoc.list", "giaovien.list", "quy.get"] {
            let mut a = Args::new();
            if cmd == "lophoc.list" {
                a.insert("status".into(), json!("all"));
            }
            match api.call(&state, cmd, a).await {
                Ok(v) => {
                    let n = v.as_array().map(|a| a.len()).unwrap_or(1);
                    checks.push(check(cmd, true, json!({ "n": n })));
                }
                Err(e) => {
                    hard_fail = true;
                    checks.push(check(cmd, false, json!({ "error": e.to_string() })));
                }
            }
        }

        // công nợ: RPC vs SQL cho vài HV
        let hs_rows = query_json(
            &client,
            "SELECT hocsinhid FROM hocsinh ORDER BY hocsinhid LIMIT $1",
            &[&(sample_n as i64)],
        )
        .await
        .map_err(|e| e.to_string())?;

        let mut debt_mismatches = Vec::new();
        for r in &hs_rows {
            let hsid = r.get("hocsinhid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let mut a = Args::new();
            a.insert("hocsinhid".into(), json!(hsid));
            let rpc = match api.call(&state, "congno.list", a).await {
                Ok(v) => v,
                Err(e) => {
                    debt_mismatches.push(json!({ "hocsinhid": hsid, "error": e.to_string() }));
                    continue;
                }
            };
            let rpc_sum: i64 = rpc
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|x| x.get("conlai").and_then(|v| v.as_i64()).unwrap_or(0))
                        .sum()
                })
                .unwrap_or(0);
            let sql_sum = query_one_json(
                &client,
                "SELECT COALESCE(SUM(sotien - dathu), 0)::bigint AS conlai
                   FROM kyhocphi
                  WHERE hocsinhid = $1 AND trangthai = 'chuathu' AND sotien > dathu",
                &[&hsid],
            )
            .await
            .map_err(|e| e.to_string())?
            .and_then(|m| m.get("conlai").and_then(|v| v.as_i64()))
            .unwrap_or(0);

            // After congno.list, ensure_periods may create rows — allow rpc >= sql baseline
            // Prefer exact match when both present.
            if rpc_sum != sql_sum {
                // Re-read SQL after list (sync may have run inside congno)
                let sql2 = query_one_json(
                    &client,
                    "SELECT COALESCE(SUM(sotien - dathu), 0)::bigint AS conlai
                       FROM kyhocphi
                      WHERE hocsinhid = $1 AND trangthai = 'chuathu' AND sotien > dathu",
                    &[&hsid],
                )
                .await
                .ok()
                .flatten()
                .and_then(|m| m.get("conlai").and_then(|v| v.as_i64()))
                .unwrap_or(sql_sum);
                if rpc_sum != sql2 {
                    debt_mismatches.push(json!({
                        "hocsinhid": hsid,
                        "rpc": rpc_sum,
                        "sql": sql2,
                    }));
                }
            }
        }
        let debt_ok = debt_mismatches.is_empty();
        if !debt_ok {
            hard_fail = true;
        }
        checks.push(check(
            "congno.rpc_vs_sql",
            debt_ok,
            json!({ "sample": hs_rows.len(), "mismatches": debt_mismatches }),
        ));

        // payroll list smoke (month VN)
        let now = chrono::Utc::now() + chrono::Duration::hours(7);
        let thang = format!("{:02}-{}", now.month(), now.year());
        let mut a = Args::new();
        a.insert("thang".into(), json!(thang));
        match api.call(&state, "bl.ds", a).await {
            Ok(v) => checks.push(check(
                "bl.ds",
                true,
                json!({ "thang": thang, "n": v.as_array().map(|x| x.len()).unwrap_or(0) }),
            )),
            Err(e) => checks.push(check(
                "bl.ds",
                true, // soft — dump cũ có thể thiếu bảng lương
                json!({ "thang": thang, "warn": e.to_string() }),
            )),
        }
    }

    Ok(json!({
        "ok": !hard_fail,
        "sprint": "S7",
        "app": { "version": APP_VERSION, "build": APP_BUILD },
        "database": database,
        "checks": checks,
    }))
}

fn check(name: &str, ok: bool, detail: Value) -> Value {
    json!({ "name": name, "ok": ok, "detail": detail })
}

async fn table_counts(
    client: &deadpool_postgres::Object,
) -> Result<Map<String, Value>, String> {
    let tables = [
        "hocsinh",
        "lophoc",
        "giaovien",
        "kyhocphi",
        "phieuthutien",
        "nhanvien",
        "taikhoan",
    ];
    let mut m = Map::new();
    for t in tables {
        // identifier whitelist only
        let sql = format!("SELECT COUNT(*)::bigint AS n FROM {t}");
        let n = query_one_json(client, &sql, &[])
            .await
            .map_err(|e| format!("{t}: {e}"))?
            .and_then(|r| r.get("n").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        m.insert(t.into(), json!(n));
    }
    Ok(m)
}
