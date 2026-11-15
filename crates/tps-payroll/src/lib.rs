//! Teacher payroll — S4 + S4b subset.
//!
//! Full `bangluong.tinhNhap` (lanh/truy/no/phụ thu) is large; this crate ports:
//! - `luong.preview` / `luong.lichsu` (class × rate preview, read-only history)
//! - `bl.ds` / `bl.gv` (month roster + per-GV class breakdown using the same rate model)
//! - S4b: `bl.khautru` / `bl.khautruXoa` / `bl.chot` / `bl.mo`
//! - S4b: `bl.ck.ds` / `bl.ck.xuat` / `bl.ck.tai` / `bl.ck.huy` (CSV, không xlsx MB)

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
    thang.len() == 7
        && thang.as_bytes().get(2) == Some(&b'-')
        && thang[0..2].chars().all(|c| c.is_ascii_digit())
        && thang[3..7].chars().all(|c| c.is_ascii_digit())
}

fn month_sort_key(thang: &str) -> String {
    // MM-YYYY → YYYYMM for lexical compare
    format!("{}{}", &thang[3..7], &thang[0..2])
}

fn month_label(thang: &str) -> String {
    format!("T{}/{}", &thang[0..2], &thang[3..7])
}

fn nv_id(ctx: &Ctx) -> Option<i32> {
    ctx.user
        .as_ref()
        .and_then(|u| u.get("nhanvienid"))
        .and_then(|v| v.as_i64())
        .map(|n| n as i32)
}

fn apply_thue(tong_gv: i64, trichthue: &str, tamung: i64, khac: i64) -> Map<String, Value> {
    let thue = if trichthue == "08" {
        0
    } else {
        ((tong_gv as f64) * 10.0 / 100.0).round() as i64
    };
    let trich = if trichthue == "08" { "08" } else { "10" };
    let mut m = Map::new();
    m.insert("gv".into(), json!(tong_gv));
    m.insert("trichthue".into(), json!(trich));
    m.insert("thue".into(), json!(thue));
    m.insert("tamung".into(), json!(tamung));
    m.insert("khac".into(), json!(khac));
    m.insert("thucnhan".into(), json!(tong_gv - thue - tamung - khac));
    m
}

async fn load_khautru(
    client: &Object,
    gv: i32,
    thang: &str,
) -> Result<Vec<Map<String, Value>>, AppError> {
    query_json(
        client,
        "SELECT k.id, k.giaovienid, k.loai, k.sotien::bigint AS sotien, k.lydo,
                k.thoigian::text AS thoigian, k.phieuchiid,
                COALESCE(NULLIF(TRIM(n.hoten), ''), n.hotennv) AS nguoighi
           FROM luong_khautru k
           LEFT JOIN nhanvien n ON n.nhanvienid = k.nhanvienid
          WHERE k.giaovienid = $1 AND k.thang = $2
          ORDER BY k.id",
        &[&gv, &thang],
    )
    .await
    .map_err(map_db)
}

fn sum_khautru(rows: &[Map<String, Value>], loai: &str) -> i64 {
    rows.iter()
        .filter(|r| r.get("loai").and_then(|v| v.as_str()) == Some(loai))
        .map(|r| r.get("sotien").and_then(|v| v.as_i64()).unwrap_or(0))
        .sum()
}

