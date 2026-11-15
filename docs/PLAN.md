# Plan chốt (TPS 3.0 Rust)

- Repo public: `tps3.0-rust`
- UI: Tauri 2 — bỏ Dart/Flutter
- OTA: GitHub Releases (không qua Tailscale / không bảng `capnhat`)
- Postgres prod: chỉ Tailscale
- Dev: Postgres trong Cloud + fixture
- So sánh: `private_data/` + `scripts/restore_so_sanh.sh` → `qlth_so_sanh`
- Không commit vào repo `tps59`

Baseline tham chiếu: tps59 build **276** / migrate **27**.
