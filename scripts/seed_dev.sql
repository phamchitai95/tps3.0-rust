-- Minimal fixture for Cloud/dev (NOT production data).
-- Run after init_dev_db.sh

INSERT INTO doichieuchucvu (chucvu, biendichchucvu) VALUES
  (1, 'Admin'), (3, 'Nhân viên'), (4, 'Cộng tác viên')
ON CONFLICT (chucvu) DO UPDATE SET biendichchucvu = EXCLUDED.biendichchucvu;

INSERT INTO thongtintt (tentrungtam, tilechia)
SELECT 'Trung tâm thử TPS 3.0', 0
WHERE NOT EXISTS (SELECT 1 FROM thongtintt);

INSERT INTO taikhoan (tienhientai, tientrongket)
SELECT 0, 0 WHERE NOT EXISTS (SELECT 1 FROM taikhoan);

-- admin / admin (plaintext; first login upgrades to scrypt)
INSERT INTO nhanvien (nhanvienid, hotennv, hoten, chucvu, matkhau, quyen, an)
VALUES (1, 'admin', 'Quản trị viên', 1, 'admin', '', '')
ON CONFLICT (nhanvienid) DO UPDATE
  SET hotennv = EXCLUDED.hotennv, matkhau = EXCLUDED.matkhau, chucvu = 1, daxoa = NULL;

INSERT INTO app_meta (key, value) VALUES ('seq:nhanvien', '1')
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value;

INSERT INTO monhoc (monid, tenmon) VALUES (1, 'Toán')
ON CONFLICT (monid) DO UPDATE SET tenmon = EXCLUDED.tenmon;

INSERT INTO giaovien (giaovienid, hotengv, monid, tilechia, trichthue, tientaikhoan, tamdung)
VALUES (1, 'Thầy A', 1, 50, '10', 0, false)
ON CONFLICT (giaovienid) DO UPDATE SET hotengv = EXCLUDED.hotengv, monid = EXCLUDED.monid;

INSERT INTO lophoc (lopid, tenlop, giaovienid, monid, khoilop, hocphi, ngaybatdau, tkb, sobuoithang, hoatdong)
VALUES (1, 'Toán 6A', 1, 1, 6, 500000, CURRENT_DATE - 30, 'T3@17:00-18:30,T7@09:00-10:30', 8, true)
ON CONFLICT (lopid) DO UPDATE SET tenlop = EXCLUDED.tenlop, hocphi = EXCLUDED.hocphi, hoatdong = true, luutru = NULL;

INSERT INTO hocsinh (hocsinhid, hotenhs, mahocvien, sdths, ngaysinh, giamhocphi, sodu, masodiemdanh)
VALUES (1, 'Nguyễn Văn An', 'A001', '0900000001', '1810', 0, 0, '')
ON CONFLICT (hocsinhid) DO UPDATE SET hotenhs = EXCLUDED.hotenhs, mahocvien = EXCLUDED.mahocvien;

INSERT INTO danhsachlop (lineid, lopid, hocsinhid, ngaybdhoc, thangdadongtien, giam, khong_phuthu, nghi_donglop)
VALUES (1, 1, 1, CURRENT_DATE - 20, '', 5, false, false)
ON CONFLICT (lineid) DO UPDATE SET lopid = 1, hocsinhid = 1, ngaykthoc = NULL, giam = 5;

INSERT INTO app_meta (key, value) VALUES
  ('seq:monhoc', '1'),
  ('seq:giaovien', '1'),
  ('seq:lophoc', '1'),
  ('seq:hocsinh', '1'),
  ('seq:danhsachlop', '1')
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value;

UPDATE app_meta SET value = '27' WHERE key = 'migrate_version';
INSERT INTO app_meta (key, value) VALUES ('migrate_version', '27')
ON CONFLICT (key) DO UPDATE SET value = '27';
