//! Catalog READ handlers — ported from tps59 catalog.dart (list/get subset for S1).

use deadpool_postgres::Object;
use serde_json::{json, Map, Value};
use tps_auth::{need, role, user_chucvu};
use tps_db::{query_json, query_one_json, DbError};
use tps_logic::fold_vi;
use tps_types::{AppError, Args, Ctx};

const OPEN_LOP: &str = "(l.ngayketthuc IS NULL OR l.ngayketthuc >= CURRENT_DATE)";
const LOP_HD: &str = "(l.hoatdong AND (l.ngayketthuc IS NULL OR l.ngayketthuc >= CURRENT_DATE))";
const HS_TEN: &str = "(h.hotenhs || COALESCE(' ' || NULLIF(h.ngaysinh, ''), ''))";

fn map_db(e: DbError) -> AppError {
    AppError::msg(e.to_string())
}

fn arg_str(args: &Args, key: &str) -> String {
    args.get(key)
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            _ => v.as_str().map(|s| s.to_string()),
        })
        .unwrap_or_default()
}

fn arg_i64(args: &Args, key: &str) -> Option<i64> {
    args.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_f64().map(|f| f as i64))
            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
    })
}

pub async fn monhoc_list(client: &Object) -> Result<Value, AppError> {
    let sql = format!(
        "SELECT m.monid, m.tenmon,
                (SELECT COUNT(*) FROM giaovien g WHERE g.monid = m.monid)::int AS sogv,
                (SELECT COUNT(*) FROM lophoc l WHERE l.monid = m.monid AND {LOP_HD})::int AS solop
           FROM monhoc m ORDER BY m.tenmon"
    );
    let rows = query_json(client, &sql, &[]).await.map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

pub async fn giaovien_list(client: &Object, ctx: &Ctx) -> Result<Value, AppError> {
    let bank = if user_chucvu(ctx) <= role::NHAN_VIEN {
        "g.mst, g.stk, g.nganhang, g.chutk,"
    } else {
        ""
    };
    let sql = format!(
        "SELECT g.giaovienid, g.hotengv, g.hotendaydu, g.danhxung, g.sdtgv, g.diachi, g.monid, m.tenmon,
                g.tilechia, g.trichthue, COALESCE(g.tientaikhoan, 0)::bigint AS tientaikhoan, g.tamdung,
                {bank}
                (SELECT COUNT(*) FROM lophoc l WHERE l.giaovienid = g.giaovienid AND {LOP_HD})::int AS solop
           FROM giaovien g LEFT JOIN monhoc m ON m.monid = g.monid
          ORDER BY g.hotengv"
    );
    let rows = query_json(client, &sql, &[]).await.map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

pub async fn hocsinh_list(client: &Object, args: &Args) -> Result<Value, AppError> {
    let search = arg_str(args, "search");
    let status = {
        let s = arg_str(args, "status");
        if s.is_empty() {
            "all".into()
        } else {
            s
        }
    };
    let mut where_c = Vec::new();
    let mut owned: Vec<String> = Vec::new();

    if !search.trim().is_empty() {
        owned.push(search.trim().to_uppercase());
        owned.push(fold_vi(&search).replace(' ', ""));
        let words: Vec<String> = fold_vi(&search)
            .split(' ')
            .filter(|w| !w.is_empty())
            .map(|w| {
                format!(
                    "%{}%",
                    w.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
                )
            })
            .collect();
        let mut conds = Vec::new();
        for w in &words {
            owned.push(w.clone());
            let p = owned.len(); // 1-based
            conds.push(format!(
                "(tc59_fold(concat_ws(' ', {HS_TEN}, h.mahocvien, h.sdths, h.sdtph, h.sdt3, h.email)) LIKE ${p} OR tc59_initials(h.hotenhs) LIKE ${p})"
            ));
        }
        if !conds.is_empty() {
            where_c.push(format!("({})", conds.join(" AND ")));
        }
    }

    if status == "active" {
        where_c.push("COALESCE(dl.danghoc, false)".into());
    } else if status == "inactive" {
        where_c.push("NOT COALESCE(dl.danghoc, false)".into());
    }

    let where_sql = if where_c.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_c.join(" AND "))
    };

    let order = if !search.trim().is_empty() {
        "(UPPER(h.mahocvien) = $1) DESC, (tc59_initials(h.hotenhs) = $2) DESC, h.hocsinhid DESC"
    } else {
        "h.hocsinhid DESC"
    };

    let limit = arg_i64(args, "limit").filter(|n| *n > 0);
    let limit_sql = limit.map(|n| format!("LIMIT {n}")).unwrap_or_default();

    let sql = format!(
        "WITH dl AS (
           SELECT d.hocsinhid,
                  string_agg(l.tenlop || ' -' || d.giam || '%', ', ' ORDER BY l.tenlop)
                    FILTER (WHERE d.ngaykthoc IS NULL AND d.giam > 0) AS giam,
                  string_agg(l.tenlop, ', ' ORDER BY l.tenlop)
                    FILTER (WHERE d.ngaykthoc IS NULL AND {OPEN_LOP}) AS caclop,
                  bool_or(d.ngaykthoc IS NULL AND {OPEN_LOP}) AS danghoc
             FROM danhsachlop d JOIN lophoc l ON l.lopid = d.lopid GROUP BY d.hocsinhid)
         SELECT h.hocsinhid, h.mahocvien, {HS_TEN} AS hotenhs, h.ngaysinh, h.loptruong,
                h.sdths, h.sdtph, h.sdt3, h.email, h.ghichu, dl.giam, dl.caclop
           FROM hocsinh h LEFT JOIN dl ON dl.hocsinhid = h.hocsinhid
           {where_sql}
          ORDER BY {order} {limit_sql}"
    );

    // Bind owned params as &str refs
    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = owned
        .iter()
        .map(|s| s as &(dyn tokio_postgres::types::ToSql + Sync))
        .collect();
    let rows = query_json(client, &sql, &param_refs)
        .await
        .map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

