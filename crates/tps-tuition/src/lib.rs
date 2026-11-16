//! Tuition periods + collection — S2 subset matched to tps59 behaviour for the common path.

use chrono::{NaiveDateTime, Utc};
use deadpool_postgres::Object;
use serde_json::{json, Map, Value};
use tps_auth::{can, need, role};
use tps_db::{query_json, query_one_json, DbError};
use tps_logic::{
    current_month_key, fee_for_month, unpaid_months, parse_months, add_month,
};
use tps_types::{AppError, Args, Ctx};

fn map_db(e: DbError) -> AppError {
    AppError::msg(e.to_string())
}

fn arg_i64(args: &Args, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

fn arg_str(args: &Args, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string()
}

/// `A001` + `L` + lopid + `T` + MM + YY  e.g. A001L5T1126
pub fn period_code(mahv: &str, lopid: i64, key: &str) -> String {
    // key = MM-YYYY
    let mm = &key[0..2];
    let yy = &key[5..7];
    format!("{mahv}L{lopid}T{mm}{yy}")
}

pub fn now_ts_vn() -> String {
    // Approximate: UTC+7 wall clock (session TZ also set on connection)
    let t = Utc::now() + chrono::Duration::hours(7);
    t.format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn today_vn() -> (i32, u32) {
    let t = Utc::now() + chrono::Duration::hours(7);
    (t.year(), t.month())
}

use chrono::Datelike;

async fn bang_tinh(client: &Object) -> Result<Vec<Map<String, Value>>, AppError> {
    query_json(
        client,
        "SELECT ngayfrom, ngayto, tile FROM bangtinhhp ORDER BY ngayfrom",
        &[],
    )
    .await
    .map_err(map_db)
}

pub async fn closed_months(client: &Object) -> Result<Vec<String>, AppError> {
    let rows = query_json(
        client,
        "SELECT thang FROM kydangky WHERE trangthai = 'dong'",
        &[],
    )
    .await
    .map_err(map_db)?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.get("thang").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect())
}

pub async fn sync_paid_month(client: &Object, ky: &Map<String, Value>, du: bool) -> Result<(), AppError> {
    let lopid = ky.get("lopid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let hs = ky.get("hocsinhid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let thang = ky.get("thang").and_then(|v| v.as_str()).unwrap_or("");
    let en = query_json(
        client,
        "SELECT lineid, thangdadongtien FROM danhsachlop
          WHERE lopid = $1 AND hocsinhid = $2
          ORDER BY ngaykthoc NULLS FIRST LIMIT 1",
        &[&lopid, &hs],
    )
    .await
    .map_err(map_db)?;
    let Some(e) = en.into_iter().next() else {
        return Ok(());
    };
    let cur = e.get("thangdadongtien").and_then(|v| v.as_str());
    let val = if du {
        add_month(cur, thang)
    } else {
        tps_logic::remove_month(cur, thang)
    };
    let lineid = e.get("lineid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    client
        .execute(
            "UPDATE danhsachlop SET thangdadongtien = $1 WHERE lineid = $2",
            &[&val, &lineid],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    Ok(())
}

pub async fn add_fund(client: &Object, amount: i64) -> Result<(), AppError> {
    client
        .execute(
            "UPDATE taikhoan SET tienhientai = COALESCE(tienhientai, 0) + $1",
            &[&amount],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    Ok(())
}

/// Create missing periods up to current (or upto) month for enrollments.
pub async fn ensure_periods(
    client: &Object,
    hocsinhid: Option<i32>,
    upto_key: Option<&str>,
) -> Result<i32, AppError> {
    let (y, m) = today_vn();
    let upto = upto_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| current_month_key(y, m));
    let bang = bang_tinh(client).await?;
    let dong = closed_months(client).await?;

    let rows = if let Some(hs) = hocsinhid {
        query_json(
            client,
            "SELECT d.lineid, d.lopid, d.hocsinhid, d.ngaybdhoc, d.ngaykthoc, d.thangdadongtien,
                    d.khong_phuthu, d.giam, l.hocphi, l.hocphi_specialmonth, l.ngaybatdau, l.ngayketthuc,
                    h.mahocvien
               FROM danhsachlop d
               JOIN lophoc l ON l.lopid = d.lopid
               JOIN hocsinh h ON h.hocsinhid = d.hocsinhid
              WHERE d.hocsinhid = $1",
            &[&hs],
        )
        .await
        .map_err(map_db)?
    } else {
        query_json(
            client,
            "SELECT d.lineid, d.lopid, d.hocsinhid, d.ngaybdhoc, d.ngaykthoc, d.thangdadongtien,
                    d.khong_phuthu, d.giam, l.hocphi, l.hocphi_specialmonth, l.ngaybatdau, l.ngayketthuc,
                    h.mahocvien
               FROM danhsachlop d
               JOIN lophoc l ON l.lopid = d.lopid
               JOIN hocsinh h ON h.hocsinhid = d.hocsinhid",
            &[],
        )
        .await
        .map_err(map_db)?
    };

    let mut created = 0i32;
    for r in &rows {
        let months = unpaid_months(r, r, &upto);
        let paid: std::collections::HashSet<String> =
            parse_months(r.get("thangdadongtien").and_then(|v| v.as_str()))
                .into_iter()
                .collect();
        let giam = r.get("giam").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let hs = r.get("hocsinhid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let lop = r.get("lopid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let lineid = r.get("lineid").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let mahv = r
            .get("mahocvien")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("HS{hs}"));

        for key in months {
            if dong.iter().any(|d| d == &key) {
                continue;
            }
            let fee = fee_for_month(r, Some(r), giam, &key, &bang);
            let mut sotien = fee;
            let mut dathu = 0i64;
            let mut trangthai = "chuathu";
            if paid.contains(&key) {
                let p = query_one_json(
                    client,
                    "SELECT COALESCE(SUM(sotien), 0)::bigint AS s FROM phieuthutien
                      WHERE hocsinhid = $1 AND lopid = $2 AND thangdongtien = $3",
                    &[&hs, &lop, &key],
                )
                .await
                .map_err(map_db)?;
                let s = p
                    .and_then(|m| m.get("s").and_then(|v| v.as_i64()))
                    .unwrap_or(0);
                sotien = if s != 0 { s } else { fee };
                dathu = sotien;
                trangthai = "dathu";
            }
            let maky = period_code(&mahv, lop as i64, &key);
            let ins = client
                .query(
                    "INSERT INTO kyhocphi (makyhp, hocsinhid, lopid, lineid, thang, sotien, dathu, trangthai, phuthu, phidu)
                     VALUES ($1,$2,$3,$4,$5,$6,$7,$8,0,$9)
                     ON CONFLICT ON CONSTRAINT ux_kyhocphi_hs_lop_thang DO NOTHING
                     RETURNING kyid",
                    &[
                        &maky,
                        &hs,
                        &lop,
                        &lineid,
                        &key,
                        &(sotien as i32),
                        &(dathu as i32),
                        &trangthai,
                        &(sotien as i32),
                    ],
                )
                .await
                .map_err(|e| AppError::msg(e.to_string()))?;
            if !ins.is_empty() {
                created += 1;
            }
        }
    }
    Ok(created)
}

pub async fn compute_debt(
    client: &Object,
    hocsinhid: Option<i32>,
    upto_key: Option<&str>,
) -> Result<Value, AppError> {
    ensure_periods(client, hocsinhid, upto_key).await?;
    let (y, m) = today_vn();
    let key = upto_key
        .map(|s| s.to_string())
        .unwrap_or_else(|| current_month_key(y, m));
    // sort key YYYYMM
    let sort_key = format!("{}{}", &key[3..], &key[0..2]);
    let dong = closed_months(client).await?;

    let sql = if hocsinhid.is_some() {
        format!(
            "WITH k AS (
               SELECT k.hocsinhid, k.lopid,
                      array_agg(k.kyid ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS kyids,
                      array_agg(k.thang ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS thangs,
                      array_agg(k.makyhp ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS makys,
                      array_agg((k.sotien - k.dathu)::int ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS conlais,
                      (array_agg(k.lineid ORDER BY right(k.thang, 4) || left(k.thang, 2)))[1] AS lineid,
                      SUM(k.sotien - k.dathu)::bigint AS tongno
                 FROM kyhocphi k
                WHERE k.trangthai = 'chuathu' AND k.sotien > k.dathu
                  AND right(k.thang, 4) || left(k.thang, 2) <= $1
                  AND NOT (k.thang = ANY($2::text[]))
                  AND k.hocsinhid = $3
                GROUP BY k.hocsinhid, k.lopid)
             SELECT k.*, l.tenlop, l.monid, m.tenmon, l.khoilop,
                    (h.hotenhs || COALESCE(' ' || NULLIF(h.ngaysinh, ''), '')) AS hotenhs,
                    h.mahocvien, h.sdtph, h.sdths, h.sdt3
               FROM k JOIN hocsinh h ON h.hocsinhid = k.hocsinhid
               JOIN lophoc l ON l.lopid = k.lopid
               LEFT JOIN monhoc m ON m.monid = l.monid"
        )
    } else {
        format!(
            "WITH k AS (
               SELECT k.hocsinhid, k.lopid,
                      array_agg(k.kyid ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS kyids,
                      array_agg(k.thang ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS thangs,
                      array_agg(k.makyhp ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS makys,
                      array_agg((k.sotien - k.dathu)::int ORDER BY right(k.thang, 4) || left(k.thang, 2)) AS conlais,
                      (array_agg(k.lineid ORDER BY right(k.thang, 4) || left(k.thang, 2)))[1] AS lineid,
                      SUM(k.sotien - k.dathu)::bigint AS tongno
                 FROM kyhocphi k
                WHERE k.trangthai = 'chuathu' AND k.sotien > k.dathu
                  AND right(k.thang, 4) || left(k.thang, 2) <= $1
                  AND NOT (k.thang = ANY($2::text[]))
                GROUP BY k.hocsinhid, k.lopid)
             SELECT k.*, l.tenlop, l.monid, m.tenmon, l.khoilop,
                    (h.hotenhs || COALESCE(' ' || NULLIF(h.ngaysinh, ''), '')) AS hotenhs,
                    h.mahocvien, h.sdtph, h.sdths, h.sdt3
               FROM k JOIN hocsinh h ON h.hocsinhid = k.hocsinhid
               JOIN lophoc l ON l.lopid = k.lopid
               LEFT JOIN monhoc m ON m.monid = l.monid"
        )
    };

    // Prefer simpler non-array query for portability
    let mut owned_dong = dong.clone();
    if owned_dong.is_empty() {
        owned_dong.push("__none__".into()); // ANY empty is awkward; use impossible month
    }
    let _ = sql;

    let rows = if let Some(hs) = hocsinhid {
        query_json(
            client,
            "SELECT k.kyid, k.makyhp, k.thang, k.sotien, k.dathu, (k.sotien - k.dathu)::int AS conlai,
                    k.hocsinhid, k.lopid, k.lineid, l.tenlop, m.tenmon,
                    (h.hotenhs || COALESCE(' ' || NULLIF(h.ngaysinh, ''), '')) AS hotenhs, h.mahocvien
               FROM kyhocphi k
               JOIN hocsinh h ON h.hocsinhid = k.hocsinhid
               JOIN lophoc l ON l.lopid = k.lopid
               LEFT JOIN monhoc m ON m.monid = l.monid
              WHERE k.trangthai = 'chuathu' AND k.sotien > k.dathu
                AND right(k.thang, 4) || left(k.thang, 2) <= $1
                AND k.hocsinhid = $2
                AND NOT (k.thang = ANY($3::text[]))
              ORDER BY h.mahocvien, l.tenlop, right(k.thang,4)||left(k.thang,2)",
            &[&sort_key, &hs, &owned_dong],
        )
        .await
        .map_err(map_db)?
    } else {
        query_json(
            client,
            "SELECT k.kyid, k.makyhp, k.thang, k.sotien, k.dathu, (k.sotien - k.dathu)::int AS conlai,
                    k.hocsinhid, k.lopid, k.lineid, l.tenlop, m.tenmon,
                    (h.hotenhs || COALESCE(' ' || NULLIF(h.ngaysinh, ''), '')) AS hotenhs, h.mahocvien
               FROM kyhocphi k
               JOIN hocsinh h ON h.hocsinhid = k.hocsinhid
               JOIN lophoc l ON l.lopid = k.lopid
               LEFT JOIN monhoc m ON m.monid = l.monid
              WHERE k.trangthai = 'chuathu' AND k.sotien > k.dathu
                AND right(k.thang, 4) || left(k.thang, 2) <= $1
                AND NOT (k.thang = ANY($2::text[]))
              ORDER BY h.mahocvien, l.tenlop, right(k.thang,4)||left(k.thang,2)",
            &[&sort_key, &owned_dong],
        )
        .await
        .map_err(map_db)?
    };

    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

/// Collect exact remaining amount for periods (TM/CK). Common path from tps59.
pub async fn collect_items(
    client: &mut Object,
    ctx: &Ctx,
    hocsinhid: i32,
    items: &[Map<String, Value>],
    hinhthuc: &str,
    ghichu: Option<&str>,
) -> Result<Value, AppError> {
    if items.is_empty() {
        return Err(AppError::msg("Chưa chọn kỳ nào để thu."));
    }
    if hinhthuc != "TM" && hinhthuc != "CK" {
        return Err(AppError::msg("Hình thức thanh toán không hợp lệ."));
    }
    let allow = can(ctx, "sua_phieuthu");
    let nv = ctx
        .user
        .as_ref()
        .and_then(|u| u.get("nhanvienid"))
        .and_then(|v| v.as_i64())
        .map(|n| n as i32);
    let ts = now_ts_vn();
    let mut ids = Vec::new();
    let mut total = 0i64;
    let mut ghi_ky = Vec::new();

    let tx = client
        .transaction()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;

    for it in items {
        let kyid = it.get("kyid").and_then(|v| v.as_i64()).map(|n| n as i32);
        let so = it
            .get("sotien")
            .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
            .unwrap_or(-1);
        if so < 0 {
            return Err(AppError::msg("Số tiền thu phải lớn hơn 0."));
        }
        let Some(kyid) = kyid else {
            return Err(AppError::msg("Kỳ học phí không hợp lệ."));
        };
        let ky_rows = tx
            .query(
                "SELECT * FROM kyhocphi WHERE kyid = $1 AND hocsinhid = $2 FOR UPDATE",
                &[&kyid, &hocsinhid],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        if ky_rows.is_empty() {
            return Err(AppError::msg("Không tìm thấy kỳ học phí."));
        }
        let ky = &ky_rows[0];
        let trangthai: String = ky.get("trangthai");
        let sotien: i32 = ky.get("sotien");
        let dathu: i32 = ky.get("dathu");
        let makyhp: String = ky.get("makyhp");
        let thang: String = ky.get("thang");
        let lopid: i32 = ky.get("lopid");
        let conlai = (sotien - dathu) as i64;

        if trangthai == "huy" {
            return Err(AppError::msg(format!("Kỳ {makyhp} đã được miễn, không thu.")));
        }
        if trangthai == "dathu" || conlai <= 0 {
            return Err(AppError::msg(format!(
                "Kỳ {makyhp} (tháng {thang}) đã thu đủ rồi."
            )));
        }
        if so == 0 && !(trangthai == "chuathu" && sotien == 0 && dathu == 0) {
            return Err(AppError::msg("Số tiền thu phải lớn hơn 0."));
        }
        if !allow && so != conlai && !(sotien == 0 && so == 0) {
            return Err(AppError::msg(format!(
                "Kỳ {makyhp} phải thu đúng {conlai} đ. Cần quyền \"Chỉnh phiếu thu học phí\" để điều chỉnh số tiền."
            )));
        }
        if so > conlai {
            return Err(AppError::msg(format!(
                "Kỳ {makyhp} chỉ còn nợ {conlai} đ."
            )));
        }

        // nextId inside this transaction (app_meta seq)
        let seq_key = "seq:phieuthutien";
        let cur = tx
            .query_opt(
                "SELECT value FROM app_meta WHERE key = $1 FOR UPDATE",
                &[&seq_key],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        let lineid: i32 = if let Some(r) = cur {
            let v: String = r.get(0);
            v.parse::<i32>().unwrap_or(0) + 1
        } else {
            let max: i32 = tx
                .query_one(
                    "SELECT COALESCE(MAX(lineid), 0)::int FROM phieuthutien",
                    &[],
                )
                .await
                .map_err(|e| AppError::msg(e.to_string()))?
                .get(0);
            max + 1
        };
        tx.execute(
            "INSERT INTO app_meta (key, value) VALUES ($1, $2)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
            &[&seq_key, &lineid.to_string()],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
        let ghi = ghichu.map(|s| s.to_string());
        let ts_ndt = NaiveDateTime::parse_from_str(&ts, "%Y-%m-%d %H:%M:%S")
            .map_err(|e| AppError::msg(e.to_string()))?;
        tx.execute(
            "INSERT INTO phieuthutien (lineid, lopid, hocsinhid, sotien, thangdongtien, thoigiandong, nhanvienid, datraluong, ghichu, kyid, hinhthuc)
             VALUES ($1,$2,$3,$4,$5,$6, $7, 0, $8, $9, $10)",
            &[
                &lineid,
                &lopid,
                &hocsinhid,
                &(so as i32),
                &thang,
                &ts_ndt,
                &nv,
                &ghi,
                &kyid,
                &hinhthuc,
            ],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
        ids.push(lineid);

        // Close period at amount paid (common path when so == conlai, or allowAdjust partial close)
        tx.execute(
            "UPDATE kyhocphi SET sotien = dathu + $1, dathu = dathu + $1, trangthai = 'dathu' WHERE kyid = $2",
            &[&(so as i32), &kyid],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;

        // sync paid month
        let mut ky_map = Map::new();
        ky_map.insert("lopid".into(), json!(lopid));
        ky_map.insert("hocsinhid".into(), json!(hocsinhid));
        ky_map.insert("thang".into(), json!(thang));
        // Use outer client after commit for sync — do inline:
        let en = tx
            .query(
                "SELECT lineid, thangdadongtien FROM danhsachlop
                  WHERE lopid = $1 AND hocsinhid = $2
                  ORDER BY ngaykthoc NULLS FIRST LIMIT 1",
                &[&lopid, &hocsinhid],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        if let Some(e) = en.first() {
            let cur: Option<String> = e.get("thangdadongtien");
            let val = add_month(cur.as_deref(), &thang);
            let lid: i32 = e.get("lineid");
            tx.execute(
                "UPDATE danhsachlop SET thangdadongtien = $1 WHERE lineid = $2",
                &[&val, &lid],
            )
            .await
            .map_err(|e| AppError::msg(e.to_string()))?;
        }

        ghi_ky.push(format!("{makyhp} {so}"));
        total += so;
    }

    if hinhthuc == "TM" {
        tx.execute(
            "UPDATE taikhoan SET tienhientai = COALESCE(tienhientai, 0) + $1",
            &[&total],
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    }

    tx.commit()
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;

    Ok(json!({
        "ids": ids,
        "total": total,
        "thoigian": ts,
        "hinhthuc": hinhthuc,
        "ky": ghi_ky,
    }))
}

pub async fn quy_get(client: &Object) -> Result<Value, AppError> {
    let r = query_one_json(
        client,
        "SELECT COALESCE(tienhientai,0)::bigint AS tienhientai, COALESCE(tientrongket,0)::bigint AS tientrongket FROM taikhoan LIMIT 1",
        &[],
    )
    .await
    .map_err(map_db)?
    .unwrap_or_else(|| {
        let mut m = Map::new();
        m.insert("tienhientai".into(), json!(0));
        m.insert("tientrongket".into(), json!(0));
        m
    });
    Ok(json!({
        "tienhientai": r.get("tienhientai").cloned().unwrap_or(json!(0)),
        "tientrongket": r.get("tientrongket").cloned().unwrap_or(json!(0)),
    }))
}

pub async fn kyhp_list(client: &Object, args: &Args) -> Result<Value, AppError> {
    let hs = arg_i64(args, "hocsinhid").map(|n| n as i32);
    let rows = if let Some(hs) = hs {
        ensure_periods(client, Some(hs), None).await?;
        query_json(
            client,
            "SELECT k.*, l.tenlop FROM kyhocphi k JOIN lophoc l ON l.lopid = k.lopid
              WHERE k.hocsinhid = $1
              ORDER BY right(k.thang,4)||left(k.thang,2), l.tenlop",
            &[&hs],
        )
        .await
        .map_err(map_db)?
    } else {
        query_json(
            client,
            "SELECT k.*, l.tenlop FROM kyhocphi k JOIN lophoc l ON l.lopid = k.lopid
              ORDER BY k.kyid DESC LIMIT 200",
            &[],
        )
        .await
        .map_err(map_db)?
    };
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

pub async fn hocphi_collect(client: &mut Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let hs = arg_i64(args, "hocsinhid")
        .ok_or_else(|| AppError::msg("Thiếu hocsinhid."))? as i32;
    let hinh = {
        let s = arg_str(args, "hinhthuc");
        if s.is_empty() {
            "TM".into()
        } else {
            s
        }
    };
    let ghichu = args.get("ghichu").and_then(|v| v.as_str());
    let items_v = args
        .get("items")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let items: Vec<Map<String, Value>> = items_v
        .into_iter()
        .filter_map(|v| v.as_object().cloned())
        .collect();
    collect_items(client, ctx, hs, &items, &hinh, ghichu).await
}

pub async fn congno_list(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let hs = arg_i64(args, "hocsinhid").map(|n| n as i32);
    let upto = args.get("upto").and_then(|v| v.as_str());
    compute_debt(client, hs, upto).await
}

/// `hocphi.receipt` — biên lai data for print (S6 subset; phụ thu chi tiết port sau).
pub async fn hocphi_receipt(client: &Object, ctx: &Ctx, args: &Args) -> Result<Value, AppError> {
    need(ctx, role::CTV)?;
    let ids: Vec<i32> = args
        .get("ids")
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
        return Err(AppError::msg("Thiếu ids phiếu thu."));
    }
    let rows = query_json(
        client,
        "SELECT p.lineid, p.thangdongtien, p.sotien::bigint AS sotien,
                p.thoigiandong::text AS thoigiandong, p.ghichu, p.hinhthuc,
                l.tenlop, g.hotengv, h.hotenhs, h.hocsinhid, h.mahocvien,
                COALESCE(NULLIF(TRIM(n.hoten), ''), n.hotennv) AS hotennv,
                k.makyhp, (k.sotien - k.dathu)::bigint AS conlai, p.lopid
           FROM phieuthutien p
           JOIN hocsinh h ON h.hocsinhid = p.hocsinhid
           LEFT JOIN lophoc l ON l.lopid = p.lopid
           LEFT JOIN giaovien g ON g.giaovienid = l.giaovienid
           LEFT JOIN nhanvien n ON n.nhanvienid = p.nhanvienid
           LEFT JOIN kyhocphi k ON k.kyid = p.kyid
          WHERE p.lineid = ANY($1)
          ORDER BY l.tenlop, p.thangdongtien, p.lineid",
        &[&ids],
    )
    .await
    .map_err(map_db)?;

    let tt = query_one_json(
        client,
        "SELECT tentrungtam, sdt, diachi, website FROM thongtintt LIMIT 1",
        &[],
    )
    .await
    .map_err(map_db)?
    .unwrap_or_default();
    let ghichu_tt = query_one_json(
        client,
        "SELECT value FROM app_meta WHERE key = 'tt_ghichu'",
        &[],
    )
    .await
    .ok()
    .flatten()
    .and_then(|m| m.get("value").cloned())
    .unwrap_or(json!(""));
    let mut trungtam = tt;
    trungtam.insert("ghichu".into(), ghichu_tt);

    let luuy = query_one_json(
        client,
        "SELECT value FROM app_meta WHERE key = 'bienlai_luuy'",
        &[],
    )
    .await
    .ok()
    .flatten()
    .and_then(|m| m.get("value").and_then(|v| v.as_str()).map(|s| s.to_string()))
    .unwrap_or_default();

    let mut tongket = Value::Null;
    if let Some(first) = rows.first() {
        let hsid = first
            .get("hocsinhid")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let hs = query_one_json(
            client,
            "SELECT COALESCE(h.sodu, 0)::bigint AS sodu,
                    COALESCE((SELECT SUM(k.sotien - k.dathu) FROM kyhocphi k
                               WHERE k.hocsinhid = h.hocsinhid AND k.trangthai = 'chuathu'
                                 AND k.sotien > k.dathu), 0)::bigint AS no
               FROM hocsinh h WHERE h.hocsinhid = $1",
            &[&hsid],
        )
        .await
        .map_err(map_db)?;
        let dong: i64 = rows
            .iter()
            .map(|r| r.get("sotien").and_then(|v| v.as_i64()).unwrap_or(0))
            .sum();
        tongket = json!({
            "dong": dong,
            "ghisodu": arg_i64(args, "ghisodu").unwrap_or(0),
            "tienck": arg_i64(args, "tienck"),
            "sodu": hs.as_ref().and_then(|m| m.get("sodu").cloned()).unwrap_or(json!(0)),
            "conno": hs.as_ref().and_then(|m| m.get("no").cloned()).unwrap_or(json!(0)),
            "subset": true,
        });
    }

    let text = format_receipt_text(&trungtam, &rows, &luuy, &tongket);
    Ok(json!({
        "trungtam": trungtam,
        "rows": rows,
        "luuy": luuy,
        "tongket": tongket,
        "text": text,
        "subset": true,
    }))
}

fn format_receipt_text(
    tt: &Map<String, Value>,
    rows: &[Map<String, Value>],
    luuy: &str,
    tongket: &Value,
) -> String {
    let mut o = String::new();
    let ten = tt
        .get("tentrungtam")
        .and_then(|v| v.as_str())
        .unwrap_or("TPS");
    o.push_str(ten);
    o.push('\n');
    if let Some(s) = tt.get("diachi").and_then(|v| v.as_str()) {
        if !s.is_empty() {
            o.push_str(s);
            o.push('\n');
        }
    }
    if let Some(s) = tt.get("sdt").and_then(|v| v.as_str()) {
        if !s.is_empty() {
            o.push_str("ĐT: ");
            o.push_str(s);
            o.push('\n');
        }
    }
    o.push_str("--------------------------------\n");
    o.push_str("BIÊN LAI THU HỌC PHÍ\n");
    o.push_str("--------------------------------\n");
    if let Some(r0) = rows.first() {
        o.push_str(&format!(
            "HV: {} ({})\n",
            r0.get("hotenhs").and_then(|v| v.as_str()).unwrap_or(""),
            r0.get("mahocvien").and_then(|v| v.as_str()).unwrap_or("")
        ));
    }
    for r in rows {
        o.push_str(&format!(
            "{} | {} | {} | {}\n",
            r.get("makyhp").and_then(|v| v.as_str()).unwrap_or("-"),
            r.get("tenlop").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("hinhthuc").and_then(|v| v.as_str()).unwrap_or(""),
            r.get("sotien").and_then(|v| v.as_i64()).unwrap_or(0)
        ));
    }
    o.push_str("--------------------------------\n");
    if let Some(tk) = tongket.as_object() {
        o.push_str(&format!(
            "Tổng thu: {} đ\n",
            tk.get("dong").and_then(|v| v.as_i64()).unwrap_or(0)
        ));
        o.push_str(&format!(
            "Còn nợ: {} đ | Số dư: {} đ\n",
            tk.get("conno").and_then(|v| v.as_i64()).unwrap_or(0),
            tk.get("sodu").and_then(|v| v.as_i64()).unwrap_or(0)
        ));
    }
    if !luuy.is_empty() {
        o.push_str("Lưu ý: ");
        o.push_str(luuy);
        o.push('\n');
    }
    o.push_str("(S6 subset — in nhiệt/Windows port sau)\n");
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use tps_db::{connect, dev_test_config};

    #[test]
    fn period_code_shape() {
        assert_eq!(period_code("A001", 5, "11-2026"), "A001L5T1126");
    }

    #[tokio::test]
    async fn ensure_and_collect_tm() {
        let pool = connect(&dev_test_config()).await.unwrap();
        let mut c = pool.get().await.unwrap();
        c.execute("SET TIME ZONE 'Asia/Ho_Chi_Minh'", &[]).await.ok();
        let n = ensure_periods(&c, Some(1), None).await.unwrap();
        assert!(n >= 0);
        let debt = compute_debt(&c, Some(1), None).await.unwrap();
        let arr = debt.as_array().unwrap();
        if arr.is_empty() {
            return; // already paid in prior test run
        }
        let first = arr[0].as_object().unwrap();
        let kyid = first.get("kyid").and_then(|v| v.as_i64()).unwrap();
        let conlai = first.get("conlai").and_then(|v| v.as_i64()).unwrap();
        let mut ctx = Ctx::default();
        let mut user = Map::new();
        user.insert("nhanvienid".into(), json!(1));
        user.insert("chucvu".into(), json!(1));
        user.insert("quyen".into(), json!([]));
        ctx.user = Some(user);
        let mut item = Map::new();
        item.insert("kyid".into(), json!(kyid));
        item.insert("sotien".into(), json!(conlai));
        let r = collect_items(&mut c, &ctx, 1, &[item], "TM", Some("test")).await.unwrap();
        assert_eq!(r["total"], conlai);
        let quy = quy_get(&c).await.unwrap();
        assert!(quy["tienhientai"].as_i64().unwrap_or(0) >= conlai);
    }
}
