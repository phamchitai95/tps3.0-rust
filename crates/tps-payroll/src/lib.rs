//! Teacher payroll — S4 subset.
//!
//! Full `bangluong.tinhNhap` (lanh/truy/no/phụ thu/tax) is large; this crate ports:
//! - `luong.preview` / `luong.lichsu` (class × rate preview, read-only history)
//! - `bl.ds` / `bl.gv` (month roster + per-GV class breakdown using the same rate model)
//!
//! Write paths (`bl.chot`, khấu trừ, chuyển lương) come in a later slice.

use deadpool_postgres::Object;
use serde_json::{json, Map, Value};
use tps_auth::{need, role};
use tps_db::{query_json, query_one_json, DbError};
use tps_logic::salary_rate;
use tps_types::{AppError, Args, Ctx};

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

fn arg_i32(args: &Args, key: &str) -> Option<i32> {
    args.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            .map(|n| n as i32)
    })
}

fn month_key_ok(thang: &str) -> bool {
    thang.len() == 7 && &thang[2..3] == "-"
}

async fn bang_ti_le(client: &Object) -> Result<Vec<Map<String, Value>>, AppError> {
    query_json(
        client,
        "SELECT ssmin, ssmax, tileluongconst FROM tinhtileluong ORDER BY ssmin",
        &[],
    )
    .await
    .map_err(map_db)
}

/// Port of Dart `computePayroll` — one row per class for month `thang` (MM-YYYY).
pub async fn compute_payroll(
    client: &Object,
    thang: &str,
) -> Result<Vec<Map<String, Value>>, AppError> {
    let bang = bang_ti_le(client).await?;
    let rows = query_json(
        client,
        "SELECT p.lopid, l.tenlop, l.tilechia AS lop_tilechia, l.giaovienid,
                g.hotengv, g.hotendaydu, g.tilechia AS gv_tilechia,
                SUM(CASE WHEN COALESCE(p.datraluong,0) = 0 THEN p.sotien ELSE 0 END)::bigint AS chuachot,
                SUM(CASE WHEN COALESCE(p.datraluong,0) = 1 THEN p.sotien ELSE 0 END)::bigint AS dachot,
                COUNT(*)::int AS sophieu,
                (SELECT COUNT(*)::int FROM danhsachlop d
                  WHERE d.lopid = p.lopid AND d.ngaykthoc IS NULL) AS siso
           FROM phieuthutien p
           JOIN lophoc l ON l.lopid = p.lopid
           LEFT JOIN giaovien g ON g.giaovienid = l.giaovienid
          WHERE p.thangdongtien = $1
          GROUP BY p.lopid, l.tenlop, l.tilechia, l.giaovienid,
                   g.hotengv, g.hotendaydu, g.tilechia
          ORDER BY g.hotengv NULLS LAST, l.tenlop",
        &[&thang],
    )
    .await
    .map_err(map_db)?;

    let mut out = Vec::new();
    for mut r in rows {
        let mut lop = Map::new();
        lop.insert(
            "tilechia".into(),
            r.get("lop_tilechia").cloned().unwrap_or(json!(0)),
        );
        let mut gv = Map::new();
        gv.insert(
            "tilechia".into(),
            r.get("gv_tilechia").cloned().unwrap_or(json!(0)),
        );
        let siso = r
            .get("siso")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let sr = salary_rate(Some(&lop), Some(&gv), siso, &bang);
        let chuachot = r
            .get("chuachot")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let luong = ((chuachot as f64) * sr.rate / 100.0 + 0.5).floor() as i64;
        r.insert("tile".into(), json!(sr.rate));
        r.insert("nguontile".into(), json!(sr.nguon));
        r.insert("luong".into(), json!(luong));
        out.push(r);
    }
    Ok(out)
}