pub async fn hocsinh_get(client: &Object, args: &Args) -> Result<Value, AppError> {
    let id = arg_i64(args, "hocsinhid").ok_or_else(|| AppError::msg("Thiếu hocsinhid."))?;
    let mut hs = query_one_json(client, "SELECT * FROM hocsinh WHERE hocsinhid = $1", &[&id])
        .await
        .map_err(map_db)?
        .ok_or_else(|| AppError::msg("Không tìm thấy học sinh."))?;

    let hoten = hs.get("hotenhs").and_then(|v| v.as_str()).unwrap_or("");
    let ns = hs.get("ngaysinh").and_then(|v| v.as_str()).unwrap_or("");
    hs.insert(
        "tenhienthi".into(),
        json!(if ns.is_empty() {
            hoten.to_string()
        } else {
            format!("{hoten} {ns}")
        }),
    );

    let lop = query_json(
        client,
        &format!(
            "SELECT d.lineid, d.lopid, l.tenlop, l.monid, l.tkb, l.hocphi, g.hotengv,
                    d.ngaybdhoc, d.ngaykthoc, d.thangdadongtien, d.giam, d.lydogiam, d.giam_tt, d.giam_gv,
                    d.khong_phuthu, d.lydo_khongphuthu
               FROM danhsachlop d JOIN lophoc l ON l.lopid = d.lopid
               LEFT JOIN giaovien g ON g.giaovienid = l.giaovienid
              WHERE d.hocsinhid = $1
              ORDER BY d.ngaykthoc NULLS FIRST, d.ngaybdhoc DESC"
        ),
        &[&id],
    )
    .await
    .map_err(map_db)?;
    hs.insert("lop".into(), Value::Array(lop.into_iter().map(Value::Object).collect()));

    // S1: empty stubs for heavy joins (phieuthu/lichsu filled in finance sprint)
    hs.insert("phieuthu".into(), json!([]));
    hs.insert("lichsu".into(), json!([]));

    let giam = query_json(
        client,
        "SELECT d.lineid, l.tenlop, d.giam AS phantram, d.lydogiam AS lydo, d.giam_tt, d.giam_gv
           FROM danhsachlop d JOIN lophoc l ON l.lopid = d.lopid
          WHERE d.hocsinhid = $1 AND d.ngaykthoc IS NULL AND d.giam > 0 ORDER BY l.tenlop",
        &[&id],
    )
    .await
    .map_err(map_db)?;
    hs.insert("giam".into(), Value::Array(giam.into_iter().map(Value::Object).collect()));

    Ok(Value::Object(hs))
}

