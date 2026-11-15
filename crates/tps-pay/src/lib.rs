//! Chuyển khoản + Pay2S — S3 subset (settings, QR, ingest webhook/pull body).

use chrono::NaiveDateTime;
use deadpool_postgres::Object;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use tps_auth::{need, role};
use tps_db::{query_json, query_one_json, DbError};
use tps_logic::{ascii_content, viet_qr_payload};
use tps_types::{AppError, Args, Ctx};

const META_KEYS: &[&str] = &[
    "ck_bin",
    "ck_stk",
    "ck_chutk",
    "pay2s_token",
    "pay2s_api_token",
    "pay2s_access",
    "pay2s_secret",
    "pay2s_cach",
    "pay2s_push_luc",
    "ck_pull",
    "ck_push",
    "ck_tugan",
    "ck_luat_tongno",
    "ck_luat_motky",
    "ck_luat_du",
    "ck_luat_ma",
    "ck_chan_ten",
    "ck_chan_tienmat",
    "ck_moc",
];

/// Common VietQR BINs (subset; full list can grow later).
const BANKS: &[(&str, &str, &str)] = &[
    ("970415", "VietinBank", "Ngân hàng TMCP Công thương Việt Nam"),
    ("970436", "Vietcombank", "Ngân hàng TMCP Ngoại Thương Việt Nam"),
    ("970418", "BIDV", "Ngân hàng TMCP Đầu tư và Phát triển Việt Nam"),
    ("970405", "Agribank", "Ngân hàng Nông nghiệp và Phát triển Nông thôn Việt Nam"),
    ("970422", "MBBank", "Ngân hàng TMCP Quân đội"),
    ("970407", "Techcombank", "Ngân hàng TMCP Kỹ thương Việt Nam"),
    ("970416", "ACB", "Ngân hàng TMCP Á Châu"),
    ("970432", "VPBank", "Ngân hàng TMCP Việt Nam Thịnh Vượng"),
    ("970423", "TPBank", "Ngân hàng TMCP Tiên Phong"),
    ("970403", "Sacombank", "Ngân hàng TMCP Sài Gòn Thương Tín"),
];

fn map_db(e: DbError) -> AppError {
    AppError::msg(e.to_string())
}

fn arg_str(args: &Args, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

pub const CK_TIEN_TO: &str = "TT59";

pub fn ck_noi_dung(ma: &str) -> String {
    format!("{CK_TIEN_TO} {ma}")
}

async fn meta_map(client: &Object) -> Result<HashMap<String, String>, AppError> {
    let keys: Vec<String> = META_KEYS.iter().map(|s| (*s).to_string()).collect();
    let rows = query_json(
        client,
        "SELECT key, value FROM app_meta WHERE key = ANY($1::text[])",
        &[&keys],
    )
    .await
    .map_err(map_db)?;
    let mut m = HashMap::new();
    for r in rows {
        let k = r.get("key").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let v = r
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !k.is_empty() {
            m.insert(k, v);
        }
    }
    Ok(m)
}

fn bank_name(bin: &str) -> Option<(&'static str, &'static str)> {
    BANKS
        .iter()
        .find(|(b, _, _)| *b == bin)
        .map(|(_, n, f)| (*n, *f))
}

pub async fn bank_info(client: &Object) -> Result<Option<Value>, AppError> {
    let m = meta_map(client).await?;
    let bin = m.get("ck_bin").cloned().unwrap_or_default();
    let stk = m.get("ck_stk").cloned().unwrap_or_default();
    if bin.is_empty() || stk.is_empty() {
        return Ok(None);
    }
    let (name, full) = bank_name(&bin).unwrap_or((bin.as_str(), ""));
    Ok(Some(json!({
        "bin": bin,
        "nganhang": name,
        "tenday": full,
        "sotaikhoan": stk,
        "chutaikhoan": m.get("ck_chutk").cloned().unwrap_or_default(),
    })))
}

pub async fn ck_bank_info(client: &Object, ctx: &Ctx) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    Ok(bank_info(client).await?.unwrap_or(Value::Null))
}

