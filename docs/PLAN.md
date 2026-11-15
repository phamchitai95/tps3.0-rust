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
3. **S2** done — tuition / finance (`kyhp.*`, `hocphi.collect`, `congno.list`, `quy.get`)  
4. **S3** done (subset) — CK settings, VietQR preview, Pay2S ingest (`ck.*`)  
5. UI shell + CI/release docs done  
6. **S4** done (subset) — `luong.preview|lichsu`, `bl.ds|gv`, `phieuchi.list` (`tps-payroll`)  
7. **S4b** done (subset) — `bl.khautru|khautruXoa|chot|mo`, `bl.ck.ds|xuat|tai|huy` (CSV; xlsx MB sau)  
8. S5 web API → S6 OTA + print → S7 so sánh backup thật