pub async fn lophoc_list(client: &Object, args: &Args) -> Result<Value, AppError> {
    let status = {
        let s = arg_str(args, "status");
        if s.is_empty() {
            "active".into()
        } else {
            s
        }
    };
    let where_sql = match status.as_str() {
        "active" => format!("WHERE {LOP_HD}"),
        "inactive" | "ended" => format!("WHERE NOT {LOP_HD} AND l.luutru IS NULL"),
        "luutru" => "WHERE l.luutru IS NOT NULL".into(),
        _ => "WHERE l.luutru IS NULL".into(),
    };
    let sql = format!(
        "SELECT l.lopid, l.tenlop, l.giaovienid, g.hotengv,
                ({LOP_HD}) AS hoatdong,
                l.monid, m.tenmon, l.khoilop, l.hocphi, l.hocphi_specialmonth,
                l.ngaybatdau, l.ngayketthuc, l.tkb, l.sltoida, l.tilechia, l.mucid, l.sobuoithang, mp.tenmuc,
                (SELECT COUNT(*) FROM danhsachlop d WHERE d.lopid = l.lopid AND d.ngaykthoc IS NULL)::int AS siso
           FROM lophoc l
           LEFT JOIN giaovien g ON g.giaovienid = l.giaovienid
           LEFT JOIN monhoc m ON m.monid = l.monid
           LEFT JOIN muchocphi mp ON mp.mucid = l.mucid
           {where_sql}
          ORDER BY ({LOP_HD}) DESC, l.tenlop"
    );
    let rows = query_json(client, &sql, &[]).await.map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

pub async fn lophoc_get(client: &Object, args: &Args) -> Result<Value, AppError> {
    let id = arg_i64(args, "lopid").ok_or_else(|| AppError::msg("Thiếu lopid."))?;
    let mut lop = query_one_json(
        client,
        "SELECT l.*, g.hotengv, m.tenmon, mp.tenmuc FROM lophoc l
           LEFT JOIN giaovien g ON g.giaovienid = l.giaovienid
           LEFT JOIN monhoc m ON m.monid = l.monid
           LEFT JOIN muchocphi mp ON mp.mucid = l.mucid
          WHERE l.lopid = $1",
        &[&id],
    )
    .await
    .map_err(map_db)?
    .ok_or_else(|| AppError::msg("Không tìm thấy lớp."))?;

    let ds = query_json(
        client,
        &format!(
            "SELECT d.lineid, h.hocsinhid, {HS_TEN} AS hotenhs, h.mahocvien, h.loptruong,
                    h.sdtph, h.sdths, h.sdt3, d.giam AS giamhocphi, d.lydogiam, d.ngaybdhoc,
                    d.ngaykthoc, d.thangdadongtien, d.ngaythem, d.ngaynghi, d.nghi_donglop
               FROM danhsachlop d JOIN hocsinh h ON h.hocsinhid = d.hocsinhid
              WHERE d.lopid = $1
              ORDER BY d.ngaykthoc NULLS FIRST, h.hotenhs"
        ),
        &[&id],
    )
    .await
    .map_err(map_db)?;
    lop.insert(
        "danhsach".into(),
        Value::Array(ds.into_iter().map(Value::Object).collect()),
    );

    if let Some(tkb) = lop.get("tkb").and_then(|v| v.as_str()) {
        let items = tps_logic::parse_tkb(Some(tkb));
        let arr: Vec<Value> = items
            .into_iter()
            .map(|b| json!({"day": b.day, "start": b.start, "end": b.end}))
            .collect();
        lop.insert("tkbItems".into(), Value::Array(arr));
    }

    Ok(Value::Object(lop))
}

pub async fn nhanvien_list(client: &Object, ctx: &Ctx) -> Result<Value, AppError> {
    need(ctx, role::ADMIN)?;
    let rows = query_json(
        client,
        "SELECT n.nhanvienid, n.hotennv, n.hoten, n.sdt, n.diachi, n.chucvu, d.biendichchucvu, n.luong, n.quyen, n.an
           FROM nhanvien n LEFT JOIN doichieuchucvu d ON d.chucvu = n.chucvu
          WHERE n.daxoa IS NULL
          ORDER BY n.chucvu, n.hotennv",
        &[],
    )
    .await
    .map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

pub async fn chucvu_list(client: &Object) -> Result<Value, AppError> {
    let rows = query_json(
        client,
        "SELECT chucvu, biendichchucvu FROM doichieuchucvu ORDER BY chucvu",
        &[],
    )
    .await
    .map_err(map_db)?;
    Ok(Value::Array(rows.into_iter().map(Value::Object).collect()))
}

pub async fn caidat_get(client: &Object) -> Result<Value, AppError> {
    let tt = query_one_json(
        client,
        "SELECT tentrungtam, sdt, diachi, website FROM thongtintt LIMIT 1",
        &[],
    )
    .await
    .map_err(map_db)?
    .unwrap_or_default();
    let ghichu = query_one_json(
        client,
        "SELECT value FROM app_meta WHERE key = 'tt_ghichu'",
        &[],
    )
    .await
    .map_err(map_db)?;
    let mut trungtam = tt;
    trungtam.insert(
        "ghichu".into(),
        ghichu
            .and_then(|m| m.get("value").cloned())
            .unwrap_or(json!("")),
    );
    let tile = query_json(
        client,
        "SELECT lineid, ssmin, ssmax, tileluongconst FROM tinhtileluong ORDER BY ssmin",
        &[],
    )
    .await
    .map_err(map_db)?;
    let bang = query_json(
        client,
        "SELECT lineid, ngayfrom, ngayto, tile FROM bangtinhhp ORDER BY ngayfrom",
        &[],
    )
    .await
    .map_err(map_db)?;
    Ok(json!({
        "trungtam": trungtam,
        "tileluong": tile,
        "bangtinhhp": bang,
    }))
}

/// Ensure roles + empty center row exist (subset of Dart migrate seed).
pub async fn ensure_bootstrap(client: &Object) -> Result<(), AppError> {
    client
        .batch_execute(
            r#"
            INSERT INTO doichieuchucvu (chucvu, biendichchucvu) VALUES
              (1, 'Admin'), (3, 'Nhân viên'), (4, 'Cộng tác viên')
            ON CONFLICT (chucvu) DO UPDATE SET biendichchucvu = EXCLUDED.biendichchucvu;
            INSERT INTO thongtintt (tilechia)
              SELECT 0 WHERE NOT EXISTS (SELECT 1 FROM thongtintt);
            INSERT INTO taikhoan (tienhientai, tientrongket)
              SELECT 0, 0 WHERE NOT EXISTS (SELECT 1 FROM taikhoan);
            "#,
        )
        .await
        .map_err(|e| AppError::msg(e.to_string()))?;
    Ok(())
}

#[allow(dead_code)]
fn _keep_map(m: Map<String, Value>) {
    let _ = m;
}
