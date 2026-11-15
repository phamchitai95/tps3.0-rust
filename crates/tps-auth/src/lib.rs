//! Auth + permissions — behaviour matched to tps59 `core/lib/src/services/auth.dart`.

use rand::RngCore;
use scrypt::{scrypt, Params};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tps_types::{AppError, Ctx, Row};

pub mod role {
    pub const ADMIN: i32 = 1;
    pub const NHAN_VIEN: i32 = 3;
    pub const CTV: i32 = 4;
}

pub fn role_name(chucvu: i32) -> &'static str {
    match chucvu {
        1 => "Admin",
        3 => "Nhân viên",
        4 => "Cộng tác viên",
        _ => "?",
    }
}

/// Fine-grained perms (Admin always has all).
pub fn perms_map() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("sua_phieuthu", "Chỉnh phiếu thu học phí"),
        ("xoa_phieuthu", "Xoá phiếu đã thu"),
        ("sua_lop", "Mở / sửa lớp học"),
        ("xem_nhatky", "Xem nhật ký thay đổi & đánh dấu đã xem"),
        ("xn_chuyenkhoan", "Xác nhận tay chuyển khoản (không có giao dịch ngân hàng)"),
        ("dk_tkb", "Điều khiển trình chiếu TKB lên tivi"),
        ("duyet_chi", "Duyệt phiếu chi (bước 2)"),
    ])
}

pub fn bao_cao_mau() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("tk.hocphi", "Thống kê · Học phí"),
        ("tk.doanhthu", "Thống kê · Doanh thu"),
        ("tk.no", "Thống kê · Nợ học phí"),
        ("tk.hocvien", "Thống kê · Học viên"),
        ("vh.congno", "Vận hành · Công nợ GV"),
        ("vh.siso", "Vận hành · Sĩ số lớp"),
        ("vh.hocvien", "Vận hành · Học viên & chỉ số T"),
        ("vh.phancong", "Vận hành · Phân công NVPT"),
        ("vh.baocao", "Vận hành · Báo cáo tối"),
    ])
}

/// scrypt$salt$hash — salt is hex ASCII string UTF-8-encoded as scrypt salt (same as Node/Dart).
pub fn hash_password(pw: &str) -> String {
    let mut raw = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut raw);
    let salt_hex = hex::encode(raw);
    let hash = scrypt_hash(pw, &salt_hex);
    format!("scrypt${salt_hex}${hash}")
}

fn scrypt_hash(pw: &str, salt_hex: &str) -> String {
    let params = Params::new(14, 8, 1, 32).expect("scrypt params"); // N=2^14=16384
    let mut out = [0u8; 32];
    scrypt(pw.as_bytes(), salt_hex.as_bytes(), &params, &mut out).expect("scrypt");
    hex::encode(out)
}

pub fn verify_password(pw: &str, stored: Option<&str>) -> bool {
    let Some(stored) = stored else {
        return false;
    };
    if let Some(rest) = stored.strip_prefix("scrypt$") {
        let mut parts = rest.splitn(2, '$');
        let salt = parts.next().unwrap_or("");
        let expect = parts.next().unwrap_or("");
        let got = scrypt_hash(pw, salt);
        return const_eq_hex(&got, expect);
    }
    pw == stored // legacy plaintext
}

fn const_eq_hex(a: &str, b: &str) -> bool {
    let x = Sha256::digest(a.as_bytes());
    let y = Sha256::digest(b.as_bytes());
    x.iter().zip(y.iter()).fold(0u8, |d, (i, j)| d | (i ^ j)) == 0
}

pub fn parse_perms(s: Option<&str>) -> Vec<String> {
    let perms = perms_map();
    let bc = bao_cao_mau();
    s.unwrap_or("")
        .split(',')
        .map(|x| x.trim())
        .filter(|x| {
            !x.is_empty()
                && (perms.contains_key(x)
                    || (x.starts_with("bc.") && bc.contains_key(&x[3..])))
        })
        .map(|x| x.to_string())
        .collect()
}

pub fn parse_an(s: Option<&str>) -> Vec<String> {
    let re = regex::Regex::new(r"^[a-z][a-z0-9_.]*$").unwrap();
    s.unwrap_or("")
        .split(',')
        .map(|x| x.trim())
        .filter(|x| re.is_match(x))
        .map(|x| x.to_string())
        .collect()
}

pub fn need(ctx: &Ctx, max_role: i32) -> Result<(), AppError> {
    let u = ctx.user.as_ref().ok_or_else(|| AppError::msg("Vui lòng đăng nhập."))?;
    let chucvu = u
        .get("chucvu")
        .and_then(|v| v.as_i64())
        .unwrap_or(99) as i32;
    if chucvu > max_role {
        return Err(AppError::msg("Bạn không có quyền dùng chức năng này."));
    }
    Ok(())
}

pub fn can(ctx: &Ctx, perm: &str) -> bool {
    let Some(u) = &ctx.user else {
        return false;
    };
    let chucvu = u.get("chucvu").and_then(|v| v.as_i64()).unwrap_or(99) as i32;
    if chucvu <= role::ADMIN {
        return true;
    }
    u.get("quyen")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().any(|x| x.as_str() == Some(perm)))
        .unwrap_or(false)
}

pub fn need_perm(ctx: &Ctx, perm: &str) -> Result<(), AppError> {
    need(ctx, role::CTV)?;
    if !can(ctx, perm) {
        let label = perms_map().get(perm).copied().unwrap_or(perm);
        return Err(AppError::msg(format!("Bạn chưa được cấp quyền \"{label}\".")));
    }
    Ok(())
}

pub fn user_chucvu(ctx: &Ctx) -> i32 {
    ctx.user
        .as_ref()
        .and_then(|u| u.get("chucvu"))
        .and_then(|v| v.as_i64())
        .unwrap_or(99) as i32
}

/// Build session user map from a nhanvien row (+ role name).
pub fn session_user(nv: &Row, tenchucvu: Option<&str>) -> Row {
    use serde_json::json;
    let chucvu = nv.get("chucvu").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    let hotennv = nv.get("hotennv").cloned().unwrap_or(json!(""));
    let hoten = nv
        .get("hoten")
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty());
    let quyen = parse_perms(nv.get("quyen").and_then(|v| v.as_str()));
    let an = parse_an(nv.get("an").and_then(|v| v.as_str()));
    let mut m = Row::new();
    m.insert("nhanvienid".into(), nv.get("nhanvienid").cloned().unwrap_or(json!(null)));
    m.insert("hotennv".into(), hotennv.clone());
    m.insert("chucvu".into(), json!(chucvu));
    m.insert("hoten".into(), json!(hoten));
    m.insert(
        "tenhienthi".into(),
        json!(hoten.unwrap_or_else(|| hotennv.as_str().unwrap_or(""))),
    );
    m.insert(
        "tenchucvu".into(),
        json!(tenchucvu.unwrap_or_else(|| role_name(chucvu))),
    );
    m.insert("quyen".into(), json!(quyen));
    m.insert("an".into(), json!(an));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrypt_roundtrip() {
        let h = hash_password("admin");
        assert!(h.starts_with("scrypt$"));
        assert!(verify_password("admin", Some(&h)));
        assert!(!verify_password("sai", Some(&h)));
    }

    #[test]
    fn plaintext_legacy() {
        assert!(verify_password("admin", Some("admin")));
        assert!(!verify_password("x", Some("admin")));
    }
}
