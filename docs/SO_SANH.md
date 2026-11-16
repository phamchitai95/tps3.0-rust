# S7 — So sánh backup thật

Không commit dump. Mục tiêu: nạp Postgres thật → chạy smoke RPC/SQL trên `qlth_so_sanh`.

## 1. Lấy dump (máy prod / Tailscale)

```bash
pg_dump -Fc -f qlth_YYYYMMDD.dump qlth
# copy vào máy dev:
#   private_data/qlth_YYYYMMDD.dump
```

## 2. Restore + harness

```bash
./scripts/so_sanh.sh private_data/qlth_YYYYMMDD.dump
# hoặc tách bước:
./scripts/restore_so_sanh.sh private_data/qlth_YYYYMMDD.dump
QLTH_DATABASE=qlth_so_sanh SO_SANH_USER=... SO_SANH_PASS=... cargo run -p tps-so-sanh
```

Env hữu ích:

| Env | Mặc định | Nghĩa |
|---|---|---|
| `QLTH_DATABASE` | `qlth_so_sanh` | DB đích |
| `SO_SANH_USER` / `SO_SANH_PASS` | `admin` / `admin` | login smoke |
| `SO_SANH_REQUIRE_LOGIN=1` | off | fail cứng nếu login sai |
| `SO_SANH_SAMPLE` | `5` | số HV so `congno` RPC vs SQL |

## 3. Checklist tay (sau khi harness `ok`)

- [ ] Login được với tài khoản thật
- [ ] `hocsinh.list` / `lophoc.list` không rỗng
- [ ] `congno.list` khớp kỳ còn nợ trên SQL
- [ ] `hocphi.collect` (TM) thử trên DB copy — không trên prod
- [ ] `quy.get` khớp `taikhoan`
- [ ] `bl.ds` tháng hiện tại chạy (có thể soft-warn dump cũ)
- [ ] `app.version` / `diag.soSanh` từ UI tab Hệ thống (cùng DB)

## 4. Self-test không cần dump thật

```bash
./scripts/so_sanh_selftest.sh   # dump qlth_test → qlth_so_sanh → tps-so-sanh
```

## 5. RPC

- CLI đầy đủ: binary `tps-so-sanh`
- In-app: `diag.soSanh` (counts + migrate; cần đã login)
