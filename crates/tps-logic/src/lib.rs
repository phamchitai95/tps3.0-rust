//! Pure business rules — ported from tps59 `core/lib/src/logic.dart` (+ VietQR CRC helpers).
//! No database I/O. Keep behaviour identical to Dart for dual-run comparison.

use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

/// Month key as stored historically: "MM-YYYY" (e.g. "03-2018").
pub fn month_key(date_str: &str) -> String {
    let d = &date_str[..10.min(date_str.len())];
    let p: Vec<&str> = d.split('-').collect();
    format!("{}-{}", p[1], p[0])
}

/// Display build number: from build 251, every 100 builds bumps the minor label.
pub fn so_build_hien_thi(build: i32) -> i32 {
    if build > 250 {
        build - ((build - 1) / 100) * 100
    } else {
        build
    }
}

pub fn current_month_key(year: i32, month: u32) -> String {
    format!("{month:02}-{year}")
}

pub fn month_index(key: &str) -> i32 {
    let p: Vec<i32> = key.split('-').map(|x| x.parse().unwrap_or(0)).collect();
    p[1] * 12 + (p[0] - 1)
}

pub fn key_from_index(i: i32) -> String {
    format!("{:02}-{}", (i % 12) + 1, i / 12)
}

pub fn months_between(from_key: &str, to_key: &str) -> Vec<String> {
    let a = month_index(from_key);
    let b = month_index(to_key);
    (a..=b).map(key_from_index).collect()
}

fn month_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\d{2}-\d{4}$").unwrap())
}

pub fn parse_months(s: Option<&str>) -> Vec<String> {
    match s {
        None | Some("") => vec![],
        Some(s) => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| month_re().is_match(x))
            .collect(),
    }
}

pub fn add_month(s: Option<&str>, key: &str) -> String {
    let mut set = parse_months(s);
    if !set.iter().any(|k| k == key) {
        set.push(key.to_string());
    }
    set.sort_by_key(|k| month_index(k));
    set.join(",")
}

pub fn remove_month(s: Option<&str>, key: &str) -> String {
    parse_months(s)
        .into_iter()
        .filter(|k| k != key)
        .collect::<Vec<_>>()
        .join(",")
}

pub const DAYS: [&str; 7] = ["T2", "T3", "T4", "T5", "T6", "T7", "CN"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buoi {
    pub day: String,
    pub start: String,
    pub end: String,
}

fn tkb_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^(T[2-7]|CN)@(\d{2}:\d{2})-(\d{2}:\d{2})$").unwrap())
}

fn hhmm_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^([01]\d|2[0-3]):[0-5]\d$").unwrap())
}

pub fn parse_tkb(s: Option<&str>) -> Vec<Buoi> {
    match s {
        None | Some("") => vec![],
        Some(s) => s
            .split(',')
            .filter_map(|p| {
                let m = tkb_re().captures(p.trim())?;
                Some(Buoi {
                    day: m[1].to_string(),
                    start: m[2].to_string(),
                    end: m[3].to_string(),
                })
            })
            .collect(),
    }
}

pub fn format_tkb(items: &[Buoi]) -> Result<String, String> {
    for i in items {
        if !DAYS.contains(&i.day.as_str()) {
            return Err("Thứ trong lịch học không hợp lệ.".into());
        }
        if !hhmm_re().is_match(&i.start) || !hhmm_re().is_match(&i.end) {
            return Err(
                "Vui lòng nhập đủ giờ bắt đầu và kết thúc cho mỗi buổi học.".into(),
            );
        }
        if i.start >= i.end {
            return Err(format!(
                "Buổi {}: giờ kết thúc phải sau giờ bắt đầu.",
                day_label(&i.day)
            ));
        }
    }
    let mut s = items.to_vec();
    s.sort_by(|a, b| {
        let d = DAYS
            .iter()
            .position(|x| *x == a.day)
            .unwrap_or(0)
            .cmp(&DAYS.iter().position(|x| *x == b.day).unwrap_or(0));
        d.then(a.start.cmp(&b.start))
    });
    Ok(s.iter()
        .map(|i| format!("{}@{}-{}", i.day, i.start, i.end))
        .collect::<Vec<_>>()
        .join(","))
}

