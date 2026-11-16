# private_data — backup thật (không commit)

Thả file `pg_dump` (`.dump` / `.sql`) vào đây để so sánh với logic Rust (**S7**).

```bash
# Trên máy có Postgres thật:
pg_dump -Fc -f qlth_YYYYMMDD.dump qlth

# Trên Cloud / máy dev:
./scripts/so_sanh.sh private_data/qlth_YYYYMMDD.dump
```

Self-test (fixture, không cần dump prod):

```bash
./scripts/so_sanh_selftest.sh
```

Chi tiết checklist: `docs/SO_SANH.md`.

Toàn bộ `*.dump`, `*.sql`, mật khẩu trong thư mục này đều bị `.gitignore`.
