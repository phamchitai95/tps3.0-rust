# private_data — backup thật (không commit)

Thả file `pg_dump` (`.dump` / `.sql`) vào đây để so sánh với logic Rust.

```bash
# Trên máy có Postgres thật:
pg_dump -Fc -f qlth_YYYYMMDD.dump qlth

# Trên Cloud / máy dev:
./scripts/restore_so_sanh.sh private_data/qlth_YYYYMMDD.dump
```

Toàn bộ `*.dump`, `*.sql`, mật khẩu trong thư mục này đều bị `.gitignore`.
