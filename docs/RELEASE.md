# Đóng gói & OTA (GitHub Releases)

## Dev / Cloud

```bash
./scripts/init_dev_db.sh
cargo test --workspace --exclude tps -- --test-threads=1
npm install && npm run build
```

Config mẫu `tps` / `tps_dev` / `qlth_test` **chỉ dùng fixture Cloud/dev**. Production: Postgres qua Tailscale, mật khẩu riêng, không commit.

## Bản desktop

```bash
npm ci
npm run tauri build
```

Artifact nằm dưới `src-tauri/target/release/bundle/`.

## OTA

1. Tag `vX.Y.Z` trên repo public `tps3.0-rust`.
2. Workflow `release-linux` (và sau này macOS/Windows runners) upload bundle.
3. App cập nhật từ GitHub Releases — **không** nhét gói vào Postgres / Tailscale.

## RPC đã có tới S3

- S1: `app.*`, `auth.*`, catalog reads  
- S2: `kyhp.sync|list`, `hocphi.collect`, `congno.list`, `quy.get`  
- S3: `ck.bankInfo|getSettings|saveSettings|qrPreview|processPay2s|gdList`