pub async fn luong_preview(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::ADMIN)?;
    let thang = arg_str(args, "thang");
    if !month_key_ok(&thang) {
        return Err(AppError::msg("Thiếu tháng (MM-YYYY)."));
    }
    let lop = compute_payroll(client, &thang).await?;
    let dachot = query_json(
        client,
        "SELECT r.giaovienid, SUM(r.tienluong)::bigint AS luong
           FROM recordtinhluonggv r
          WHERE r.thangdongtien = $1
          GROUP BY r.giaovienid",
        &[&thang],
    )
    .await
    .map_err(map_db)?;
    let gvs = query_json(
        client,
        "SELECT giaovienid, hotengv, hotendaydu, COALESCE(tientaikhoan,0)::bigint AS tientaikhoan
           FROM giaovien ORDER BY hotengv",
        &[],
    )
    .await
    .map_err(map_db)?;

    let mut by_gv = Vec::new();
    for g in &gvs {
        let gvid = g.get("giaovienid").and_then(|v| v.as_i64());
        let ls: Vec<_> = lop
            .iter()
            .filter(|x| x.get("giaovienid").and_then(|v| v.as_i64()) == gvid)
            .collect();
        let hocphi: i64 = ls
            .iter()
            .map(|x| x.get("chuachot").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        let luongtam: i64 = ls
            .iter()
            .map(|x| x.get("luong").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        let luongdachot = dachot
            .iter()
            .find(|x| x.get("giaovienid").and_then(|v| v.as_i64()) == gvid)
            .and_then(|x| x.get("luong").and_then(|v| v.as_i64()))
            .unwrap_or(0);
        let tk = g
            .get("tientaikhoan")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if hocphi == 0 && luongdachot == 0 && tk == 0 {
            continue;
        }
        let mut row = g.clone();
        row.insert("hocphi".into(), json!(hocphi));
        row.insert("luongtam".into(), json!(luongtam));
        row.insert("luongdachot".into(), json!(luongdachot));
        by_gv.push(Value::Object(row));
    }

    let khongcogv: Vec<Value> = lop
        .iter()
        .filter(|x| x.get("giaovienid").map(|v| v.is_null()).unwrap_or(true))
        .cloned()
        .map(Value::Object)
        .collect();

    Ok(json!({
        "lop": lop.into_iter().map(Value::Object).collect::<Vec<_>>(),
        "giaovien": by_gv,
        "khongcogv": khongcogv,
        "thang": thang,
        "note": "S4 subset: tỉ lệ × HP chưa chốt (computePayroll). Bảng lương đầy đủ (lanh/truy/no/thuế) port sau.",
    }))
}

pub async fn luong_lichsu(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::ADMIN)?;
    let thang = arg_str(args, "thang");
    if !month_key_ok(&thang) {
        return Err(AppError::msg("Thiếu tháng (MM-YYYY)."));
    }
    let rows = query_json(
        client,
        "SELECT r.lineid, r.thoigian::text AS thoigian, g.hotengv, g.hotendaydu, l.tenlop,
                r.hpdathu, r.siso, r.tileluongconst, r.tienluong
           FROM recordtinhluonggv r
           LEFT JOIN giaovien g ON g.giaovienid = r.giaovienid
           LEFT JOIN lophoc l ON l.lopid = r.lopid
          WHERE r.thangdongtien = $1
          ORDER BY g.hotengv, l.tenlop, r.lineid",
        &[&thang],
    )
    .await
    .map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

/// Simplified `bl.ds` — roster from computePayroll + chốt snapshot flags.
pub async fn bl_ds(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let thang = arg_str(args, "thang");
    if !month_key_ok(&thang) {
        return Err(AppError::msg("Thiếu tháng (MM-YYYY)."));
    }
    let preview = compute_payroll(client, &thang).await?;
    let chot_rows = query_json(
        client,
        "SELECT c.chotid, c.giaovienid, c.thucnhan, c.thoigian::text AS thoigian,
                c.xuatluc::text AS xuatluc
           FROM chotluong c
          WHERE c.thang = $1",
        &[&thang],
    )
    .await
    .map_err(map_db)
    .unwrap_or_default();

    let gvs = query_json(
        client,
        "SELECT giaovienid, hotengv, hotendaydu FROM giaovien ORDER BY hotengv",
        &[],
    )
    .await
    .map_err(map_db)?;

    let mut out = Vec::new();
    let mut tong_gv = 0i64;
    let mut n_chot = 0;
    let mut n_xuat = 0;
    let mut n_dang = 0;

    for g in gvs {
        let gvid = g.get("giaovienid").and_then(|v| v.as_i64());
        let ls: Vec<_> = preview
            .iter()
            .filter(|x| x.get("giaovienid").and_then(|v| v.as_i64()) == gvid)
            .collect();
        if ls.is_empty() {
            // still show if already chốt
            if !chot_rows
                .iter()
                .any(|c| c.get("giaovienid").and_then(|v| v.as_i64()) == gvid)
            {
                continue;
            }
        }
        let hocphi: i64 = ls
            .iter()
            .map(|x| x.get("chuachot").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        let gv: i64 = ls
            .iter()
            .map(|x| x.get("luong").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        let chot = chot_rows
            .iter()
            .find(|c| c.get("giaovienid").and_then(|v| v.as_i64()) == gvid);
        let trangthai = if let Some(c) = chot {
            if c.get("xuatluc").map(|v| !v.is_null()).unwrap_or(false) {
                n_xuat += 1;
                "xuat"
            } else {
                n_chot += 1;
                "chot"
            }
        } else if gv > 0 || hocphi > 0 {
            n_dang += 1;
            "chua"
        } else {
            continue;
        };
        let thucnhan = chot
            .and_then(|c| c.get("thucnhan").and_then(|v| v.as_i64()))
            .unwrap_or(gv);
        tong_gv += thucnhan;
        out.push(json!({
            "giaovienid": gvid,
            "hotengv": g.get("hotengv"),
            "hotendaydu": g.get("hotendaydu"),
            "hocphi": hocphi,
            "gv": gv,
            "thucnhan": thucnhan,
            "trangthai": trangthai,
            "chotid": chot.and_then(|c| c.get("chotid").cloned()),
            "solop": ls.len(),
        }));
    }

    Ok(json!({
        "thang": thang,
        "gv": out,
        "tiendo": {
            "tong": out.len(),
            "chot": n_chot,
            "xuat": n_xuat,
            "dang": n_dang,
        },
        "tong": { "gv": tong_gv, "thucnhan": tong_gv },
        "subset": true,
    }))
}

pub async fn bl_gv(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let thang = arg_str(args, "thang");
    let gvid = arg_i32(args, "giaovienid")
        .ok_or_else(|| AppError::msg("Thiếu giaovienid."))?;
    if !month_key_ok(&thang) {
        return Err(AppError::msg("Thiếu tháng (MM-YYYY)."));
    }
    let preview = compute_payroll(client, &thang).await?;
    let lop: Vec<Value> = preview
        .into_iter()
        .filter(|x| {
            x.get("giaovienid")
                .and_then(|v| v.as_i64())
                .map(|id| id as i32)
                == Some(gvid)
        })
        .map(Value::Object)
        .collect();
    let gv = query_one_json(
        client,
        "SELECT giaovienid, hotengv, hotendaydu, COALESCE(tientaikhoan,0)::bigint AS tientaikhoan
           FROM giaovien WHERE giaovienid = $1",
        &[&gvid],
    )
    .await
    .map_err(map_db)?
    .ok_or_else(|| AppError::msg("Không tìm thấy giáo viên."))?;

    let gv_sum: i64 = lop
        .iter()
        .filter_map(|v| v.get("luong").and_then(|x| x.as_i64()))
        .sum();
    let hp_sum: i64 = lop
        .iter()
        .filter_map(|v| v.get("chuachot").and_then(|x| x.as_i64()))
        .sum();

    Ok(json!({
        "thang": thang,
        "giaovien": gv,
        "lop": lop,
        "tong": { "hocphi": hp_sum, "gv": gv_sum },
        "subset": true,
    }))
}

pub async fn phieuchi_list(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let limit = args
        .get("limit")
        .and_then(|v| v.as_i64())
        .unwrap_or(50)
        .clamp(1, 200) as i64;
    let rows = query_json(
        client,
        "SELECT p.lineid, p.tennguoinhan, p.lido, p.sotien, p.thoigian::text AS thoigian,
                p.hinhthuc, p.ghichu, p.giaovienid, p.duyet, g.hotengv
           FROM phieuchi p
           LEFT JOIN giaovien g ON g.giaovienid = p.giaovienid
          ORDER BY p.lineid DESC
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
    use chrono::Datelike;
    use tps_db::{connect, dev_test_config};

    #[tokio::test]
    async fn preview_runs_on_fixture() {
        let pool = connect(&dev_test_config()).await.unwrap();
        let c = pool.get().await.unwrap();
        let mut ctx = Ctx::default();
        let mut user = Map::new();
        user.insert("nhanvienid".into(), json!(1));
        user.insert("chucvu".into(), json!(1));
        user.insert("quyen".into(), json!([]));
        ctx.user = Some(user);
        let mut args = Args::new();
        // Use current VN month if seed has no receipts — still must return shape
        let now = chrono::Utc::now() + chrono::Duration::hours(7);
        let thang = format!("{:02}-{}", now.month(), now.year());
        args.insert("thang".into(), json!(thang));
        let r = luong_preview(&c, &ctx, &args).await.unwrap();
        assert!(r.get("lop").is_some());
        assert!(r.get("giaovien").is_some());
        let ds = bl_ds(&c, &ctx, &args).await.unwrap();
        assert_eq!(ds["thang"], thang);
    }
}
