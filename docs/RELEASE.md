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

## OTA (S6)

Luồng hiện tại:

1. Tag `vX.Y.Z` trên repo public `tps3.0-rust`.
2. Workflow `release-linux` (tag `v*`) build bundle + upload artifact.
3. App gọi `app.checkUpdate` → GitHub Releases API (`/repos/{owner}/tps3.0-rust/releases/latest`), so sánh semver với `APP_VERSION`.
4. Override repo: env `TPS_OTA_REPO` hoặc args `{ "repo": "owner/name" }`.

In-app updater plugin (tải + cài tự động) **chưa bật** trong build này — cần:

```bash
npm run tauri signer generate -w ~/.tauri/tps3.key
# pubkey → tauri.conf.json plugins.updater.pubkey
# private key → GitHub secret TAURI_SIGNING_PRIVATE_KEY
```

Rồi bật `bundle.createUpdaterArtifacts: true`, thêm `tauri-plugin-updater`, endpoint:

`https://github.com/phamchitai95/tps3.0-rust/releases/latest/download/latest.json`

(và `includeUpdaterJson: true` trên `tauri-apps/tauri-action`).

## Print (S6 subset)

- `hocphi.receipt` `{ ids: [lineid…] }` → JSON + `text` biên lai (console / preview).
- In nhiệt Windows / PDF đầy đủ: port sau.

## RPC tới S6

- S1: `app.*`, `auth.*`, catalog reads  
- S2: `kyhp.sync|list`, `hocphi.collect`, `congno.list`, `quy.get`  
- S3: `ck.bankInfo|getSettings|saveSettings|qrPreview|processPay2s|gdList`  
- S4/S4b: `luong.*`, `bl.*`, `phieuchi.list`  
- S5: HTTP `POST /api/<cmd>` (`tps-web`)  
- S6: `app.checkUpdate`, `hocphi.receipt`