fn day_label(day: &str) -> String {
    match day {
        "T2" => "Thứ 2".into(),
        "T3" => "Thứ 3".into(),
        "T4" => "Thứ 4".into(),
        "T5" => "Thứ 5".into(),
        "T6" => "Thứ 6".into(),
        "T7" => "Thứ 7".into(),
        "CN" => "Chủ nhật".into(),
        _ => day.to_string(),
    }
}

/// JS Math.round (0.5 rounds away from -∞ / toward +∞ for positives).
pub fn js_round(v: f64) -> i64 {
    (v + 0.5).floor() as i64
}

fn n(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Number(x) => x.as_f64().unwrap_or(0.0),
        Value::String(s) => s.parse().unwrap_or(0.0),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    }
}

pub fn special_fee(s: Option<&str>, key: &str) -> Option<f64> {
    let s = s?;
    if s.is_empty() {
        return None;
    }
    for part in s.split(',') {
        let kv: Vec<&str> = part.split(':').map(|x| x.trim()).collect();
        if kv.len() >= 2 && kv[0] == key && !kv[1].is_empty() {
            if let Ok(v) = kv[1].parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

/// Tuition for one student / class / month.
pub fn fee_for_month(
    lop: &Map<String, Value>,
    enroll: Option<&Map<String, Value>>,
    giam: f64,
    key: &str,
    bang_tinh_hp: &[Map<String, Value>],
) -> i64 {
    let mut fee = special_fee(
        lop.get("hocphi_specialmonth").and_then(|v| v.as_str()),
        key,
    )
    .unwrap_or_else(|| n(lop.get("hocphi").unwrap_or(&Value::Null)));

    if let Some(en) = enroll {
        if let Some(bd) = en.get("ngaybdhoc").and_then(|v| v.as_str()) {
            if month_key(bd) == key && !bang_tinh_hp.is_empty() {
                let day: f64 = bd[8..10].parse().unwrap_or(0.0);
                for r in bang_tinh_hp {
                    if day >= n(r.get("ngayfrom").unwrap_or(&Value::Null))
                        && day <= n(r.get("ngayto").unwrap_or(&Value::Null))
                    {
                        fee = fee * n(r.get("tile").unwrap_or(&Value::Null)) / 100.0;
                        break;
                    }
                }
            }
        }
    }
    fee = fee * (100.0 - giam) / 100.0;
    js_round(fee / 1000.0) * 1000
}

pub fn unpaid_months(
    lop: &Map<String, Value>,
    enroll: &Map<String, Value>,
    upto_key: &str,
) -> Vec<String> {
    let Some(bd) = enroll.get("ngaybdhoc").and_then(|v| v.as_str()) else {
        return vec![];
    };
    let mut end_idx = month_index(upto_key);
    if let Some(kt) = enroll.get("ngaykthoc").and_then(|v| v.as_str()) {
        end_idx = end_idx.min(month_index(&month_key(kt)));
    }
    if let Some(kt) = lop.get("ngayketthuc").and_then(|v| v.as_str()) {
        end_idx = end_idx.min(month_index(&month_key(kt)));
    }
    let mut start_idx = month_index(&month_key(bd));
    if let Some(bd_lop) = lop.get("ngaybatdau").and_then(|v| v.as_str()) {
        start_idx = start_idx.max(month_index(&month_key(bd_lop)));
    }
    if end_idx < start_idx {
        return vec![];
    }
    let paid: std::collections::HashSet<String> = parse_months(
        enroll
            .get("thangdadongtien")
            .and_then(|v| v.as_str()),
    )
    .into_iter()
    .collect();
    months_between(&key_from_index(start_idx), &key_from_index(end_idx))
        .into_iter()
        .filter(|k| !paid.contains(k))
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct SalaryRate {
    pub rate: f64,
    pub nguon: String,
}

pub fn salary_rate(
    lop: Option<&Map<String, Value>>,
    gv: Option<&Map<String, Value>>,
    siso: i32,
    bang_ti_le: &[Map<String, Value>],
) -> SalaryRate {
    let lop_rate = lop.map(|l| n(l.get("tilechia").unwrap_or(&Value::Null))).unwrap_or(0.0);
    if lop_rate > 0.0 && lop_rate <= 100.0 {
        return SalaryRate {
            rate: lop_rate,
            nguon: "Tỉ lệ riêng của lớp".into(),
        };
    }
    let gv_rate = gv.map(|g| n(g.get("tilechia").unwrap_or(&Value::Null))).unwrap_or(0.0);
    if gv_rate > 0.0 && gv_rate <= 100.0 {
        return SalaryRate {
            rate: gv_rate,
            nguon: "Tỉ lệ riêng của giáo viên".into(),
        };
    }
    let mut sorted = bang_ti_le.to_vec();
    sorted.sort_by(|a, b| {
        n(a.get("ssmin").unwrap_or(&Value::Null))
            .partial_cmp(&n(b.get("ssmin").unwrap_or(&Value::Null)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for r in &sorted {
        let ssmin = n(r.get("ssmin").unwrap_or(&Value::Null)) as i32;
        let ssmax = n(r.get("ssmax").unwrap_or(&Value::Null)) as i32;
        if siso >= ssmin && siso <= ssmax {
            return SalaryRate {
                rate: n(r.get("tileluongconst").unwrap_or(&Value::Null)),
                nguon: format!("Sĩ số {ssmin}–{ssmax}"),
            };
        }
    }
    SalaryRate {
        rate: 0.0,
        nguon: "Chưa có tỉ lệ".into(),
    }
}

/// Normalize birth day-month to 4 digits "ddmm".
pub fn norm_ngay_sinh(v: &str) -> Result<Option<String>, String> {
    let s = v.trim();
    if s.is_empty() {
        return Ok(None);
    }
    let re1 = Regex::new(r"^(\d{1,2})\s*[/.\-]\s*(\d{1,2})$").unwrap();
    let re2 = Regex::new(r"^(\d{2})(\d{2})$").unwrap();
    let (d, mo) = if let Some(m) = re1.captures(s) {
        (m[1].parse::<u32>().unwrap(), m[2].parse::<u32>().unwrap())
    } else if let Some(m) = re2.captures(s) {
        (m[1].parse::<u32>().unwrap(), m[2].parse::<u32>().unwrap())
    } else {
        return Err(
            "Ngày tháng sinh nhập 4 số ngày + tháng, vd 1810 (= 18/10).".into(),
        );
    };
    const MAX_DAY: [u32; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if mo < 1 || mo > 12 || d < 1 || d > MAX_DAY[(mo - 1) as usize] {
        return Err(format!("Ngày tháng sinh không hợp lệ: {s}."));
    }
    Ok(Some(format!("{d:02}{mo:02}")))
}

// ---- VietQR (from chuyenkhoan.dart) ----

fn tlv(id: &str, v: &str) -> String {
    format!("{id}{:02}{v}", v.len())
}

pub fn crc16(s: &str) -> String {
    let mut crc: u32 = 0xffff;
    for c in s.bytes() {
        crc ^= (c as u32) << 8;
        for _ in 0..8 {
            crc = if (crc & 0x8000) != 0 {
                ((crc << 1) ^ 0x1021) & 0xffff
            } else {
                (crc << 1) & 0xffff
            };
        }
    }
    format!("{crc:04X}")
}

const VN: &str = "àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđ";
const EN: &str = "aaaaaaaaaaaaaaaaaeeeeeeeeeeeiiiiiooooooooooooooooouuuuuuuuuuuyyyyyd";

pub fn unaccent(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        let lo = ch.to_lowercase().next().unwrap_or(ch);
        if let Some(i) = VN.chars().position(|c| c == lo) {
            let en = EN.chars().nth(i).unwrap_or(lo);
            if ch != lo {
                out.extend(en.to_uppercase());
            } else {
                out.push(en);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

pub fn ascii_content(s: &str) -> String {
    let re = Regex::new(r"[^A-Za-z0-9 -]").unwrap();
    let folded = unaccent(s);
    let t = re.replace_all(&folded, "");
    if t.len() > 50 {
        t[..50].to_string()
    } else {
        t.to_string()
    }
}

pub fn viet_qr_payload(bin: &str, sotaikhoan: &str, sotien: f64, noidung: &str) -> String {
    let merchant = tlv("00", "A000000727")
        + &tlv("01", &(tlv("00", bin) + &tlv("01", sotaikhoan)))
        + &tlv("02", "QRIBFTTA");
    let mut s = tlv("00", "01")
        + &tlv("01", if sotien > 0.0 { "12" } else { "11" })
        + &tlv("38", &merchant)
        + &tlv("53", "704");
    if sotien > 0.0 {
        s += &tlv("54", &format!("{}", sotien.round() as i64));
    }
    s += &tlv("58", "VN");
    if !noidung.is_empty() {
        s += &tlv("62", &tlv("08", &ascii_content(noidung)));
    }
    s += "6304";
    let crc = crc16(&s);
    s + &crc
}

/// Helper: build a JSON object map from pairs (for tests / call sites).
pub fn obj(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hoc_phi_tru_giam() {
        let lop = obj(&[("hocphi", json!(500000))]);
        assert_eq!(fee_for_month(&lop, None, 5.0, "03-2018", &[]), 475000);
    }

    #[test]
    fn hoc_phi_thang_dau_bang_tinh() {
        let lop = obj(&[("hocphi", json!(500000))]);
        let enroll = obj(&[("ngaybdhoc", json!("2018-03-25"))]);
        let bang = [obj(&[
            ("ngayfrom", json!(16)),
            ("ngayto", json!(31)),
            ("tile", json!(50)),
        ])];
        assert_eq!(
            fee_for_month(&lop, Some(&enroll), 0.0, "03-2018", &bang),
            250000
        );
    }

    #[test]
    fn hoc_phi_rieng_theo_thang() {
        let lop = obj(&[
            ("hocphi", json!(500000)),
            ("hocphi_specialmonth", json!("06-2018:300000")),
        ]);
        assert_eq!(fee_for_month(&lop, None, 0.0, "06-2018", &[]), 300000);
    }

    #[test]
    fn thang_con_no() {
        let lop = obj(&[("ngayketthuc", json!("2019-03-01"))]);
        let enroll = obj(&[
            ("ngaybdhoc", json!("2018-03-25")),
            ("thangdadongtien", json!("03-2018,05-2018")),
        ]);
        assert_eq!(
            unpaid_months(&lop, &enroll, "06-2018"),
            vec!["04-2018".to_string(), "06-2018".to_string()]
        );
    }

    #[test]
    fn ti_le_luong_theo_si_so() {
        let bang = [
            obj(&[
                ("ssmin", json!(0)),
                ("ssmax", json!(10)),
                ("tileluongconst", json!(50)),
            ]),
            obj(&[
                ("ssmin", json!(10)),
                ("ssmax", json!(100)),
                ("tileluongconst", json!(60)),
            ]),
        ];
        assert_eq!(salary_rate(None, None, 2, &bang).rate, 50.0);
    }

    #[test]
    fn ti_le_rieng_lop_uu_tien() {
        let lop = obj(&[("tilechia", json!(70))]);
        let gv = obj(&[("tilechia", json!(40))]);
        assert_eq!(salary_rate(Some(&lop), Some(&gv), 2, &[]).rate, 70.0);
    }

    #[test]
    fn tkb_doc_ghi() {
        let parsed = parse_tkb(Some("T7@09:00-10:30,T3@17:00-18:30"));
        assert_eq!(
            format_tkb(&parsed).unwrap(),
            "T3@17:00-18:30,T7@09:00-10:30"
        );
    }

    #[test]
    fn tkb_gio_ket_thuc_sau() {
        let bad = [Buoi {
            day: "T2".into(),
            start: "18:00".into(),
            end: "17:00".into(),
        }];
        assert!(format_tkb(&bad).is_err());
    }

    #[test]
    fn vietqr_crc16() {
        assert_eq!(crc16("123456789"), "29B1");
    }

    #[test]
    fn vietqr_payload_khop_ban_1_0() {
        assert_eq!(
            viet_qr_payload("970436", "0123456789", 300000.0, "A006HP000003"),
            "00020101021238540010A00000072701240006970436011001234567890208QRIBFTTA530370454063000005802VN62160812A006HP00000363046C37"
        );
    }

    #[test]
    fn ngay_sinh() {
        assert_eq!(norm_ngay_sinh("18/10").unwrap(), Some("1810".into()));
        assert_eq!(norm_ngay_sinh("1810").unwrap(), Some("1810".into()));
        assert_eq!(norm_ngay_sinh("1-2").unwrap(), Some("0102".into()));
        assert_eq!(norm_ngay_sinh("").unwrap(), None);
        assert!(norm_ngay_sinh("3213").is_err());
        assert!(norm_ngay_sinh("3002").is_err());
    }

    #[test]
    fn so_build_hien_thi_276() {
        assert_eq!(so_build_hien_thi(276), 76);
        assert_eq!(so_build_hien_thi(251), 51);
        assert_eq!(so_build_hien_thi(200), 200);
    }
}
