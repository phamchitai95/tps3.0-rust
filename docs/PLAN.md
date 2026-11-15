# Plan chốt (TPS 3.0 Rust)

- Repo làm việc: **tps3.0-rust** only
- `tps59`: chỉ đọc tham chiếu — không commit / không sửa
- UI: Tauri 2 — bỏ Dart/Flutter
- OTA: GitHub Releases (không qua Tailscale / không bảng `capnhat`)
- Postgres prod: chỉ Tailscale
- Dev: Postgres trong Cloud + fixture (`scripts/init_dev_db.sh`, seed `admin`/`admin`)
- So sánh: `private_data/` + `scripts/restore_so_sanh.sh` → `qlth_so_sanh`

Baseline tham chiếu: tps59 build **276** / migrate **27**.

## Sprint

1. **S0** done — workspace + Tauri + DB + logic tests  
2. **S1** done — auth (scrypt) + catalog reads  
3. **S2** next — tuition / finance (`kyhocphi`, `collectItems`, nợ)  
4. S3 Pay2S → S4 payroll → S5 web API → S6 OTA + print → S7 so sánh backup thật