pub async fn ck_get_settings(client: &Object, ctx: &Ctx) -> Result<Value, AppError> {
    need(ctx, role::ADMIN)?;
    let m = meta_map(client).await?;
    Ok(json!({
        "bin": m.get("ck_bin").cloned().unwrap_or_default(),
        "sotaikhoan": m.get("ck_stk").cloned().unwrap_or_default(),
        "chutaikhoan": m.get("ck_chutk").cloned().unwrap_or_default(),
        "pay2s_token_set": !m.get("pay2s_token").map(|s| s.is_empty()).unwrap_or(true),
        "pay2s_api_token_set": !m.get("pay2s_api_token").map(|s| s.is_empty()).unwrap_or(true),
        "pay2s_access_set": !m.get("pay2s_access").map(|s| s.is_empty()).unwrap_or(true),
        "pay2s_secret_set": !m.get("pay2s_secret").map(|s| s.is_empty()).unwrap_or(true),
        "ck_tugan": m.get("ck_tugan").cloned().unwrap_or_else(|| "0".into()),
        "ck_pull": m.get("ck_pull").cloned().unwrap_or_else(|| "1".into()),
        "ck_push": m.get("ck_push").cloned().unwrap_or_else(|| "0".into()),
        "moc": m.get("ck_moc").cloned().unwrap_or_default(),
    }))
}