async fn load_chot(
    client: &tokio_postgres::Client,
    gv: i32,
    thang: &str,
) -> Result<Option<Map<String, Value>>, AppError> {
    let rows = client
        .query(
            "SELECT c.chotid, c.giaovienid, c.thang, c.trangthai, c.lan,
                    c.thoigian::text AS thoigian, c.xuatluc::text AS xuatluc,
                    c.tonggv::bigint AS tonggv, c.tongthue::bigint AS tongthue,
                    c.thucnhan::bigint AS thucnhan, c.tongtamung::bigint AS tongtamung,
                    c.tongkhac::bigint AS tongkhac, c.trichthue, c.mst, c.stk,
                    c.nganhang, c.chutk, c.solop,
                    COALESCE(NULLIF(TRIM(n.hoten), ''), n.hotennv) AS nguoichot
               FROM chotluong c
               LEFT JOIN nhanvien n ON n.nhanvienid = c.nhanvienid
              WHERE c.giaovienid = $1 AND c.thang = $2",
            &[&gv, &thang],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    let Some(r) = rows.first() else {
        return Ok(None);
    };
    Ok(Some(Map::from_iter([
        ("chotid".into(), json!(r.get::<_, i32>(0))),
        ("giaovienid".into(), json!(r.get::<_, i32>(1))),
        ("thang".into(), json!(r.get::<_, String>(2))),
        ("trangthai".into(), json!(r.get::<_, String>(3))),
        ("lan".into(), json!(r.get::<_, i32>(4))),
        ("thoigian".into(), json!(r.get::<_, Option<String>>(5))),
        ("xuatluc".into(), json!(r.get::<_, Option<String>>(6))),
        ("tonggv".into(), json!(r.get::<_, i64>(7))),
        ("tongthue".into(), json!(r.get::<_, i64>(8))),
        ("thucnhan".into(), json!(r.get::<_, i64>(9))),
        ("tongtamung".into(), json!(r.get::<_, i64>(10))),
        ("tongkhac".into(), json!(r.get::<_, i64>(11))),
        ("trichthue".into(), json!(r.get::<_, String>(12))),
        ("mst".into(), json!(r.get::<_, Option<String>>(13))),
        ("stk".into(), json!(r.get::<_, Option<String>>(14))),
        ("nganhang".into(), json!(r.get::<_, Option<String>>(15))),
        ("chutk".into(), json!(r.get::<_, Option<String>>(16))),
        ("solop".into(), json!(r.get::<_, i32>(17))),
        ("nguoichot".into(), json!(r.get::<_, Option<String>>(18))),
    ])))
}

async fn assert_chua_chot(
    client: &tokio_postgres::Client,
    gv: i32,
    thang: &str,
) -> Result<(), AppError> {
    if let Some(c) = load_chot(client, gv, thang).await? {
        if c.get("trangthai").and_then(|v| v.as_str()) == Some("chot") {
            let luc = c
                .get("thoigian")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            return Err(AppError::msg(format!(
                "Bảng lương {} của giáo viên này đã chốt lúc {}. Admin mở lại rồi mới sửa được.",
                month_label(thang),
                luc.chars().take(16).collect::<String>()
            )));
        }
    }
    Ok(())
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
                SUM(CASE WHEN p.chotluongid IS NULL AND COALESCE(p.datraluong,0) = 0
                         THEN p.sotien ELSE 0 END)::bigint AS chuachot,
                SUM(CASE WHEN p.chotluongid IS NOT NULL OR COALESCE(p.datraluong,0) <> 0
                         THEN p.sotien ELSE 0 END)::bigint AS dachot,
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
                c.xuatluc::text AS xuatluc, c.trangthai
           FROM chotluong c
          WHERE c.thang = $1 AND c.trangthai = 'chot'",
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
    chi_tiet_gv(client, gvid, &thang).await
}

async fn chi_tiet_gv(client: &Object, gvid: i32, thang: &str) -> Result<Value, AppError> {
    let mut gv = query_one_json(
        client,
        "SELECT giaovienid, hotengv, hotendaydu, COALESCE(trichthue,'10') AS trichthue,
                mst, stk, nganhang, chutk,
                COALESCE(tientaikhoan,0)::bigint AS tientaikhoan
           FROM giaovien WHERE giaovienid = $1",
        &[&gvid],
    )
    .await
    .map_err(map_db)?
    .ok_or_else(|| AppError::msg("Không tìm thấy giáo viên."))?;

    let chot = load_chot(client.as_ref(), gvid, thang).await?;
    let da_chot = chot
        .as_ref()
        .and_then(|c| c.get("trangthai").and_then(|v| v.as_str()))
        == Some("chot");

    let preview = compute_payroll(client, thang).await?;
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
    let gv_sum: i64 = lop
        .iter()
        .filter_map(|v| v.get("luong").and_then(|x| x.as_i64()))
        .sum();
    let hp_sum: i64 = lop
        .iter()
        .filter_map(|v| v.get("chuachot").and_then(|x| x.as_i64()))
        .sum();

    let khautru: Vec<Value> = if da_chot {
        let chotid = chot
            .as_ref()
            .and_then(|c| c.get("chotid").and_then(|v| v.as_i64()))
            .unwrap_or(0) as i32;
        query_json(
            client,
            "SELECT khautruid AS id, loai, sotien::bigint AS sotien, lydo,
                    nguoighi, thoigian::text AS thoigian
               FROM chotluong_khautru WHERE chotid = $1 ORDER BY id",
            &[&chotid],
        )
        .await
        .map_err(map_db)?
        .into_iter()
        .map(Value::Object)
        .collect()
    } else {
        load_khautru(client, gvid, thang)
            .await?
            .into_iter()
            .map(Value::Object)
            .collect()
    };

    let (tong_gv, trich, tamung, khac) = if da_chot {
        let c = chot.as_ref().unwrap();
        (
            c.get("tonggv").and_then(|v| v.as_i64()).unwrap_or(gv_sum),
            c.get("trichthue")
                .and_then(|v| v.as_str())
                .unwrap_or("10")
                .to_string(),
            c.get("tongtamung").and_then(|v| v.as_i64()).unwrap_or(0),
            c.get("tongkhac").and_then(|v| v.as_i64()).unwrap_or(0),
        )
    } else {
        let tamung = khautru
            .iter()
            .filter(|k| k.get("loai").and_then(|v| v.as_str()) == Some("tamung"))
            .map(|k| k.get("sotien").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        let khac = khautru
            .iter()
            .filter(|k| k.get("loai").and_then(|v| v.as_str()) == Some("khac"))
            .map(|k| k.get("sotien").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        let trich = gv
            .get("trichthue")
            .and_then(|v| v.as_str())
            .unwrap_or("10")
            .to_string();
        (gv_sum, trich, tamung, khac)
    };

    if da_chot {
        if let Some(c) = &chot {
            for k in ["trichthue", "mst", "stk", "nganhang", "chutk"] {
                if let Some(v) = c.get(k) {
                    gv.insert(k.into(), v.clone());
                }
            }
        }
    }

    let mut tong = apply_thue(tong_gv, &trich, tamung, khac);
    tong.insert("hocphi".into(), json!(hp_sum));
    if da_chot {
        if let Some(c) = &chot {
            tong.insert(
                "thucnhan".into(),
                c.get("thucnhan").cloned().unwrap_or(tong["thucnhan"].clone()),
            );
            tong.insert(
                "thue".into(),
                c.get("tongthue").cloned().unwrap_or(tong["thue"].clone()),
            );
        }
    }

    let trangthai = if !da_chot {
        "chua"
    } else if chot
        .as_ref()
        .and_then(|c| c.get("xuatluc"))
        .map(|v| !v.is_null())
        .unwrap_or(false)
    {
        "xuat"
    } else {
        "chot"
    };

    Ok(json!({
        "thang": thang,
        "giaovien": gv,
        "lop": lop,
        "tong": tong,
        "khautru": khautru,
        "chot": chot,
        "trangthai": trangthai,
        "subset": true,
        "note": "S4b: tổng kỳ = tỉ lệ × HP chưa gắn chotluongid (chưa port lanh/truy/no đầy đủ).",
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

/// `bl.khautru` — thêm / sửa khấu trừ sau thuế (tamung|khac).
pub async fn bl_khautru(
    client: &mut Object,
    ctx: &Ctx,
    args: &Args,
) -> Result<Value, AppError> {
    need(ctx, role::NHAN_VIEN)?;
    let thang = arg_str(args, "thang");
    let gv = arg_i32(args, "giaovienid").unwrap_or(0);
    if !month_key_ok(&thang) || gv <= 0 {
        return Err(AppError::msg("Thiếu tháng / giaovienid."));
    }
    let loai = arg_str(args, "loai");
    if loai != "tamung" && loai != "khac" {
        return Err(AppError::msg("Chọn loại khấu trừ: Tạm ứng hoặc Khác."));
    }
    let so = args
        .get("sotien")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
        .unwrap_or(0);
    if so <= 0 {
        return Err(AppError::msg("Số tiền khấu trừ phải lớn hơn 0."));
    }
    let lydo = arg_str(args, "lydo");
    if lydo.is_empty() {
        return Err(AppError::msg("Ghi lý do khấu trừ."));
    }
    let ld: String = lydo.chars().take(500).collect();
    let id = arg_i32(args, "id");
    let nvid = nv_id(ctx);

    let tx = client
        .transaction()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    tx.execute("SELECT pg_advisory_xact_lock(59001, $1)", &[&gv])
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    assert_chua_chot(tx.client(), gv, &thang).await?;

    let ten: Option<String> = tx
        .query_opt(
            "SELECT COALESCE(hotendaydu, hotengv) FROM giaovien WHERE giaovienid = $1",
            &[&gv],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?
        .map(|r| r.get(0));
    if ten.is_none() {
        return Err(AppError::msg("Không tìm thấy giáo viên."));
    }

    if let Some(id) = id {
        let cu = tx
            .query_opt(
                "SELECT loai, sotien, lydo, phieuchiid FROM luong_khautru
                  WHERE id = $1 AND giaovienid = $2 AND thang = $3 FOR UPDATE",
                &[&id, &gv, &thang],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        let Some(cu) = cu else {
            return Err(AppError::msg("Không tìm thấy khoản khấu trừ này."));
        };
        let pc: Option<i32> = cu.get(3);
        if pc.is_some() {
            return Err(AppError::msg(format!(
                "Khoản này từ phiếu chi {}. Sửa / huỷ ở Phiếu thu/chi khác.",
                pc.unwrap()
            )));
        }
        tx.execute(
            "UPDATE luong_khautru SET loai = $2, sotien = $3, lydo = $4, nhanvienid = $5,
                    thoigian = date_trunc('second', now()) WHERE id = $1",
            &[&id, &loai, &so, &ld, &nvid],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    } else {
        tx.execute(
            "INSERT INTO luong_khautru (giaovienid, thang, loai, sotien, lydo, nhanvienid)
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[&gv, &thang, &loai, &so, &ld, &nvid],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    }
    tx.commit()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    chi_tiet_gv(client, gv, &thang).await
}

pub async fn bl_khautru_xoa(
    client: &mut Object,
    ctx: &Ctx,
    args: &Args,
) -> Result<Value, AppError> {
    need(ctx, role::NHAN_VIEN)?;
    let thang = arg_str(args, "thang");
    let gv = arg_i32(args, "giaovienid").unwrap_or(0);
    let id = arg_i32(args, "id").ok_or_else(|| AppError::msg("Thiếu khoản khấu trừ."))?;
    if !month_key_ok(&thang) || gv <= 0 {
        return Err(AppError::msg("Thiếu tháng / giaovienid."));
    }
    let tx = client
        .transaction()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    tx.execute("SELECT pg_advisory_xact_lock(59001, $1)", &[&gv])
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    assert_chua_chot(tx.client(), gv, &thang).await?;
    let pc: Option<i32> = tx
        .query_opt(
            "SELECT phieuchiid FROM luong_khautru WHERE id = $1",
            &[&id],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?
        .and_then(|r| r.get(0));
    if let Some(pc) = pc {
        return Err(AppError::msg(format!(
            "Khoản này từ phiếu chi {pc}. Huỷ phiếu chi đó ở Phiếu thu/chi khác."
        )));
    }
    let n = tx
        .execute(
            "DELETE FROM luong_khautru WHERE id = $1 AND giaovienid = $2 AND thang = $3",
            &[&id, &gv, &thang],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    if n == 0 {
        return Err(AppError::msg("Không tìm thấy khoản khấu trừ này."));
    }
    tx.commit()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    chi_tiet_gv(client, gv, &thang).await
}

/// `bl.chot` — chốt bảng lương 1 GV (subset: tỉ lệ × HP + thuế + khấu trừ).
pub async fn bl_chot(
    client: &mut Object,
    ctx: &Ctx,
    args: &Args,
) -> Result<Value, AppError> {
    need(ctx, role::NHAN_VIEN)?;
    let thang = arg_str(args, "thang");
    let gv = arg_i32(args, "giaovienid").unwrap_or(0);
    if !month_key_ok(&thang) || gv <= 0 {
        return Err(AppError::msg("Thiếu tháng / giaovienid."));
    }
    let nvid = nv_id(ctx);
    let expect_tong = args
        .get("tonggv")
        .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)));

    // Compute outside tx (read), then lock + write.
    let preview = compute_payroll(client, &thang).await?;
    let lop_rows: Vec<_> = preview
        .into_iter()
        .filter(|x| {
            x.get("giaovienid")
                .and_then(|v| v.as_i64())
                .map(|id| id as i32)
                == Some(gv)
        })
        .collect();
    if lop_rows.is_empty() {
        return Err(AppError::msg(format!(
            "Giáo viên này không có lớp nào trong {}.",
            month_label(&thang)
        )));
    }
    let tong_gv: i64 = lop_rows
        .iter()
        .map(|x| x.get("luong").and_then(|v| v.as_i64()).unwrap_or(0))
        .sum();
    let tong_hp: i64 = lop_rows
        .iter()
        .map(|x| x.get("chuachot").and_then(|v| v.as_i64()).unwrap_or(0))
        .sum();

    let gvi = query_one_json(
        client,
        "SELECT COALESCE(trichthue,'10') AS trichthue, mst, stk, nganhang, chutk
           FROM giaovien WHERE giaovienid = $1",
        &[&gv],
    )
    .await
    .map_err(map_db)?
    .ok_or_else(|| AppError::msg("Không tìm thấy giáo viên."))?;
    let trich = gvi
        .get("trichthue")
        .and_then(|v| v.as_str())
        .unwrap_or("10")
        .to_string();
    let khautru = load_khautru(client, gv, &thang).await?;
    let tamung = sum_khautru(&khautru, "tamung");
    let khac = sum_khautru(&khautru, "khac");
    let tong = apply_thue(tong_gv, &trich, tamung, khac);
    let tong_gv_final = tong.get("gv").and_then(|v| v.as_i64()).unwrap_or(tong_gv);
    if let Some(exp) = expect_tong {
        if exp != tong_gv_final {
            return Err(AppError::msg(format!(
                "Vừa có phiếu thu mới (giáo viên nhận bây giờ là {tong_gv_final} đ). Bấm Tải lại rồi xem lại trước khi chốt."
            )));
        }
    }

    let thue = tong.get("thue").and_then(|v| v.as_i64()).unwrap_or(0);
    let thucnhan = tong.get("thucnhan").and_then(|v| v.as_i64()).unwrap_or(0);
    let trich_s = tong
        .get("trichthue")
        .and_then(|v| v.as_str())
        .unwrap_or("10")
        .to_string();
    let mst = gvi.get("mst").and_then(|v| v.as_str()).map(|s| s.to_string());
    let stk = gvi.get("stk").and_then(|v| v.as_str()).map(|s| s.to_string());
    let nganhang = gvi
        .get("nganhang")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let chutk = gvi
        .get("chutk")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let solop = lop_rows.len() as i32;
    let sort_this = month_sort_key(&thang);

    let tx = client
        .transaction()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    tx.execute("SELECT pg_advisory_xact_lock(59001, $1)", &[&gv])
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    assert_chua_chot(tx.client(), gv, &thang).await?;

    // Block if older month still open (mo).
    let mo = tx
        .query(
            "SELECT thang FROM chotluong WHERE giaovienid = $1 AND trangthai = 'mo'
              AND (substring(thang, 4, 4) || substring(thang, 1, 2)) < $2
              ORDER BY substring(thang, 4, 4), substring(thang, 1, 2) LIMIT 1",
            &[&gv, &sort_this],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    if let Some(r) = mo.first() {
        let t0: String = r.get(0);
        return Err(AppError::msg(format!(
            "Bảng lương {} của giáo viên này đang mở lại. Chốt lại tháng đó trước.",
            month_label(&t0)
        )));
    }

    let cu = load_chot(tx.client(), gv, &thang).await?;
    let chotid: i32 = if let Some(cu) = &cu {
        let id = cu.get("chotid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        tx.execute(
            "UPDATE chotluong SET trangthai = 'chot', lan = lan + 1,
                    thoigian = date_trunc('second', now()), nhanvienid = $2,
                    tonglanh = $3, tongtruy = 0, tongdc = 0, tonggv = $3, tongtt = 0,
                    tongno = 0, sono = 0, quahan = 0, tonghplanh = $4, tonghptruy = 0,
                    solop = $5, xuatluc = NULL, file = NULL,
                    trichthue = $6, tongthue = $7, thucnhan = $8,
                    mst = $9, stk = $10, nganhang = $11, chutk = $12,
                    tongtamung = $13, tongkhac = $14
              WHERE chotid = $1",
            &[
                &id, &nvid, &tong_gv_final, &tong_hp, &solop, &trich_s, &thue, &thucnhan, &mst,
                &stk, &nganhang, &chutk, &tamung, &khac,
            ],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
        tx.execute("DELETE FROM chotluong_dong WHERE chotid = $1", &[&id])
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        tx.execute("DELETE FROM chotluong_lop WHERE chotid = $1", &[&id])
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        tx.execute("DELETE FROM chotluong_khautru WHERE chotid = $1", &[&id])
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        id
    } else {
        let row = tx
            .query_one(
                "INSERT INTO chotluong (
                    giaovienid, thang, trangthai, lan, thoigian, nhanvienid,
                    tonglanh, tongtruy, tongdc, tonggv, tongtt, tongno, sono, quahan,
                    tonghplanh, tonghptruy, solop, trichthue, tongthue, thucnhan,
                    mst, stk, nganhang, chutk, tongtamung, tongkhac)
                 VALUES ($1, $2, 'chot', 1, date_trunc('second', now()), $3,
                    $4, 0, 0, $4, 0, 0, 0, 0,
                    $5, 0, $6, $7, $8, $9,
                    $10, $11, $12, $13, $14, $15)
                 RETURNING chotid",
                &[
                    &gv, &thang, &nvid, &tong_gv_final, &tong_hp, &solop, &trich_s, &thue,
                    &thucnhan, &mst, &stk, &nganhang, &chutk, &tamung, &khac,
                ],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        row.get(0)
    };

    for k in &khautru {
        let kid = k.get("id").and_then(|v| v.as_i64()).map(|n| n as i32);
        let loai = k.get("loai").and_then(|v| v.as_str()).unwrap_or("khac");
        let so = k.get("sotien").and_then(|v| v.as_i64()).unwrap_or(0);
        let lydo = k.get("lydo").and_then(|v| v.as_str());
        let nguoi = k.get("nguoighi").and_then(|v| v.as_str());
        let tg = k.get("thoigian").and_then(|v| v.as_str());
        let tg_parsed = tg.and_then(|s| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .or_else(|_| chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
                .ok()
        });
        tx.execute(
            "INSERT INTO chotluong_khautru (chotid, khautruid, loai, sotien, lydo, nguoighi, thoigian)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[&chotid, &kid, &loai, &so, &lydo, &nguoi, &tg_parsed],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    }

    for g in &lop_rows {
        let lopid = g.get("lopid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let tenlop = g.get("tenlop").and_then(|v| v.as_str());
        let siso = g.get("siso").and_then(|v| v.as_i64()).map(|n| n as i32);
        let tile = g.get("tile").cloned().unwrap_or(json!(0));
        let tile_f: f64 = tile.as_f64().unwrap_or(0.0);
        tx.execute(
            "INSERT INTO chotluong_lop (chotid, lopid, tenlop, siso, tile, ghichu, quahan, quahantien)
             VALUES ($1, $2, $3, $4, $5, NULL, 0, 0)",
            &[&chotid, &lopid, &tenlop, &siso, &tile_f],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    }

    // Attach unchốt receipts of this GV's classes for the month.
    tx.execute(
        "UPDATE phieuthutien p SET chotluongid = $1
           FROM lophoc l
          WHERE p.lopid = l.lopid AND l.giaovienid = $2
            AND p.thangdongtien = $3
            AND p.chotluongid IS NULL
            AND COALESCE(p.datraluong, 0) = 0",
        &[&chotid, &gv, &thang],
    )
    .await
    .map_err(|e| AppError::msg(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;

    let mut out = chi_tiet_gv(client, gv, &thang).await?;
    if let Some(obj) = out.as_object_mut() {
        obj.insert(
            "datreo".into(),
            json!({ "soky": 0, "sohv": 0, "tien": 0 }),
        );
    }
    Ok(out)
}

/// `bl.mo` — Admin mở lại bảng đã chốt.
pub async fn bl_mo(
    client: &mut Object,
    ctx: &Ctx,
    args: &Args,
) -> Result<Value, AppError> {
    need(ctx, role::ADMIN)?;
    let thang = arg_str(args, "thang");
    let gv = arg_i32(args, "giaovienid").unwrap_or(0);
    let lydo = arg_str(args, "lydo");
    if !month_key_ok(&thang) || gv <= 0 {
        return Err(AppError::msg("Thiếu tháng / giaovienid."));
    }
    if lydo.is_empty() {
        return Err(AppError::msg("Ghi lý do mở lại."));
    }
    let tx = client
        .transaction()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    let c = load_chot(tx.client(), gv, &thang).await?;
    let Some(c) = c else {
        return Err(AppError::msg("Bảng lương này chưa chốt."));
    };
    if c.get("trangthai").and_then(|v| v.as_str()) != Some("chot") {
        return Err(AppError::msg("Bảng lương này chưa chốt."));
    }
    let id = c.get("chotid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    tx.execute(
        "UPDATE phieuthutien SET chotluongid = NULL WHERE chotluongid = $1",
        &[&id],
    )
    .await
    .map_err(|e| AppError::msg(e.to_string()))?;
    let _ = tx
        .execute(
            "UPDATE hoantra SET chotluongid = NULL WHERE chotluongid = $1",
            &[&id],
        )
        .await;
    tx.execute("DELETE FROM chotluong_dong WHERE chotid = $1", &[&id])
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    tx.execute("DELETE FROM chotluong_lop WHERE chotid = $1", &[&id])
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    tx.execute("DELETE FROM chotluong_khautru WHERE chotid = $1", &[&id])
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    tx.execute(
        "UPDATE chotluong SET trangthai = 'mo', xuatluc = NULL WHERE chotid = $1",
        &[&id],
    )
    .await
    .map_err(|e| AppError::msg(e.to_string()))?;
    tx.commit()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    Ok(json!(true))
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn build_ck_csv(dong: &[Map<String, Value>], chitiet: &str) -> String {
    let mut out = String::from(
        "STT,So tai khoan,Ten thu huong,Ngan hang,So tien,Chi tiet thanh toan\n",
    );
    for (i, d) in dong.iter().enumerate() {
        let stk = d.get("stk").and_then(|v| v.as_str()).unwrap_or("");
        let ten = d.get("ten").and_then(|v| v.as_str()).unwrap_or("");
        let nh = d.get("nganhang").and_then(|v| v.as_str()).unwrap_or("");
        let so = d.get("sotien").and_then(|v| v.as_i64()).unwrap_or(0);
        out.push_str(&format!(
            "{},{},{},{},{},{}\n",
            i + 1,
            csv_escape(stk),
            csv_escape(ten),
            csv_escape(nh),
            so,
            csv_escape(chitiet)
        ));
    }
    out
}

fn ascii_upper_unaccent(s: &str) -> String {
    // Minimal: uppercase + strip common VN tones via unicode NFKD-ish manual map is heavy;
    // subset: uppercase ASCII-safe chars, keep letters.
    let mut o = String::new();
    for c in s.chars() {
        let u = match c {
            'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ'
            | 'ặ' | 'ẳ' | 'ẵ' | 'À' | 'Á' | 'Ạ' | 'Ả' | 'Ã' | 'Â' | 'Ầ' | 'Ấ' | 'Ậ' | 'Ẩ' | 'Ẫ'
            | 'Ă' | 'Ằ' | 'Ắ' | 'Ặ' | 'Ẳ' | 'Ẵ' => 'A',
            'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' | 'È' | 'É' | 'Ẹ'
            | 'Ẻ' | 'Ẽ' | 'Ê' | 'Ề' | 'Ế' | 'Ệ' | 'Ể' | 'Ễ' => 'E',
            'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' | 'Ì' | 'Í' | 'Ị' | 'Ỉ' | 'Ĩ' => 'I',
            'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ'
            | 'ợ' | 'ở' | 'ỡ' | 'Ò' | 'Ó' | 'Ọ' | 'Ỏ' | 'Õ' | 'Ô' | 'Ồ' | 'Ố' | 'Ộ' | 'Ổ' | 'Ỗ'
            | 'Ơ' | 'Ờ' | 'Ớ' | 'Ợ' | 'Ở' | 'Ỡ' => 'O',
            'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' | 'Ù' | 'Ú' | 'Ụ'
            | 'Ủ' | 'Ũ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ự' | 'Ử' | 'Ữ' => 'U',
            'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' | 'Ỳ' | 'Ý' | 'Ỵ' | 'Ỷ' | 'Ỹ' => 'Y',
            'đ' | 'Đ' => 'D',
            _ => c.to_ascii_uppercase(),
        };
        o.push(u);
    }
    o.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn chi_tiet_mac_dinh(thang: &str) -> String {
    format!(
        "CTY TNHH GIAO DUC 59 thanh toan luong T{}.{}",
        &thang[0..2],
        &thang[5..7]
    )
}

async fn ds_chot_ck(
    client: &Object,
    thang: &str,
) -> Result<Vec<Map<String, Value>>, AppError> {
    let rows = query_json(
        client,
        "SELECT c.chotid, c.giaovienid, g.hotengv, c.thucnhan::bigint AS thucnhan,
                c.thoigian::text AS chotluc, c.xuatluc::text AS xuatluc,
                COALESCE(NULLIF(TRIM(g.stk), ''), c.stk) AS stk,
                COALESCE(NULLIF(TRIM(g.nganhang), ''), c.nganhang) AS nganhang,
                COALESCE(NULLIF(TRIM(g.chutk), ''), NULLIF(TRIM(c.chutk), ''),
                         NULLIF(TRIM(g.hotendaydu), '')) AS chutk,
                d.loid, d.sotien::bigint AS dachuyen, o.thoigian::text AS xuatckluc
           FROM chotluong c
           JOIN giaovien g ON g.giaovienid = c.giaovienid
           LEFT JOIN chuyenluong_dong d ON d.chotid = c.chotid
             AND EXISTS (SELECT 1 FROM chuyenluong_lo o2 WHERE o2.loid = d.loid AND o2.huyluc IS NULL)
           LEFT JOIN chuyenluong_lo o ON o.loid = d.loid
          WHERE c.thang = $1 AND c.trangthai = 'chot'
          ORDER BY g.hotengv",
        &[&thang],
    )
    .await
    .map_err(map_db)?;
    let mut out = Vec::new();
    for mut r in rows {
        let stk = r
            .get("stk")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let chutk = r
            .get("chutk")
            .and_then(|v| v.as_str())
            .map(|s| ascii_upper_unaccent(s));
        let nganhang = r
            .get("nganhang")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let thucnhan = r.get("thucnhan").and_then(|v| v.as_i64()).unwrap_or(0);
        let loi = if stk.is_empty() {
            Some("Chưa có số tài khoản")
        } else if !stk
            .chars()
            .all(|c| c.is_ascii_alphanumeric())
            || stk.len() > 24
        {
            Some("Số tài khoản có ký tự lạ (khoảng trắng / dấu) – sửa ở hồ sơ giáo viên")
        } else if nganhang.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            Some("Chưa có ngân hàng")
        } else if chutk.as_ref().map(|s| s.is_empty()).unwrap_or(true) {
            Some("Chưa có tên chủ tài khoản")
        } else if thucnhan <= 0 {
            Some("Thực lãnh bằng 0")
        } else {
            None
        };
        r.insert("ten".into(), json!(chutk));
        r.insert("loi".into(), json!(loi));
        out.push(r);
    }
    Ok(out)
}

pub async fn bl_ck_ds(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::NHAN_VIEN)?;
    let thang = arg_str(args, "thang");
    if !month_key_ok(&thang) {
        return Err(AppError::msg("Tháng không hợp lệ (MM-YYYY)."));
    }
    let gv = ds_chot_ck(client, &thang).await?;
    let chuachot = query_one_json(
        client,
        "SELECT COUNT(*)::int AS n FROM giaovien g
          WHERE EXISTS (SELECT 1 FROM lophoc l WHERE l.giaovienid = g.giaovienid)
            AND NOT EXISTS (
              SELECT 1 FROM chotluong c
               WHERE c.giaovienid = g.giaovienid AND c.thang = $1 AND c.trangthai = 'chot')",
        &[&thang],
    )
    .await
    .map_err(map_db)?
    .and_then(|m| m.get("n").cloned())
    .unwrap_or(json!(0));
    let lo = query_json(
        client,
        "SELECT o.loid, o.thoigian::text AS thoigian, o.soluong, o.tongtien,
                o.huyluc::text AS huyluc, o.huylydo, o.chitiet,
                COALESCE(NULLIF(TRIM(n.hoten), ''), n.hotennv) AS nguoi,
                COALESCE(NULLIF(TRIM(h.hoten), ''), h.hotennv) AS nguoihuy
           FROM chuyenluong_lo o
           LEFT JOIN nhanvien n ON n.nhanvienid = o.nhanvienid
           LEFT JOIN nhanvien h ON h.nhanvienid = o.huynv
          WHERE o.thang = $1
          ORDER BY o.loid DESC",
        &[&thang],
    )
    .await
    .map_err(map_db)?;
    Ok(json!({
        "thang": thang,
        "gv": gv,
        "chuachot": chuachot,
        "lo": lo,
        "chitiet": chi_tiet_mac_dinh(&thang),
        "subset": true,
        "note": "S4b: xuất CSV (xlsx mẫu MB port sau).",
    }))
}

pub async fn bl_ck_xuat(
    client: &mut Object,
    ctx: &Ctx,
    args: &Args,
) -> Result<Value, AppError> {
    need(ctx, role::NHAN_VIEN)?;
    let thang = arg_str(args, "thang");
    if !month_key_ok(&thang) {
        return Err(AppError::msg("Tháng không hợp lệ (MM-YYYY)."));
    }
    let ids: Vec<i32> = args
        .get("chotids")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| {
                    x.as_i64()
                        .or_else(|| x.as_f64().map(|f| f as i64))
                        .map(|n| n as i32)
                })
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        return Err(AppError::msg("Chọn giáo viên cần chuyển khoản."));
    }
    let mut ct = arg_str(args, "chitiet");
    if ct.is_empty() {
        ct = chi_tiet_mac_dinh(&thang);
    }
    ct = ascii_upper_unaccent(&ct);
    if ct.len() > 69 {
        ct = ct.chars().take(69).collect();
    }
    if ct.is_empty() {
        return Err(AppError::msg("Ghi chi tiết thanh toán."));
    }
    let nvid = nv_id(ctx);
    let ds_all = ds_chot_ck(client, &thang).await?;
    let ds: Vec<_> = ds_all
        .into_iter()
        .filter(|r| {
            r.get("chotid")
                .and_then(|v| v.as_i64())
                .map(|n| n as i32)
                .map(|id| ids.contains(&id))
                .unwrap_or(false)
        })
        .collect();
    if ds.len() != {
        let mut s = ids.clone();
        s.sort_unstable();
        s.dedup();
        s.len()
    } {
        return Err(AppError::msg(
            "Có giáo viên vừa bị mở lại bảng lương. Bấm Tải lại.",
        ));
    }
    for r in &ds {
        if r.get("loid").map(|v| !v.is_null()).unwrap_or(false) {
            return Err(AppError::msg(format!(
                "{} đã xuất chuyển khoản ở lô {}. Bấm Tải lại.",
                r.get("hotengv")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GV"),
                r.get("loid").and_then(|v| v.as_i64()).unwrap_or(0)
            )));
        }
        if let Some(loi) = r.get("loi").and_then(|v| v.as_str()) {
            return Err(AppError::msg(format!(
                "{}: {loi}.",
                r.get("hotengv")
                    .and_then(|v| v.as_str())
                    .unwrap_or("GV")
            )));
        }
    }
    let tong: i64 = ds
        .iter()
        .map(|r| r.get("thucnhan").and_then(|v| v.as_i64()).unwrap_or(0))
        .sum();
    let soluong = ds.len() as i32;

    let tx = client
        .transaction()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    tx.execute("SELECT pg_advisory_xact_lock(59002, 0)", &[])
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    let loid: i32 = tx
        .query_one(
            "INSERT INTO chuyenluong_lo (thang, thoigian, nhanvienid, soluong, tongtien, chitiet)
             VALUES ($1, date_trunc('second', now()), $2, $3, $4, $5) RETURNING loid",
            &[&thang, &nvid, &soluong, &tong, &ct],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?
        .get(0);

    let mut dong_out = Vec::new();
    for (i, r) in ds.iter().enumerate() {
        let chotid = r.get("chotid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let gvid = r.get("giaovienid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let stk = r.get("stk").and_then(|v| v.as_str()).unwrap_or("");
        let ten = r.get("ten").and_then(|v| v.as_str()).unwrap_or("");
        let nh = r.get("nganhang").and_then(|v| v.as_str()).unwrap_or("");
        let so = r.get("thucnhan").and_then(|v| v.as_i64()).unwrap_or(0);
        let stt = (i + 1) as i32;
        tx.execute(
            "INSERT INTO chuyenluong_dong (loid, stt, chotid, giaovienid, stk, ten, nganhang, sotien)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            &[&loid, &stt, &chotid, &gvid, &stk, &ten, &nh, &so],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
        dong_out.push(Map::from_iter([
            ("stk".into(), json!(stk)),
            ("ten".into(), json!(ten)),
            ("nganhang".into(), json!(nh)),
            ("sotien".into(), json!(so)),
        ]));
    }
    tx.commit()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;

    let csv = build_ck_csv(&dong_out, &ct);
    Ok(json!({
        "loid": loid,
        "soluong": soluong,
        "tong": tong,
        "ten": format!("Chuyen luong T{}.{} - lo {loid}.csv", &thang[0..2], &thang[3..7]),
        "csv": csv,
        "dong": dong_out,
        "subset": true,
        "note": "CSV thay xlsx MB — kế toán có thể import / copy.",
    }))
}

pub async fn bl_ck_tai(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::NHAN_VIEN)?;
    let loid = arg_i32(args, "loid").unwrap_or(0);
    if loid <= 0 {
        return Err(AppError::msg("Thiếu loid."));
    }
    let lo = query_one_json(
        client,
        "SELECT loid, thang, tongtien, chitiet, soluong FROM chuyenluong_lo WHERE loid = $1",
        &[&loid],
    )
    .await
    .map_err(map_db)?
    .ok_or_else(|| AppError::msg("Không tìm thấy lô chuyển khoản."))?;
    let dong = query_json(
        client,
        "SELECT stk, ten, nganhang, sotien::bigint AS sotien
           FROM chuyenluong_dong WHERE loid = $1 ORDER BY stt",
        &[&loid],
    )
    .await
    .map_err(map_db)?;
    let thang = lo
        .get("thang")
        .and_then(|v| v.as_str())
        .unwrap_or("01-2026");
    let ct = lo
        .get("chitiet")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let csv = build_ck_csv(&dong, ct);
    Ok(json!({
        "loid": loid,
        "soluong": dong.len(),
        "tong": lo.get("tongtien"),
        "ten": format!("Chuyen luong T{}.{} - lo {loid}.csv", &thang[0..2], &thang[3..7]),
        "csv": csv,
        "dong": dong,
        "subset": true,
    }))
}

pub async fn bl_ck_huy(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::ADMIN)?;
    let loid = arg_i32(args, "loid").unwrap_or(0);
    let lydo = arg_str(args, "lydo");
    if loid <= 0 {
        return Err(AppError::msg("Thiếu loid."));
    }
    if lydo.is_empty() {
        return Err(AppError::msg("Ghi lý do huỷ lô."));
    }
    let nvid = nv_id(ctx);
    let n = client
        .execute(
            "UPDATE chuyenluong_lo SET huyluc = date_trunc('second', now()), huynv = $2, huylydo = $3
              WHERE loid = $1 AND huyluc IS NULL",
            &[&loid, &nvid, &lydo],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    if n == 0 {
        return Err(AppError::msg("Lô này không còn hiệu lực."));
    }
    Ok(json!(true))
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
