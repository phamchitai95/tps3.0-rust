# TPS 3.0 (Rust + Tauri)

Viết lại phần mềm quản lý trung tâm (học phí, Pay2S, lương GV, TKB…) bằng **Rust** + **Tauri 2**.  
Tham chiếu nghiệp vụ: repo Dart/Flutter [tps59](https://github.com/phamchitai95/tps59) (build baseline **276**) — **chỉ đọc, không sửa**.

| | |
|---|---|
| UI | Tauri 2 (desktop); web cùng frontend (sau) |
| Core | Cargo workspace — RPC tên lệnh giống tps59 |
| DB | PostgreSQL — prod chỉ qua Tailscale |
| OTA | GitHub Releases (không nhét gói vào Postgres) |
| License | Proprietary / nội bộ (`NOTICE`) |

## Crates

```
crates/tps-types      # AppError, Ctx, Row
crates/tps-logic      # công thức thuần (fee, TKB, VietQR…)
crates/tps-db         # pool + config (%APPDATA%/TC59 hoặc ~/.local/share/TC59)
crates/tps-migrate    # migrate_version = 27
crates/tps-auth       # scrypt + Role/perms + login
crates/tps-catalog    # mon/gv/hs/lop/nv list+get (đọc)
crates/tps-api        # Api::call registry
src-tauri/            # app desktop TPS 3.0
```

## Dev nhanh (Cloud / Linux)

```bash
# Postgres local user tps / pass tps_dev
./scripts/init_dev_db.sh          # schema + seed → qlth_test (admin/admin)

cargo test -p tps-logic -p tps-auth
QLTH_DATABASE=qlth_test cargo test -p tps-api

npm install
npm run tauri dev                 # cần WebKitGTK trên Linux
```

RPC đã có (S1): `app.init|ping|version`, `auth.*`, `monhoc.list`, `giaovien.list`, `hocsinh.list|get`, `lophoc.list|get`, `nhanvien.list`, `chucvu.list`, `caidat.get`.

Config mẫu:

```json
{
  "host": "127.0.0.1",
  "port": 5432,
  "user": "tps",
  "password": "tps_dev",
  "database": "qlth_test",
  "timeZone": "Asia/Ho_Chi_Minh"
}
```

Đặt file tại `$TPS_DATA_DIR/config.json` hoặc `%APPDATA%\TC59\config.json`.

## Nạp backup thật để so sánh

```bash
./scripts/restore_so_sanh.sh private_data/qlth.dump
```

Xem `private_data/README.md`.

## Roadmap

1. **S0** done — workspace + Tauri + DB + logic tests  
2. **S1** done — auth + catalog reads  
3. **S2** — tuition / finance  
4. S3 Pay2S → S4 payroll → S5 web API → S6 OTA + print → S7 so sánh backup thật