async fn upsert_meta(client: &Object, key: &str, value: &str) -> Result<(), AppError> {
    client
        .execute(
            "INSERT INTO app_meta (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
            &[&key, &value],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    Ok(())
}

pub async fn ck_save_settings(
    client: &Object,
    ctx: &Ctx,
    args: &Args,
) -> Result<Value, AppError> {
    need(ctx, role::ADMIN)?;
    let bin = arg_str(args, "bin");
    if !bin.is_empty() && bank_name(&bin).is_none() {
        return Err(AppError::msg("Ngân hàng không hợp lệ."));
    }
    let stk = arg_str(args, "sotaikhoan").replace(char::is_whitespace, "");
    let re = Regex::new(r"^[0-9A-Za-z]{4,25}$").unwrap();
    if !stk.is_empty() && !re.is_match(&stk) {
        return Err(AppError::msg(
            "Số tài khoản chỉ gồm chữ số (và chữ), không dấu cách.",
        ));
    }
    let chutk = ascii_content(&arg_str(args, "chutaikhoan"))
        .to_uppercase()
        .trim()
        .to_string();

    upsert_meta(client, "ck_bin", &bin).await?;
    upsert_meta(client, "ck_stk", &stk).await?;
    upsert_meta(client, "ck_chutk", &chutk).await?;

    if let Some(v) = args.get("pay2s_token").and_then(|v| v.as_str()) {
        upsert_meta(client, "pay2s_token", v.trim()).await?;
    }
    let ak = args
        .get("pay2s_access")
        .and_then(|v| v.as_str())
        .map(|s| s.replace(char::is_whitespace, ""));
    let sk = args
        .get("pay2s_secret")
        .and_then(|v| v.as_str())
        .map(|s| s.replace(char::is_whitespace, ""));
    if ak
        .as_ref()
        .map(|s| s.to_uppercase().starts_with("PAY2S"))
        .unwrap_or(false)
        || sk
            .as_ref()
            .map(|s| s.to_uppercase().starts_with("PAY2S"))
            .unwrap_or(false)
    {
        return Err(AppError::msg(
            "Đây là Partner code. Nhập Access key và Secret key (2 ô bên cạnh Partner code ở trang Tích hợp → Website/Application).",
        ));
    }
    if let Some(v) = args.get("pay2s_api_token").and_then(|v| v.as_str()) {
        upsert_meta(client, "pay2s_api_token", v.trim()).await?;
    }
    if let Some(v) = &ak {
        upsert_meta(client, "pay2s_access", v).await?;
    }
    if let Some(v) = &sk {
        upsert_meta(client, "pay2s_secret", v).await?;
    }
    if args.get("pay2s_api_token").is_some() || ak.is_some() || sk.is_some() {
        upsert_meta(client, "pay2s_cach", "").await?;
    }
    Ok(Value::Bool(true))
}

/// Preview QR from current bank settings (no yeucau_ck required).
pub async fn ck_qr_preview(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let bank = bank_info(client)
        .await?
        .ok_or_else(|| AppError::msg("Chưa cài tài khoản nhận chuyển khoản."))?;
    let sotien = args
        .get("sotien")
        .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
        .unwrap_or(0.0);
    let noidung = {
        let s = arg_str(args, "noidung");
        if s.is_empty() {
            ck_noi_dung("TEST000001")
        } else {
            s
        }
    };
    let bin = bank.get("bin").and_then(|v| v.as_str()).unwrap_or("");
    let stk = bank
        .get("sotaikhoan")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let qr = viet_qr_payload(bin, stk, sotien, &noidung);
    Ok(json!({
        "bank": bank,
        "noidung": noidung,
        "sotien": sotien,
        "qr": qr,
    }))
}

fn magd_of(t: &Map<String, Value>) -> String {
    if let Some(id) = t.get("id") {
        let s = match id {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            _ => String::new(),
        };
        if !s.is_empty() {
            return format!("pay2s:{s}");
        }
    }
    let gw = t
        .get("gateway")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let acc = t
        .get("accountNumber")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let tn = t
        .get("transactionNumber")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    format!("pay2s:{gw}:{acc}:{tn}")
}

fn parse_txn_ts(date: &str) -> Option<NaiveDateTime> {
    let re = Regex::new(r"^(\d{4}-\d{2}-\d{2})").ok()?;
    let cap = re.captures(date)?;
    let d = cap.get(1)?.as_str();
    NaiveDateTime::parse_from_str(&format!("{d} 00:00:00"), "%Y-%m-%d %H:%M:%S").ok()
}

/// Ingest Pay2S body `{transactions:[...]}` or a single transfer object.
/// Default: store as `khongkhop` (chờ gán tay) — matches tps59 when `ck_tugan=0`.
pub async fn process_pay2s(
    client: &Object,
    body: &Value,
    nguon: &str,
) -> Result<Value, AppError> {
    let list: Vec<Map<String, Value>> = if let Some(arr) = body
        .get("transactions")
        .and_then(|v| v.as_array())
    {
        arr.iter()
            .filter_map(|v| v.as_object().cloned())
            .collect()
    } else if body.get("transferAmount").is_some() {
        body.as_object()
            .cloned()
            .into_iter()
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let m = meta_map(client).await?;
    let stk = m.get("ck_stk").cloned().unwrap_or_default();
    let tugan = m.get("ck_tugan").map(|s| s.as_str() == "1").unwrap_or(false);
    let moc = m.get("ck_moc").cloned().unwrap_or_default();

    let mut out = Vec::new();
    for t in list {
        let magd = magd_of(&t);
        let exists = query_one_json(
            client,
            "SELECT magd FROM giaodich_ck WHERE magd = $1",
            &[&magd],
        )
        .await
        .map_err(map_db)?;
        if exists.is_some() {
            out.push(json!({"magd": magd, "trangthai": "trung"}));
            continue;
        }

        let content = t
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let content = if content.len() > 500 {
            content[..500].to_string()
        } else {
            content
        };
        let date = t
            .get("transactionDate")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let amount: i64 = t
            .get("transferAmount")
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f.round() as i64)))
            .unwrap_or(0);
        let gateway = t
            .get("gateway")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let account = t
            .get("accountNumber")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let xfer = t
            .get("transferType")
            .and_then(|v| v.as_str())
            .unwrap_or("IN")
            .to_uppercase();
        let raw = Value::Object(t.clone());
        let ngaygd = parse_txn_ts(&date);

        let (trangthai, ghichu): (String, String) = if xfer != "IN" {
            ("boqua".into(), "Tiền ra".into())
        } else if !stk.is_empty() && !account.is_empty() && norm(&account) != norm(&stk) {
            ("boqua".into(), "Không phải tài khoản thu học phí".into())
        } else if !moc.is_empty()
            && Regex::new(r"^\d{4}-\d{2}-\d{2}")
                .unwrap()
                .is_match(&date)
            && date.as_str() <= moc.as_str()
        {
            (
                "boqua".into(),
                format!("Trước mốc bỏ qua {moc}"),
            )
        } else if !tugan {
            ("khongkhop".into(), "Chờ nhân viên gán".into())
        } else {
            // Full auto-match (ck_tugan) deferred — still store for manual assign.
            (
                "khongkhop".into(),
                "Chờ nhân viên gán · tự gán chưa bật đủ trong TPS 3.0".into(),
            )
        };

        let row = client
            .query_opt(
                "INSERT INTO giaodich_ck (magd, nganhang, sotaikhoan, noidung, sotien, ngaygd, trangthai, raw, nguon, ghichu)
                 VALUES ($1,$2,$3,$4,$5,$6,'moi',$7,$8,$9)
                 ON CONFLICT (magd) DO NOTHING
                 RETURNING gdid",
                &[
                    &magd,
                    &gateway,
                    &account,
                    &content,
                    &amount,
                    &ngaygd,
                    &raw,
                    &nguon,
                    &ghichu,
                ],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;

        let Some(r) = row else {
            out.push(json!({"magd": magd, "trangthai": "trung"}));
            continue;
        };
        let gdid: i32 = r.get(0);
        client
            .execute(
                "UPDATE giaodich_ck SET trangthai = $1, ghichu = $2 WHERE gdid = $3",
                &[&trangthai, &ghichu, &gdid],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        out.push(json!({
            "magd": magd,
            "gdid": gdid,
            "trangthai": trangthai,
            "ghichu": ghichu,
        }));
    }
    Ok(Value::Array(out))
}

pub async fn ck_process_pay2s(
    client: &Object,
    ctx: &Ctx,
    args: &Args,
) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let body = args
        .get("body")
        .cloned()
        .unwrap_or_else(|| Value::Object(args.clone()));
    let nguon = {
        let s = arg_str(args, "nguon");
        if s.is_empty() {
            "pay2s".into()
        } else {
            s
        }
    };
    process_pay2s(client, &body, &nguon).await
}

