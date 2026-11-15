# TPS web (S5)

Binary: `tps-web` (`crates/tps-web`).

```bash
# config: TPS_DATA_DIR/config.json or QLTH_CONFIG (same as desktop)
TPS_DATA_DIR=~/.local/share/TC59 TPS_WEB_PORT=8787 cargo run -p tps-web
```

- `GET /` — health / version
- `POST /api/<cmd>` — JSON args → `{kq}` / `{loi}` (Bearer after login)
- `POST /api/auth.login` body `{tendangnhap, matkhau}` or `{username, password}` → `{token,user}`
- `POST /pay2s/push` — webhook when `ck_push=1` + matching `pay2s_token`
- `TPS_WEB_CHI_XEM=1` — read-only filter

Not yet: static Flutter UI hosting, R2 backup jobs, TV TKB page, session disk persist.