pub async fn ck_gd_list(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .clamp(1, 200) as i64;
    let rows = query_json(
        client,
        "SELECT gdid, magd, nganhang, sotaikhoan, noidung, sotien, ngaygd::text AS ngaygd,
                trangthai, ghichu, nguon, hocsinhid, ycid
           FROM giaodich_ck
          ORDER BY gdid DESC
          LIMIT $1",
        &[&limit],
    )
    .await
    .map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tps_db::{connect, dev_test_config};

    #[test]
    fn noi_dung_prefix() {
        assert_eq!(ck_noi_dung("A001HP000001"), "TT59 A001HP000001");
    }

    #[tokio::test]
    async fn save_settings_and_ingest() {
        let pool = connect(&dev_test_config()).await.unwrap();
        let c = pool.get().await.unwrap();
        let mut ctx = Ctx::default();
        let mut user = Map::new();
        user.insert("nhanvienid".into(), json!(1));
        user.insert("chucvu".into(), json!(1));
        user.insert("quyen".into(), json!([]));
        ctx.user = Some(user);

        let mut args = Args::new();
        args.insert("bin".into(), json!("970436"));
        args.insert("sotaikhoan".into(), json!("0123456789"));
        args.insert("chutaikhoan".into(), json!("Trung tâm Năm Chín"));
        ck_save_settings(&c, &ctx, &args).await.unwrap();

        let info = bank_info(&c).await.unwrap().unwrap();
        assert_eq!(info["bin"], "970436");

        let qr = ck_qr_preview(&c, &ctx, &{
            let mut a = Args::new();
            a.insert("sotien".into(), json!(300000));
            a.insert("noidung".into(), json!("TT59 A001HP000001"));
            a
        })
        .await
        .unwrap();
        assert!(qr["qr"].as_str().unwrap().starts_with("000201"));

        let id = chrono::Utc::now().timestamp_millis();
        let body = json!({
            "transactions": [{
                "id": id,
                "gateway": "VCB",
                "accountNumber": "0123456789",
                "content": "TT59 TEST",
                "transferAmount": 100000,
                "transferType": "IN",
                "transactionDate": "2026-11-15 10:00:00",
                "transactionNumber": format!("TX{id}")
            }]
        });
        let r1 = process_pay2s(&c, &body, "pay2s").await.unwrap();
        assert_eq!(r1[0]["trangthai"], "khongkhop");
        let r2 = process_pay2s(&c, &body, "pay2s").await.unwrap();
        assert_eq!(r2[0]["trangthai"], "trung");
    }
}
