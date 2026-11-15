--
-- PostgreSQL database dump
--


-- Dumped from database version 18.6
-- Dumped by pg_dump version 18.6

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: tc59_fold(text); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.tc59_fold(t text) RETURNS text
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
         SELECT regexp_replace(lower(translate(normalize(coalesce(t, ''), NFC), 'àáạảãâầấậẩẫăằắặẳẵèéẹẻẽêềếệểễìíịỉĩòóọỏõôồốộổỗơờớợởỡùúụủũưừứựửữỳýỵỷỹđÀÁẠẢÃÂẦẤẬẨẪĂẰẮẶẲẴÈÉẸẺẼÊỀẾỆỂỄÌÍỊỈĨÒÓỌỎÕÔỒỐỘỔỖƠỜỚỢỞỠÙÚỤỦŨƯỪỨỰỬỮỲÝỴỶỸĐ', 'aaaaaaaaaaaaaaaaaeeeeeeeeeeeiiiiiooooooooooooooooouuuuuuuuuuuyyyyydaaaaaaaaaaaaaaaaaeeeeeeeeeeeiiiiiooooooooooooooooouuuuuuuuuuuyyyyyd')), '[[:space:]]+', ' ', 'g') $$;


--
-- Name: tc59_initials(text); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.tc59_initials(t text) RETURNS text
    LANGUAGE sql IMMUTABLE PARALLEL SAFE
    AS $$
         SELECT coalesce(string_agg(left(w, 1), ''), '') FROM regexp_split_to_table(btrim(tc59_fold(t)), ' ') AS w WHERE w <> '' $$;


--
-- Name: tps_dsl_biendong(); Type: FUNCTION; Schema: public; Owner: -
--

CREATE FUNCTION public.tps_dsl_biendong() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
      BEGIN
        IF TG_OP = 'INSERT' THEN
          IF NEW.ngaythem IS NULL THEN NEW.ngaythem := clock_timestamp()::timestamp(3); END IF;
          IF NEW.ngaykthoc IS NOT NULL AND NEW.ngaynghi IS NULL THEN NEW.ngaynghi := clock_timestamp()::timestamp(3); END IF;
        ELSIF OLD.ngaykthoc IS NULL AND NEW.ngaykthoc IS NOT NULL THEN
          NEW.ngaynghi := clock_timestamp()::timestamp(3);
          NEW.nghi_donglop := COALESCE(current_setting('tps.donglop', true), '') = '1';
        ELSIF OLD.ngaykthoc IS NOT NULL AND NEW.ngaykthoc IS NULL THEN
          NEW.ngaynghi := NULL;
          NEW.nghi_donglop := false;
        END IF;
        RETURN NEW;
      END $$;


SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: app_meta; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.app_meta (
    key character varying(50) NOT NULL,
    value text
);


--
-- Name: aspnetroles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aspnetroles (
    id character varying(128) NOT NULL,
    name character varying(256) NOT NULL
);


--
-- Name: aspnetuserroles; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aspnetuserroles (
    userid character varying(128) NOT NULL,
    roleid character varying(128) NOT NULL
);


--
-- Name: aspnetusers; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.aspnetusers (
    id character varying(128) NOT NULL,
    email character varying(256),
    emailconfirmed boolean DEFAULT false NOT NULL,
    passwordhash text,
    securitystamp text,
    phonenumber text,
    phonenumberconfirmed boolean DEFAULT false NOT NULL,
    twofactorenabled boolean DEFAULT false NOT NULL,
    lockoutenddateutc timestamp(3) without time zone,
    lockoutenabled boolean DEFAULT false NOT NULL,
    accessfailedcount integer DEFAULT 0 NOT NULL,
    username character varying(256) NOT NULL
);


--
-- Name: bangiao; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bangiao (
    ngay date NOT NULL,
    tienmat bigint DEFAULT 0 NOT NULL,
    sophieutm integer DEFAULT 0 NOT NULL,
    chuyenkhoan bigint DEFAULT 0 NOT NULL,
    sophieuck integer DEFAULT 0 NOT NULL,
    chi bigint DEFAULT 0 NOT NULL,
    sophieuchi integer DEFAULT 0 NOT NULL,
    phainop bigint DEFAULT 0 NOT NULL,
    thucnop bigint DEFAULT 0 NOT NULL,
    ghichu character varying(500),
    nhanvienid integer,
    thoigian timestamp(0) without time zone DEFAULT now() NOT NULL
);


--
-- Name: bangiao_anh; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bangiao_anh (
    ngay date NOT NULL,
    vitri smallint NOT NULL,
    ghichu character varying(200),
    dulieu bytea NOT NULL,
    nhanvienid integer,
    thoigian timestamp(0) without time zone DEFAULT now() NOT NULL,
    luotid integer,
    CONSTRAINT bangiao_anh_vitri_check CHECK (((vitri >= 1) AND (vitri <= 5)))
);


--
-- Name: bangtinhhp; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.bangtinhhp (
    lineid integer NOT NULL,
    ngayfrom integer,
    ngayto integer,
    tile integer
);


--
-- Name: baocaotoi; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.baocaotoi (
    id integer NOT NULL,
    ngay date NOT NULL,
    tudong boolean DEFAULT false NOT NULL,
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL,
    telegram text,
    zalo text,
    trangthai character varying(20),
    noidung text,
    nhanvienid integer
);


--
-- Name: baocaotoi_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.baocaotoi_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: baocaotoi_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.baocaotoi_id_seq OWNED BY public.baocaotoi.id;


--
-- Name: cahoc; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.cahoc (
    caid integer NOT NULL,
    giobatdau time without time zone,
    gioketthuc time without time zone
);


--
-- Name: capnhat; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.capnhat (
    build integer NOT NULL,
    phienban character varying(20) NOT NULL,
    sha256 character(64) NOT NULL,
    kichthuoc bigint NOT NULL,
    chuky text NOT NULL,
    ghichu text,
    ngaytao timestamp(0) without time zone DEFAULT (date_trunc('second'::text, now()))::timestamp without time zone NOT NULL,
    dulieu bytea NOT NULL
);


--
-- Name: capnhat_lichsu; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.capnhat_lichsu (
    build integer NOT NULL,
    phienban character varying(20) NOT NULL,
    ghichu text,
    ngaytao timestamp(0) without time zone DEFAULT (date_trunc('second'::text, now()))::timestamp without time zone,
    moc text
);


--
-- Name: chamconggv; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.chamconggv (
    lineid integer NOT NULL,
    giaovienid integer,
    lopid integer,
    ngaycham date,
    sotiet integer,
    ghichu character varying(500)
);


--
-- Name: chotluong; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.chotluong (
    chotid integer NOT NULL,
    giaovienid integer NOT NULL,
    thang character varying(7) NOT NULL,
    trangthai character varying(4) DEFAULT 'chot'::character varying NOT NULL,
    lan integer DEFAULT 1 NOT NULL,
    thoigian timestamp(0) without time zone NOT NULL,
    nhanvienid integer,
    xuatluc timestamp(0) without time zone,
    file character varying(500),
    tonglanh bigint DEFAULT 0 NOT NULL,
    tongtruy bigint DEFAULT 0 NOT NULL,
    tongdc bigint DEFAULT 0 NOT NULL,
    tonggv bigint DEFAULT 0 NOT NULL,
    tongtt bigint DEFAULT 0 NOT NULL,
    tongno bigint DEFAULT 0 NOT NULL,
    sono integer DEFAULT 0 NOT NULL,
    quahan integer DEFAULT 0 NOT NULL,
    tonghplanh bigint DEFAULT 0 NOT NULL,
    tonghptruy bigint DEFAULT 0 NOT NULL,
    solop integer DEFAULT 0 NOT NULL,
    trichthue character varying(2) DEFAULT '10'::character varying NOT NULL,
    tongthue bigint DEFAULT 0 NOT NULL,
    thucnhan bigint DEFAULT 0 NOT NULL,
    mst character varying(20),
    stk character varying(30),
    nganhang character varying(120),
    chutk character varying(150),
    tongtamung bigint DEFAULT 0 NOT NULL,
    tongkhac bigint DEFAULT 0 NOT NULL
);


--
-- Name: chotluong_chotid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.chotluong_chotid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: chotluong_chotid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.chotluong_chotid_seq OWNED BY public.chotluong.chotid;


--
-- Name: chotluong_dong; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.chotluong_dong (
    id integer NOT NULL,
    chotid integer NOT NULL,
    lopid integer NOT NULL,
    loai character varying(4) NOT NULL,
    khoa character varying(30),
    phieuid integer,
    kyid integer,
    hocsinhid integer,
    hotenhs character varying(250),
    mahocvien character varying(30),
    thang character varying(7),
    thoigian timestamp(0) without time zone,
    hpthu bigint,
    gvnhan bigint,
    ttnhan bigint,
    conno bigint,
    ghichutd character varying(1000),
    ghichunv character varying(500),
    ghichugv character varying(1000)
);


--
-- Name: chotluong_dong_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.chotluong_dong_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: chotluong_dong_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.chotluong_dong_id_seq OWNED BY public.chotluong_dong.id;


--
-- Name: chotluong_khautru; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.chotluong_khautru (
    id integer NOT NULL,
    chotid integer NOT NULL,
    khautruid integer,
    loai character varying(8) NOT NULL,
    sotien bigint NOT NULL,
    lydo character varying(500),
    nguoighi character varying(200),
    thoigian timestamp(0) without time zone
);


--
-- Name: chotluong_khautru_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.chotluong_khautru_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: chotluong_khautru_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.chotluong_khautru_id_seq OWNED BY public.chotluong_khautru.id;


--
-- Name: chotluong_lop; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.chotluong_lop (
    chotid integer NOT NULL,
    lopid integer NOT NULL,
    tenlop character varying(200),
    siso integer,
    tile numeric,
    ghichu character varying(500),
    quahan integer DEFAULT 0 NOT NULL,
    quahantien bigint DEFAULT 0 NOT NULL
);


--
-- Name: chuyenluong_dong; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.chuyenluong_dong (
    id integer NOT NULL,
    loid integer NOT NULL,
    stt integer NOT NULL,
    chotid integer NOT NULL,
    giaovienid integer NOT NULL,
    stk character varying(30) NOT NULL,
    ten character varying(150) NOT NULL,
    nganhang character varying(200) NOT NULL,
    sotien bigint NOT NULL
);


--
-- Name: chuyenluong_dong_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.chuyenluong_dong_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: chuyenluong_dong_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.chuyenluong_dong_id_seq OWNED BY public.chuyenluong_dong.id;


--
-- Name: chuyenluong_lo; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.chuyenluong_lo (
    loid integer NOT NULL,
    thang character varying(7) NOT NULL,
    thoigian timestamp(0) without time zone NOT NULL,
    nhanvienid integer,
    soluong integer NOT NULL,
    tongtien bigint NOT NULL,
    chitiet character varying(200) NOT NULL,
    huyluc timestamp(0) without time zone,
    huynv integer,
    huylydo character varying(300)
);


--
-- Name: chuyenluong_lo_loid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.chuyenluong_lo_loid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: chuyenluong_lo_loid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.chuyenluong_lo_loid_seq OWNED BY public.chuyenluong_lo.loid;


--
-- Name: danhsachdanghoc; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.danhsachdanghoc (
    hocsinhid integer NOT NULL,
    thoigianden timestamp(3) without time zone
);


--
-- Name: danhsachlop; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.danhsachlop (
    lineid integer NOT NULL,
    lopid integer NOT NULL,
    hocsinhid integer,
    ngaybdhoc date,
    ngaykthoc date,
    thangdadongtien character varying(500),
    ngayvanghoc character varying(500),
    hocphino integer,
    thangconthieu character varying(500),
    diemtongket numeric(10,2),
    giam integer DEFAULT 0 NOT NULL,
    lydogiam text,
    giam_tt integer,
    giam_gv integer,
    khong_phuthu boolean DEFAULT false NOT NULL,
    lydo_khongphuthu character varying(300),
    ngaythem timestamp(3) without time zone,
    ngaynghi timestamp(3) without time zone,
    nghi_donglop boolean DEFAULT false NOT NULL
);


--
-- Name: diemdanh; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.diemdanh (
    lopid integer NOT NULL,
    hocsinhid integer NOT NULL,
    ngay date NOT NULL,
    comat boolean DEFAULT true NOT NULL,
    ghichu character varying(200)
);


--
-- Name: dinhkem; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.dinhkem (
    loai character varying(4) NOT NULL,
    lineid integer NOT NULL,
    dulieu bytea NOT NULL,
    tenfile character varying(255),
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL,
    nhanvienid integer,
    CONSTRAINT dinhkem_loai_check CHECK (((loai)::text = ANY ((ARRAY['thu'::character varying, 'chi'::character varying])::text[])))
);


--
-- Name: dinhkem_cu; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.dinhkem_cu (
    id integer NOT NULL,
    loai character varying(4) NOT NULL,
    lineid integer NOT NULL,
    dulieu bytea NOT NULL,
    tenfile character varying(255),
    thoigian timestamp(0) without time zone,
    nhanvienid integer,
    thayboi integer,
    thayluc timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL
);


--
-- Name: dinhkem_cu_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.dinhkem_cu_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: dinhkem_cu_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.dinhkem_cu_id_seq OWNED BY public.dinhkem_cu.id;


--
-- Name: doichieuchucvu; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.doichieuchucvu (
    chucvu integer NOT NULL,
    biendichchucvu character varying(100)
);


--
-- Name: giaodich_ck; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.giaodich_ck (
    gdid integer NOT NULL,
    nguon character varying(20) DEFAULT 'pay2s'::character varying NOT NULL,
    magd character varying(80) NOT NULL,
    nganhang character varying(40),
    sotaikhoan character varying(40),
    noidung character varying(500),
    sotien bigint NOT NULL,
    ngaygd timestamp(0) without time zone,
    trangthai character varying(12) NOT NULL,
    ycid integer,
    hocsinhid integer,
    ghichu character varying(300),
    nhanluc timestamp(0) without time zone DEFAULT now() NOT NULL,
    raw jsonb
);


--
-- Name: giaodich_ck_gdid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.giaodich_ck_gdid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: giaodich_ck_gdid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.giaodich_ck_gdid_seq OWNED BY public.giaodich_ck.gdid;


--
-- Name: giaovien; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.giaovien (
    giaovienid integer NOT NULL,
    hotengv character varying(100) NOT NULL,
    sdtgv character varying(30),
    diachi character varying(200),
    monid integer,
    tientaikhoan integer,
    tilechia integer,
    matkhau character varying(30),
    hotendaydu character varying(100),
    trichthue character varying(2) DEFAULT '10'::character varying NOT NULL,
    mst character varying(20),
    stk character varying(30),
    nganhang character varying(120),
    chutk character varying(150),
    danhxung character varying(10),
    tamdung boolean DEFAULT false NOT NULL
);


--
-- Name: giaovien_lichsu; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.giaovien_lichsu (
    id integer NOT NULL,
    giaovienid integer NOT NULL,
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL,
    nhanvienid integer,
    truong character varying(40) NOT NULL,
    giatricu character varying(300),
    giatrimoi character varying(300)
);


--
-- Name: giaovien_lichsu_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.giaovien_lichsu_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: giaovien_lichsu_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.giaovien_lichsu_id_seq OWNED BY public.giaovien_lichsu.id;


--
-- Name: hoantra; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.hoantra (
    id integer NOT NULL,
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL,
    loai character varying(8) NOT NULL,
    huyphieu boolean DEFAULT false NOT NULL,
    hocsinhid integer,
    lopid integer,
    phieuid integer,
    thang character varying(7),
    sotien bigint NOT NULL,
    hinhthuc character varying(2),
    lydo character varying(500),
    nhanvienid integer,
    phieuthukhacid integer,
    phieuchiid integer,
    kycu jsonb,
    giuphieu boolean DEFAULT false NOT NULL,
    chotluongid integer
);


--
-- Name: hoantra_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.hoantra_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: hoantra_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.hoantra_id_seq OWNED BY public.hoantra.id;


--
-- Name: hocsinh; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.hocsinh (
    hocsinhid integer NOT NULL,
    hotenhs character varying(100),
    namsinh integer,
    hotenph character varying(100),
    sdtph character varying(30),
    sdths character varying(30),
    ghichu character varying(200),
    masodiemdanh character varying(1800),
    giamhocphi integer DEFAULT 0 NOT NULL,
    mahocvien character varying(20),
    ngaysinh character varying(4),
    sdt3 character varying(20),
    email character varying(120),
    sodu bigint DEFAULT 0 NOT NULL,
    loptruong character varying(40)
);


--
-- Name: hocsinh_giam; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.hocsinh_giam (
    hocsinhid integer NOT NULL,
    monid integer NOT NULL,
    phantram integer NOT NULL,
    lydo text,
    CONSTRAINT hocsinh_giam_phantram_check CHECK (((phantram >= 0) AND (phantram <= 100)))
);


--
-- Name: kb_bang; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_bang (
    bangid integer NOT NULL,
    ten character varying(120) NOT NULL,
    mota text,
    bimat boolean DEFAULT false NOT NULL,
    chu integer NOT NULL,
    luutru_ngay integer DEFAULT 7 NOT NULL,
    taoluc timestamp(0) without time zone DEFAULT now() NOT NULL,
    salt character varying(40),
    vong integer,
    khoa_pin text,
    xacthuc character varying(64),
    salt_kp character varying(40),
    khoa_kp text,
    xacthuc_kp character varying(64),
    sai_pin integer DEFAULT 0 NOT NULL,
    khoa_den timestamp(0) without time zone
);


--
-- Name: kb_bang_bangid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kb_bang_bangid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kb_bang_bangid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kb_bang_bangid_seq OWNED BY public.kb_bang.bangid;


--
-- Name: kb_binhluan; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_binhluan (
    blid integer NOT NULL,
    theid integer NOT NULL,
    nhanvienid integer,
    noidung text NOT NULL,
    luc timestamp(0) without time zone DEFAULT now() NOT NULL
);


--
-- Name: kb_binhluan_blid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kb_binhluan_blid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kb_binhluan_blid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kb_binhluan_blid_seq OWNED BY public.kb_binhluan.blid;


--
-- Name: kb_cot; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_cot (
    cotid integer NOT NULL,
    bangid integer NOT NULL,
    ten character varying(60) NOT NULL,
    thutu integer DEFAULT 0 NOT NULL,
    xong boolean DEFAULT false NOT NULL
);


--
-- Name: kb_cot_cotid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kb_cot_cotid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kb_cot_cotid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kb_cot_cotid_seq OWNED BY public.kb_cot.cotid;


--
-- Name: kb_lichsu; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_lichsu (
    id integer NOT NULL,
    theid integer NOT NULL,
    tu_cot character varying(60),
    den_cot character varying(60) NOT NULL,
    nhanvienid integer,
    luc timestamp(0) without time zone DEFAULT now() NOT NULL
);


--
-- Name: kb_lichsu_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kb_lichsu_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kb_lichsu_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kb_lichsu_id_seq OWNED BY public.kb_lichsu.id;


--
-- Name: kb_tep; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_tep (
    tepid integer NOT NULL,
    theid integer NOT NULL,
    ten text NOT NULL,
    loai character varying(80),
    kichthuoc integer NOT NULL,
    dulieu bytea NOT NULL,
    nhanvienid integer,
    luc timestamp(0) without time zone DEFAULT now() NOT NULL
);


--
-- Name: kb_tep_tepid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kb_tep_tepid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kb_tep_tepid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kb_tep_tepid_seq OWNED BY public.kb_tep.tepid;


--
-- Name: kb_thanhvien; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_thanhvien (
    bangid integer NOT NULL,
    nv integer NOT NULL,
    muc character varying(4) NOT NULL,
    CONSTRAINT kb_thanhvien_muc_check CHECK (((muc)::text = ANY ((ARRAY['ql'::character varying, 'lv'::character varying, 'xem'::character varying])::text[])))
);


--
-- Name: kb_the; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_the (
    theid integer NOT NULL,
    bangid integer NOT NULL,
    cotid integer NOT NULL,
    thutu double precision DEFAULT 0 NOT NULL,
    tieude text NOT NULL,
    mota text,
    nguoinhan integer,
    han date,
    uutien smallint DEFAULT 0 NOT NULL,
    nhan character varying(200),
    hocsinhid integer,
    lopid integer,
    nhanvienid integer,
    taoluc timestamp(0) without time zone DEFAULT now() NOT NULL,
    sualuc timestamp(0) without time zone DEFAULT now() NOT NULL,
    vaoxong timestamp(0) without time zone,
    luutru timestamp(0) without time zone
);


--
-- Name: kb_the_theid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kb_the_theid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kb_the_theid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kb_the_theid_seq OWNED BY public.kb_the.theid;


--
-- Name: kb_viec; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kb_viec (
    viecid integer NOT NULL,
    theid integer NOT NULL,
    ten text NOT NULL,
    xong boolean DEFAULT false NOT NULL,
    thutu integer DEFAULT 0 NOT NULL
);


--
-- Name: kb_viec_viecid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kb_viec_viecid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kb_viec_viecid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kb_viec_viecid_seq OWNED BY public.kb_viec.viecid;


--
-- Name: kydangky; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kydangky (
    thang character varying(7) NOT NULL,
    trangthai character varying(4) DEFAULT 'mo'::character varying NOT NULL,
    ghichu character varying(300),
    nhanvienid integer,
    ngaytao timestamp(0) without time zone DEFAULT now() NOT NULL,
    CONSTRAINT kydangky_trangthai_check CHECK (((trangthai)::text = ANY ((ARRAY['mo'::character varying, 'dong'::character varying])::text[])))
);


--
-- Name: kyhocphi; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.kyhocphi (
    kyid integer NOT NULL,
    makyhp character varying(40) NOT NULL,
    hocsinhid integer NOT NULL,
    lopid integer NOT NULL,
    lineid integer,
    thang character varying(7) NOT NULL,
    sotien integer NOT NULL,
    dathu integer DEFAULT 0 NOT NULL,
    trangthai character varying(10) DEFAULT 'chuathu'::character varying NOT NULL,
    ghichu character varying(300),
    ngaytao timestamp(3) without time zone DEFAULT (date_trunc('second'::text, now()))::timestamp without time zone NOT NULL,
    chot boolean DEFAULT false NOT NULL,
    phuthu integer DEFAULT 0 NOT NULL,
    phidu integer,
    CONSTRAINT kyhocphi_trangthai_check CHECK (((trangthai)::text = ANY ((ARRAY['chuathu'::character varying, 'dathu'::character varying, 'huy'::character varying, 'treo'::character varying])::text[])))
);


--
-- Name: kyhocphi_kyid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.kyhocphi_kyid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: kyhocphi_kyid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.kyhocphi_kyid_seq OWNED BY public.kyhocphi.kyid;


--
-- Name: lichchung; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lichchung (
    lichid integer NOT NULL,
    ngay date NOT NULL,
    denngay date,
    loai character varying(10) DEFAULT 'viec'::character varying NOT NULL,
    tieude character varying(200) NOT NULL,
    ghichu character varying(1000),
    hangnam boolean DEFAULT false NOT NULL,
    nhanvienid integer,
    taoluc timestamp(0) without time zone DEFAULT now() NOT NULL,
    suanv integer,
    sualuc timestamp(0) without time zone
);


--
-- Name: lichchung_lichid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.lichchung_lichid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: lichchung_lichid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.lichchung_lichid_seq OWNED BY public.lichchung.lichid;


--
-- Name: lichnghigv; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lichnghigv (
    lineid integer NOT NULL,
    giaovienid integer,
    lopid integer,
    ngaynghi timestamp(3) without time zone,
    ngaybu timestamp(3) without time zone,
    ghichu character varying(200),
    hoanthanh integer
);


--
-- Name: lidothuchi; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lidothuchi (
    id integer NOT NULL,
    loai character varying(4) NOT NULL,
    ten character varying(150) NOT NULL,
    thutu integer DEFAULT 0 NOT NULL,
    CONSTRAINT lidothuchi_loai_check CHECK (((loai)::text = ANY ((ARRAY['thu'::character varying, 'chi'::character varying])::text[])))
);


--
-- Name: lidothuchi_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.lidothuchi_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: lidothuchi_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.lidothuchi_id_seq OWNED BY public.lidothuchi.id;


--
-- Name: lop_daxem; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lop_daxem (
    nhanvienid integer NOT NULL,
    lopid integer NOT NULL,
    luc timestamp(3) without time zone DEFAULT (clock_timestamp())::timestamp(3) without time zone NOT NULL
);


--
-- Name: lop_inds; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lop_inds (
    id integer NOT NULL,
    lopid integer NOT NULL,
    soban integer DEFAULT 1 NOT NULL,
    socot integer,
    sothang integer,
    tuthang character varying(7),
    sotrang integer,
    nhanvienid integer,
    ngaytao timestamp(0) without time zone DEFAULT (date_trunc('second'::text, now()))::timestamp without time zone NOT NULL
);


--
-- Name: lop_inds_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.lop_inds_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: lop_inds_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.lop_inds_id_seq OWNED BY public.lop_inds.id;


--
-- Name: lop_phuthu; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lop_phuthu (
    lopid integer NOT NULL,
    tuthang character varying(7) NOT NULL,
    sotien integer DEFAULT 0 NOT NULL,
    lydo character varying(300),
    nhanvienid integer,
    ngaytao timestamp(0) without time zone DEFAULT (date_trunc('second'::text, now()))::timestamp without time zone NOT NULL
);


--
-- Name: lophoc; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.lophoc (
    lopid integer NOT NULL,
    tenlop character varying(100),
    giaovienid integer,
    monid integer,
    khoilop integer,
    hocphi integer,
    hocphi_specialmonth character varying(500),
    ngaybatdau date,
    ngayketthuc date,
    tkb character varying(300),
    sltoida integer,
    tilechia integer,
    mucid integer,
    sobuoithang integer DEFAULT 8 NOT NULL,
    hoatdong boolean DEFAULT true NOT NULL,
    luutru timestamp(0) without time zone
);


--
-- Name: luong_dieuchinh; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.luong_dieuchinh (
    id integer NOT NULL,
    giaovienid integer NOT NULL,
    thang character varying(7) NOT NULL,
    lopid integer NOT NULL,
    sotien bigint NOT NULL,
    lydo character varying(500) NOT NULL,
    nhanvienid integer,
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL
);


--
-- Name: luong_dieuchinh_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.luong_dieuchinh_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: luong_dieuchinh_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.luong_dieuchinh_id_seq OWNED BY public.luong_dieuchinh.id;


--
-- Name: luong_ghichu; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.luong_ghichu (
    giaovienid integer NOT NULL,
    thang character varying(7) NOT NULL,
    khoa character varying(30) NOT NULL,
    ghichu character varying(500) NOT NULL,
    nhanvienid integer,
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL
);


--
-- Name: luong_khautru; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.luong_khautru (
    id integer NOT NULL,
    giaovienid integer NOT NULL,
    thang character varying(7) NOT NULL,
    loai character varying(8) NOT NULL,
    sotien bigint NOT NULL,
    lydo character varying(500) NOT NULL,
    nhanvienid integer,
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL,
    phieuchiid integer,
    CONSTRAINT luong_khautru_loai_check CHECK (((loai)::text = ANY ((ARRAY['tamung'::character varying, 'khac'::character varying])::text[]))),
    CONSTRAINT luong_khautru_sotien_check CHECK ((sotien > 0))
);


--
-- Name: luong_khautru_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.luong_khautru_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: luong_khautru_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.luong_khautru_id_seq OWNED BY public.luong_khautru.id;


--
-- Name: luotbangiao; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.luotbangiao (
    luotid integer NOT NULL,
    ngay date NOT NULL,
    stt integer NOT NULL,
    thoigian timestamp(0) without time zone DEFAULT now() NOT NULL,
    nhanvienid integer,
    tienmat bigint DEFAULT 0 NOT NULL,
    sophieutm integer DEFAULT 0 NOT NULL,
    chuyenkhoan bigint DEFAULT 0 NOT NULL,
    sophieuck integer DEFAULT 0 NOT NULL,
    chi bigint DEFAULT 0 NOT NULL,
    sophieuchi integer DEFAULT 0 NOT NULL,
    tienmatcu bigint DEFAULT 0 NOT NULL,
    sophieucu integer DEFAULT 0 NOT NULL,
    chicu bigint DEFAULT 0 NOT NULL,
    phainop bigint DEFAULT 0 NOT NULL,
    thucnop bigint DEFAULT 0 NOT NULL,
    ghichu character varying(500),
    khoitao boolean DEFAULT false NOT NULL,
    chuyenkhoancu bigint DEFAULT 0 NOT NULL,
    trangthai character varying(4) DEFAULT 'xong'::character varying NOT NULL,
    nhanvienid2 integer,
    thoigian2 timestamp(0) without time zone,
    thucnop2 bigint,
    ghichu2 character varying(500)
);


--
-- Name: luotbangiao_luotid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.luotbangiao_luotid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: luotbangiao_luotid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.luotbangiao_luotid_seq OWNED BY public.luotbangiao.luotid;


--
-- Name: mauthuchi; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.mauthuchi (
    mauid integer NOT NULL,
    loai character varying(4) NOT NULL,
    ten character varying(150) NOT NULL,
    lido character varying(150),
    sotien bigint,
    hinhthuc character varying(2),
    nguoi character varying(200),
    ghichu character varying(500),
    lopid integer,
    thutu integer DEFAULT 0 NOT NULL,
    CONSTRAINT mauthuchi_loai_check CHECK (((loai)::text = ANY ((ARRAY['thu'::character varying, 'chi'::character varying])::text[])))
);


--
-- Name: mauthuchi_mauid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.mauthuchi_mauid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: mauthuchi_mauid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.mauthuchi_mauid_seq OWNED BY public.mauthuchi.mauid;


--
-- Name: monhoc; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.monhoc (
    monid integer NOT NULL,
    tenmon character varying(50)
);


--
-- Name: muchocphi; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.muchocphi (
    mucid integer NOT NULL,
    tenmuc character varying(100) NOT NULL,
    sotien integer NOT NULL,
    ghichu character varying(200)
);


--
-- Name: nhantienthungan; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.nhantienthungan (
    lineid integer NOT NULL,
    tennguoinhan character varying(200),
    sotien integer,
    ghichu character varying(500),
    thoigian timestamp(3) without time zone
);


--
-- Name: nhanvien; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.nhanvien (
    nhanvienid integer NOT NULL,
    hotennv character varying(100) NOT NULL,
    sdt character varying(100),
    diachi character varying(100),
    chucvu integer NOT NULL,
    matkhau character varying(200),
    luong integer,
    quyen character varying(200) DEFAULT ''::character varying NOT NULL,
    an text DEFAULT ''::text NOT NULL,
    hoten character varying(100),
    daxoa timestamp(0) without time zone
);


--
-- Name: nhatky; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.nhatky (
    id integer NOT NULL,
    thoigian timestamp(0) without time zone DEFAULT now() NOT NULL,
    nhanvienid integer,
    hanhdong character varying(20) NOT NULL,
    hocsinhid integer,
    sotien bigint,
    noidung character varying(500),
    doituong text[],
    truoc jsonb,
    sau jsonb,
    lydo character varying(300),
    canxem boolean DEFAULT false NOT NULL,
    daxemluc timestamp(0) without time zone,
    daxemnv integer
);


--
-- Name: nhatky_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.nhatky_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: nhatky_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.nhatky_id_seq OWNED BY public.nhatky.id;


--
-- Name: notreo; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.notreo (
    id integer NOT NULL,
    hocsinhid integer NOT NULL,
    lineid integer,
    lopid integer,
    tenlop character varying(200),
    hotenhs character varying(200),
    mahocvien character varying(20),
    ngaysinh character varying(4),
    sdths character varying(20),
    sdtph character varying(20),
    sdt3 character varying(20),
    kyids text DEFAULT ''::text NOT NULL,
    thang text DEFAULT ''::text NOT NULL,
    sotien bigint DEFAULT 0 NOT NULL,
    ngaytao timestamp(0) without time zone DEFAULT now() NOT NULL,
    nhanvienid integer,
    ghichu character varying(500),
    trangthai character varying(10) DEFAULT 'treo'::character varying NOT NULL,
    ngayxuly timestamp(0) without time zone,
    nvxuly integer
);


--
-- Name: notreo_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.notreo_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: notreo_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.notreo_id_seq OWNED BY public.notreo.id;


--
-- Name: phancongnvpt; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.phancongnvpt (
    id integer NOT NULL,
    giaovienid integer NOT NULL,
    nvpt character varying(50) NOT NULL,
    tungay date NOT NULL,
    nhanvienid integer,
    ghichu text,
    thoigian timestamp(0) without time zone DEFAULT date_trunc('second'::text, now()) NOT NULL,
    nvpt_id integer
);


--
-- Name: phancongnvpt_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.phancongnvpt_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: phancongnvpt_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.phancongnvpt_id_seq OWNED BY public.phancongnvpt.id;


--
-- Name: phieuchi; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.phieuchi (
    lineid integer NOT NULL,
    tennguoinhan character varying(200),
    lido character varying(150),
    sotien bigint,
    ghichu character varying(500),
    thoigian timestamp(3) without time zone,
    lopid integer,
    giaovienid integer,
    luotbangiao integer,
    hinhthuc character varying(2),
    hocsinhid integer,
    phieuthuid integer,
    chotluongid integer,
    nhanvienid integer,
    duyet character varying(8) DEFAULT 'cho'::character varying NOT NULL,
    duyet_nv integer,
    duyet_luc timestamp(0) without time zone,
    duyet_lydo character varying(300)
);


--
-- Name: phieuthukhac; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.phieuthukhac (
    lineid integer NOT NULL,
    thoigiandong timestamp(0) without time zone NOT NULL,
    sotien bigint NOT NULL,
    hinhthuc character varying(2) DEFAULT 'TM'::character varying NOT NULL,
    lido character varying(150),
    nguoinop character varying(200),
    ghichu character varying(500),
    lopid integer,
    hocsinhid integer,
    nhanvienid integer,
    luotbangiao integer
);


--
-- Name: phieuthukhac_lineid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.phieuthukhac_lineid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: phieuthukhac_lineid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.phieuthukhac_lineid_seq OWNED BY public.phieuthukhac.lineid;


--
-- Name: phieuthutien; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.phieuthutien (
    lineid integer NOT NULL,
    lopid integer NOT NULL,
    hocsinhid integer,
    sotien integer,
    thangdongtien character varying(200),
    thoigiandong timestamp(3) without time zone,
    nhanvienid integer,
    datraluong integer,
    idtinhluong integer,
    ghichu character varying(500),
    kyid integer,
    hinhthuc character varying(2) DEFAULT 'TM'::character varying NOT NULL,
    ycid integer,
    luotbangiao integer,
    chotluongid integer
);


--
-- Name: phonghoc; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.phonghoc (
    lineid integer NOT NULL,
    tenphong character varying(30),
    ghichu character varying(500)
);


--
-- Name: recordtinhluonggv; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.recordtinhluonggv (
    lineid integer NOT NULL,
    thoigian timestamp(3) without time zone,
    giaovienid integer,
    lopid integer,
    hpdathu integer,
    siso integer,
    tileluongconst integer,
    tienluong integer,
    thangdongtien character varying(20)
);


--
-- Name: taikhoan; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.taikhoan (
    tienhientai bigint,
    tientrongket bigint
);


--
-- Name: thongtintt; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.thongtintt (
    tentrungtam character varying(50),
    sdt character varying(20),
    diachi character varying(100),
    tilechia integer,
    website character varying(100)
);


--
-- Name: tinhtileluong; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tinhtileluong (
    lineid integer NOT NULL,
    ssmin integer,
    ssmax integer,
    tileluongconst integer
);


--
-- Name: tkb_tamthoi; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tkb_tamthoi (
    id integer NOT NULL,
    lopid integer NOT NULL,
    ngay date NOT NULL,
    loai character varying(8) NOT NULL,
    bd character varying(5),
    kt character varying(5),
    lydo character varying(300),
    nhom integer,
    nhanvienid integer,
    thoigian timestamp(0) without time zone DEFAULT now() NOT NULL,
    CONSTRAINT tkb_tamthoi_loai_check CHECK (((loai)::text = ANY ((ARRAY['nghi'::character varying, 'them'::character varying])::text[])))
);


--
-- Name: tkb_tamthoi_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.tkb_tamthoi_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: tkb_tamthoi_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.tkb_tamthoi_id_seq OWNED BY public.tkb_tamthoi.id;


--
-- Name: tkb_xepphong; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tkb_xepphong (
    id integer NOT NULL,
    lopid integer NOT NULL,
    thu character varying(2) NOT NULL,
    bd character varying(5) NOT NULL,
    ngay date,
    phong integer,
    nhanvienid integer,
    thoigian timestamp(0) without time zone DEFAULT now() NOT NULL
);


--
-- Name: tkb_xepphong_id_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.tkb_xepphong_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: tkb_xepphong_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.tkb_xepphong_id_seq OWNED BY public.tkb_xepphong.id;


--
-- Name: tongketthang; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.tongketthang (
    lineid integer NOT NULL,
    thangtongket date,
    tongthu bigint,
    tongluongnv bigint,
    tongchiagiaovien bigint,
    tonghoantien bigint,
    muckhac bigint,
    thue bigint,
    phihangthang bigint,
    phimatbang bigint,
    loinhuan bigint,
    ngaytongket timestamp(3) without time zone
);


--
-- Name: yeucau_ck; Type: TABLE; Schema: public; Owner: -
--

CREATE TABLE public.yeucau_ck (
    ycid integer NOT NULL,
    ma character varying(30),
    hocsinhid integer NOT NULL,
    items jsonb NOT NULL,
    tongtien integer NOT NULL,
    trangthai character varying(10) DEFAULT 'cho'::character varying NOT NULL,
    nhanvienid integer,
    ngaytao timestamp(0) without time zone DEFAULT now() NOT NULL,
    ngayxong timestamp(0) without time zone,
    gdid integer,
    CONSTRAINT yeucau_ck_trangthai_check CHECK (((trangthai)::text = ANY ((ARRAY['cho'::character varying, 'xong'::character varying, 'huy'::character varying])::text[])))
);


--
-- Name: yeucau_ck_ycid_seq; Type: SEQUENCE; Schema: public; Owner: -
--

CREATE SEQUENCE public.yeucau_ck_ycid_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


--
-- Name: yeucau_ck_ycid_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: -
--

ALTER SEQUENCE public.yeucau_ck_ycid_seq OWNED BY public.yeucau_ck.ycid;


--
-- Name: baocaotoi id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.baocaotoi ALTER COLUMN id SET DEFAULT nextval('public.baocaotoi_id_seq'::regclass);


--
-- Name: chotluong chotid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong ALTER COLUMN chotid SET DEFAULT nextval('public.chotluong_chotid_seq'::regclass);


--
-- Name: chotluong_dong id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong_dong ALTER COLUMN id SET DEFAULT nextval('public.chotluong_dong_id_seq'::regclass);


--
-- Name: chotluong_khautru id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong_khautru ALTER COLUMN id SET DEFAULT nextval('public.chotluong_khautru_id_seq'::regclass);


--
-- Name: chuyenluong_dong id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chuyenluong_dong ALTER COLUMN id SET DEFAULT nextval('public.chuyenluong_dong_id_seq'::regclass);


--
-- Name: chuyenluong_lo loid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chuyenluong_lo ALTER COLUMN loid SET DEFAULT nextval('public.chuyenluong_lo_loid_seq'::regclass);


--
-- Name: dinhkem_cu id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dinhkem_cu ALTER COLUMN id SET DEFAULT nextval('public.dinhkem_cu_id_seq'::regclass);


--
-- Name: giaodich_ck gdid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.giaodich_ck ALTER COLUMN gdid SET DEFAULT nextval('public.giaodich_ck_gdid_seq'::regclass);


--
-- Name: giaovien_lichsu id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.giaovien_lichsu ALTER COLUMN id SET DEFAULT nextval('public.giaovien_lichsu_id_seq'::regclass);


--
-- Name: hoantra id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.hoantra ALTER COLUMN id SET DEFAULT nextval('public.hoantra_id_seq'::regclass);


--
-- Name: kb_bang bangid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_bang ALTER COLUMN bangid SET DEFAULT nextval('public.kb_bang_bangid_seq'::regclass);


--
-- Name: kb_binhluan blid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_binhluan ALTER COLUMN blid SET DEFAULT nextval('public.kb_binhluan_blid_seq'::regclass);


--
-- Name: kb_cot cotid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_cot ALTER COLUMN cotid SET DEFAULT nextval('public.kb_cot_cotid_seq'::regclass);


--
-- Name: kb_lichsu id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_lichsu ALTER COLUMN id SET DEFAULT nextval('public.kb_lichsu_id_seq'::regclass);


--
-- Name: kb_tep tepid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_tep ALTER COLUMN tepid SET DEFAULT nextval('public.kb_tep_tepid_seq'::regclass);


--
-- Name: kb_the theid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_the ALTER COLUMN theid SET DEFAULT nextval('public.kb_the_theid_seq'::regclass);


--
-- Name: kb_viec viecid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_viec ALTER COLUMN viecid SET DEFAULT nextval('public.kb_viec_viecid_seq'::regclass);


--
-- Name: kyhocphi kyid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kyhocphi ALTER COLUMN kyid SET DEFAULT nextval('public.kyhocphi_kyid_seq'::regclass);


--
-- Name: lichchung lichid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lichchung ALTER COLUMN lichid SET DEFAULT nextval('public.lichchung_lichid_seq'::regclass);


--
-- Name: lidothuchi id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lidothuchi ALTER COLUMN id SET DEFAULT nextval('public.lidothuchi_id_seq'::regclass);


--
-- Name: lop_inds id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lop_inds ALTER COLUMN id SET DEFAULT nextval('public.lop_inds_id_seq'::regclass);


--
-- Name: luong_dieuchinh id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luong_dieuchinh ALTER COLUMN id SET DEFAULT nextval('public.luong_dieuchinh_id_seq'::regclass);


--
-- Name: luong_khautru id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luong_khautru ALTER COLUMN id SET DEFAULT nextval('public.luong_khautru_id_seq'::regclass);


--
-- Name: luotbangiao luotid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luotbangiao ALTER COLUMN luotid SET DEFAULT nextval('public.luotbangiao_luotid_seq'::regclass);


--
-- Name: mauthuchi mauid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mauthuchi ALTER COLUMN mauid SET DEFAULT nextval('public.mauthuchi_mauid_seq'::regclass);


--
-- Name: nhatky id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.nhatky ALTER COLUMN id SET DEFAULT nextval('public.nhatky_id_seq'::regclass);


--
-- Name: notreo id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.notreo ALTER COLUMN id SET DEFAULT nextval('public.notreo_id_seq'::regclass);


--
-- Name: phancongnvpt id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.phancongnvpt ALTER COLUMN id SET DEFAULT nextval('public.phancongnvpt_id_seq'::regclass);


--
-- Name: phieuthukhac lineid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.phieuthukhac ALTER COLUMN lineid SET DEFAULT nextval('public.phieuthukhac_lineid_seq'::regclass);


--
-- Name: tkb_tamthoi id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tkb_tamthoi ALTER COLUMN id SET DEFAULT nextval('public.tkb_tamthoi_id_seq'::regclass);


--
-- Name: tkb_xepphong id; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tkb_xepphong ALTER COLUMN id SET DEFAULT nextval('public.tkb_xepphong_id_seq'::regclass);


--
-- Name: yeucau_ck ycid; Type: DEFAULT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.yeucau_ck ALTER COLUMN ycid SET DEFAULT nextval('public.yeucau_ck_ycid_seq'::regclass);


--
-- Name: app_meta app_meta_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.app_meta
    ADD CONSTRAINT app_meta_pkey PRIMARY KEY (key);


--
-- Name: aspnetroles aspnetroles_name_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aspnetroles
    ADD CONSTRAINT aspnetroles_name_key UNIQUE (name);


--
-- Name: aspnetroles aspnetroles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aspnetroles
    ADD CONSTRAINT aspnetroles_pkey PRIMARY KEY (id);


--
-- Name: aspnetuserroles aspnetuserroles_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aspnetuserroles
    ADD CONSTRAINT aspnetuserroles_pkey PRIMARY KEY (userid, roleid);


--
-- Name: aspnetusers aspnetusers_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aspnetusers
    ADD CONSTRAINT aspnetusers_pkey PRIMARY KEY (id);


--
-- Name: aspnetusers aspnetusers_username_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.aspnetusers
    ADD CONSTRAINT aspnetusers_username_key UNIQUE (username);


--
-- Name: bangiao bangiao_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bangiao
    ADD CONSTRAINT bangiao_pkey PRIMARY KEY (ngay);


--
-- Name: bangtinhhp bangtinhhp_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.bangtinhhp
    ADD CONSTRAINT bangtinhhp_pkey PRIMARY KEY (lineid);


--
-- Name: baocaotoi baocaotoi_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.baocaotoi
    ADD CONSTRAINT baocaotoi_pkey PRIMARY KEY (id);


--
-- Name: cahoc cahoc_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.cahoc
    ADD CONSTRAINT cahoc_pkey PRIMARY KEY (caid);


--
-- Name: capnhat_lichsu capnhat_lichsu_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.capnhat_lichsu
    ADD CONSTRAINT capnhat_lichsu_pkey PRIMARY KEY (build);


--
-- Name: capnhat capnhat_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.capnhat
    ADD CONSTRAINT capnhat_pkey PRIMARY KEY (build);


--
-- Name: chamconggv chamconggv_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chamconggv
    ADD CONSTRAINT chamconggv_pkey PRIMARY KEY (lineid);


--
-- Name: chotluong_dong chotluong_dong_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong_dong
    ADD CONSTRAINT chotluong_dong_pkey PRIMARY KEY (id);


--
-- Name: chotluong chotluong_giaovienid_thang_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong
    ADD CONSTRAINT chotluong_giaovienid_thang_key UNIQUE (giaovienid, thang);


--
-- Name: chotluong_khautru chotluong_khautru_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong_khautru
    ADD CONSTRAINT chotluong_khautru_pkey PRIMARY KEY (id);


--
-- Name: chotluong_lop chotluong_lop_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong_lop
    ADD CONSTRAINT chotluong_lop_pkey PRIMARY KEY (chotid, lopid);


--
-- Name: chotluong chotluong_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chotluong
    ADD CONSTRAINT chotluong_pkey PRIMARY KEY (chotid);


--
-- Name: chuyenluong_dong chuyenluong_dong_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chuyenluong_dong
    ADD CONSTRAINT chuyenluong_dong_pkey PRIMARY KEY (id);


--
-- Name: chuyenluong_lo chuyenluong_lo_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chuyenluong_lo
    ADD CONSTRAINT chuyenluong_lo_pkey PRIMARY KEY (loid);


--
-- Name: danhsachdanghoc danhsachdanghoc_hocsinhid_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.danhsachdanghoc
    ADD CONSTRAINT danhsachdanghoc_hocsinhid_key UNIQUE (hocsinhid);


--
-- Name: danhsachlop danhsachlop_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.danhsachlop
    ADD CONSTRAINT danhsachlop_pkey PRIMARY KEY (lineid);


--
-- Name: diemdanh diemdanh_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.diemdanh
    ADD CONSTRAINT diemdanh_pkey PRIMARY KEY (lopid, hocsinhid, ngay);


--
-- Name: dinhkem_cu dinhkem_cu_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dinhkem_cu
    ADD CONSTRAINT dinhkem_cu_pkey PRIMARY KEY (id);


--
-- Name: dinhkem dinhkem_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.dinhkem
    ADD CONSTRAINT dinhkem_pkey PRIMARY KEY (loai, lineid);


--
-- Name: doichieuchucvu doichieuchucvu_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.doichieuchucvu
    ADD CONSTRAINT doichieuchucvu_pkey PRIMARY KEY (chucvu);


--
-- Name: giaodich_ck giaodich_ck_magd_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.giaodich_ck
    ADD CONSTRAINT giaodich_ck_magd_key UNIQUE (magd);


--
-- Name: giaodich_ck giaodich_ck_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.giaodich_ck
    ADD CONSTRAINT giaodich_ck_pkey PRIMARY KEY (gdid);


--
-- Name: giaovien giaovien_hotengv_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.giaovien
    ADD CONSTRAINT giaovien_hotengv_key UNIQUE (hotengv);


--
-- Name: giaovien_lichsu giaovien_lichsu_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.giaovien_lichsu
    ADD CONSTRAINT giaovien_lichsu_pkey PRIMARY KEY (id);


--
-- Name: giaovien giaovien_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.giaovien
    ADD CONSTRAINT giaovien_pkey PRIMARY KEY (giaovienid);


--
-- Name: hoantra hoantra_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.hoantra
    ADD CONSTRAINT hoantra_pkey PRIMARY KEY (id);


--
-- Name: hocsinh_giam hocsinh_giam_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.hocsinh_giam
    ADD CONSTRAINT hocsinh_giam_pkey PRIMARY KEY (hocsinhid, monid);


--
-- Name: hocsinh hocsinh_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.hocsinh
    ADD CONSTRAINT hocsinh_pkey PRIMARY KEY (hocsinhid);


--
-- Name: kb_bang kb_bang_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_bang
    ADD CONSTRAINT kb_bang_pkey PRIMARY KEY (bangid);


--
-- Name: kb_binhluan kb_binhluan_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_binhluan
    ADD CONSTRAINT kb_binhluan_pkey PRIMARY KEY (blid);


--
-- Name: kb_cot kb_cot_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_cot
    ADD CONSTRAINT kb_cot_pkey PRIMARY KEY (cotid);


--
-- Name: kb_lichsu kb_lichsu_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_lichsu
    ADD CONSTRAINT kb_lichsu_pkey PRIMARY KEY (id);


--
-- Name: kb_tep kb_tep_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_tep
    ADD CONSTRAINT kb_tep_pkey PRIMARY KEY (tepid);


--
-- Name: kb_thanhvien kb_thanhvien_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_thanhvien
    ADD CONSTRAINT kb_thanhvien_pkey PRIMARY KEY (bangid, nv);


--
-- Name: kb_the kb_the_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_the
    ADD CONSTRAINT kb_the_pkey PRIMARY KEY (theid);


--
-- Name: kb_viec kb_viec_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_viec
    ADD CONSTRAINT kb_viec_pkey PRIMARY KEY (viecid);


--
-- Name: kydangky kydangky_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kydangky
    ADD CONSTRAINT kydangky_pkey PRIMARY KEY (thang);


--
-- Name: kyhocphi kyhocphi_makyhp_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kyhocphi
    ADD CONSTRAINT kyhocphi_makyhp_key UNIQUE (makyhp);


--
-- Name: kyhocphi kyhocphi_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kyhocphi
    ADD CONSTRAINT kyhocphi_pkey PRIMARY KEY (kyid);


--
-- Name: lichchung lichchung_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lichchung
    ADD CONSTRAINT lichchung_pkey PRIMARY KEY (lichid);


--
-- Name: lichnghigv lichnghigv_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lichnghigv
    ADD CONSTRAINT lichnghigv_pkey PRIMARY KEY (lineid);


--
-- Name: lidothuchi lidothuchi_loai_ten_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lidothuchi
    ADD CONSTRAINT lidothuchi_loai_ten_key UNIQUE (loai, ten);


--
-- Name: lidothuchi lidothuchi_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lidothuchi
    ADD CONSTRAINT lidothuchi_pkey PRIMARY KEY (id);


--
-- Name: lop_daxem lop_daxem_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lop_daxem
    ADD CONSTRAINT lop_daxem_pkey PRIMARY KEY (nhanvienid, lopid);


--
-- Name: lop_inds lop_inds_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lop_inds
    ADD CONSTRAINT lop_inds_pkey PRIMARY KEY (id);


--
-- Name: lop_phuthu lop_phuthu_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lop_phuthu
    ADD CONSTRAINT lop_phuthu_pkey PRIMARY KEY (lopid, tuthang);


--
-- Name: lophoc lophoc_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.lophoc
    ADD CONSTRAINT lophoc_pkey PRIMARY KEY (lopid);


--
-- Name: luong_dieuchinh luong_dieuchinh_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luong_dieuchinh
    ADD CONSTRAINT luong_dieuchinh_pkey PRIMARY KEY (id);


--
-- Name: luong_ghichu luong_ghichu_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luong_ghichu
    ADD CONSTRAINT luong_ghichu_pkey PRIMARY KEY (giaovienid, thang, khoa);


--
-- Name: luong_khautru luong_khautru_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luong_khautru
    ADD CONSTRAINT luong_khautru_pkey PRIMARY KEY (id);


--
-- Name: luotbangiao luotbangiao_ngay_stt_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luotbangiao
    ADD CONSTRAINT luotbangiao_ngay_stt_key UNIQUE (ngay, stt);


--
-- Name: luotbangiao luotbangiao_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.luotbangiao
    ADD CONSTRAINT luotbangiao_pkey PRIMARY KEY (luotid);


--
-- Name: mauthuchi mauthuchi_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.mauthuchi
    ADD CONSTRAINT mauthuchi_pkey PRIMARY KEY (mauid);


--
-- Name: monhoc monhoc_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.monhoc
    ADD CONSTRAINT monhoc_pkey PRIMARY KEY (monid);


--
-- Name: muchocphi muchocphi_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.muchocphi
    ADD CONSTRAINT muchocphi_pkey PRIMARY KEY (mucid);


--
-- Name: muchocphi muchocphi_tenmuc_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.muchocphi
    ADD CONSTRAINT muchocphi_tenmuc_key UNIQUE (tenmuc);


--
-- Name: nhantienthungan nhantienthungan_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.nhantienthungan
    ADD CONSTRAINT nhantienthungan_pkey PRIMARY KEY (lineid);


--
-- Name: nhanvien nhanvien_hotennv_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.nhanvien
    ADD CONSTRAINT nhanvien_hotennv_key UNIQUE (hotennv);


--
-- Name: nhanvien nhanvien_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.nhanvien
    ADD CONSTRAINT nhanvien_pkey PRIMARY KEY (nhanvienid);


--
-- Name: nhatky nhatky_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.nhatky
    ADD CONSTRAINT nhatky_pkey PRIMARY KEY (id);


--
-- Name: notreo notreo_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.notreo
    ADD CONSTRAINT notreo_pkey PRIMARY KEY (id);


--
-- Name: phancongnvpt phancongnvpt_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.phancongnvpt
    ADD CONSTRAINT phancongnvpt_pkey PRIMARY KEY (id);


--
-- Name: phieuchi phieuchi_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.phieuchi
    ADD CONSTRAINT phieuchi_pkey PRIMARY KEY (lineid);


--
-- Name: phieuthukhac phieuthukhac_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.phieuthukhac
    ADD CONSTRAINT phieuthukhac_pkey PRIMARY KEY (lineid);


--
-- Name: phieuthutien phieuthutien_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.phieuthutien
    ADD CONSTRAINT phieuthutien_pkey PRIMARY KEY (lineid);


--
-- Name: phonghoc phonghoc_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.phonghoc
    ADD CONSTRAINT phonghoc_pkey PRIMARY KEY (lineid);


--
-- Name: recordtinhluonggv recordtinhluonggv_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.recordtinhluonggv
    ADD CONSTRAINT recordtinhluonggv_pkey PRIMARY KEY (lineid);


--
-- Name: tinhtileluong tinhtileluong_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tinhtileluong
    ADD CONSTRAINT tinhtileluong_pkey PRIMARY KEY (lineid);


--
-- Name: tkb_tamthoi tkb_tamthoi_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tkb_tamthoi
    ADD CONSTRAINT tkb_tamthoi_pkey PRIMARY KEY (id);


--
-- Name: tkb_xepphong tkb_xepphong_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tkb_xepphong
    ADD CONSTRAINT tkb_xepphong_pkey PRIMARY KEY (id);


--
-- Name: tongketthang tongketthang_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.tongketthang
    ADD CONSTRAINT tongketthang_pkey PRIMARY KEY (lineid);


--
-- Name: kyhocphi ux_kyhocphi_hs_lop_thang; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kyhocphi
    ADD CONSTRAINT ux_kyhocphi_hs_lop_thang UNIQUE (hocsinhid, lopid, thang);


--
-- Name: yeucau_ck yeucau_ck_ma_key; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.yeucau_ck
    ADD CONSTRAINT yeucau_ck_ma_key UNIQUE (ma);


--
-- Name: yeucau_ck yeucau_ck_pkey; Type: CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.yeucau_ck
    ADD CONSTRAINT yeucau_ck_pkey PRIMARY KEY (ycid);


--
-- Name: ix_chamconggv_gv_ngay; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_chamconggv_gv_ngay ON public.chamconggv USING btree (giaovienid, ngaycham);


--
-- Name: ix_chotluong_dong; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_chotluong_dong ON public.chotluong_dong USING btree (chotid);


--
-- Name: ix_chotluong_khautru; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_chotluong_khautru ON public.chotluong_khautru USING btree (chotid);


--
-- Name: ix_chuyenluong_dong_chot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_chuyenluong_dong_chot ON public.chuyenluong_dong USING btree (chotid);


--
-- Name: ix_danhsachlop_hocsinh; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_danhsachlop_hocsinh ON public.danhsachlop USING btree (hocsinhid);


--
-- Name: ix_danhsachlop_hs_lop; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_danhsachlop_hs_lop ON public.danhsachlop USING btree (hocsinhid, lopid);


--
-- Name: ix_danhsachlop_lop; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_danhsachlop_lop ON public.danhsachlop USING btree (lopid);


--
-- Name: ix_diemdanh_ngay; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_diemdanh_ngay ON public.diemdanh USING btree (ngay);


--
-- Name: ix_dinhkem_cu; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_dinhkem_cu ON public.dinhkem_cu USING btree (loai, lineid);


--
-- Name: ix_dsl_ngaynghi; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_dsl_ngaynghi ON public.danhsachlop USING btree (ngaynghi) WHERE (ngaynghi IS NOT NULL);


--
-- Name: ix_dsl_ngaythem; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_dsl_ngaythem ON public.danhsachlop USING btree (ngaythem) WHERE (ngaythem IS NOT NULL);


--
-- Name: ix_giaodich_ck_nhan; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_giaodich_ck_nhan ON public.giaodich_ck USING btree (nhanluc DESC);


--
-- Name: ix_giaovien_lichsu; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_giaovien_lichsu ON public.giaovien_lichsu USING btree (giaovienid);


--
-- Name: ix_hoantra_phieu; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_hoantra_phieu ON public.hoantra USING btree (phieuid) WHERE giuphieu;


--
-- Name: ix_kb_the_bang; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_kb_the_bang ON public.kb_the USING btree (bangid, cotid);


--
-- Name: ix_kb_the_nhan; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_kb_the_nhan ON public.kb_the USING btree (nguoinhan) WHERE (luutru IS NULL);


--
-- Name: ix_kyhocphi_lop; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_kyhocphi_lop ON public.kyhocphi USING btree (lopid);


--
-- Name: ix_kyhocphi_trangthai; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_kyhocphi_trangthai ON public.kyhocphi USING btree (trangthai);


--
-- Name: ix_lichchung_ngay; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_lichchung_ngay ON public.lichchung USING btree (ngay);


--
-- Name: ix_lichnghigv_gv; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_lichnghigv_gv ON public.lichnghigv USING btree (giaovienid);


--
-- Name: ix_lop_inds_lop; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_lop_inds_lop ON public.lop_inds USING btree (lopid);


--
-- Name: ix_lophoc_giaovien; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_lophoc_giaovien ON public.lophoc USING btree (giaovienid);


--
-- Name: ix_lophoc_ngayketthuc; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_lophoc_ngayketthuc ON public.lophoc USING btree (ngayketthuc);


--
-- Name: ix_luong_khautru; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_luong_khautru ON public.luong_khautru USING btree (giaovienid, thang);


--
-- Name: ix_nhatky_canxem; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_nhatky_canxem ON public.nhatky USING btree (thoigian) WHERE (canxem AND (daxemluc IS NULL));


--
-- Name: ix_nhatky_doituong; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_nhatky_doituong ON public.nhatky USING gin (doituong);


--
-- Name: ix_nhatky_tg; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_nhatky_tg ON public.nhatky USING btree (thoigian DESC);


--
-- Name: ix_phancongnvpt_gv; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phancongnvpt_gv ON public.phancongnvpt USING btree (giaovienid, tungay);


--
-- Name: ix_phieuchi_cho; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuchi_cho ON public.phieuchi USING btree (duyet) WHERE ((duyet)::text = 'cho'::text);


--
-- Name: ix_phieuchi_luot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuchi_luot ON public.phieuchi USING btree (luotbangiao);


--
-- Name: ix_phieuchi_phieuthu; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuchi_phieuthu ON public.phieuchi USING btree (phieuthuid) WHERE (phieuthuid IS NOT NULL);


--
-- Name: ix_phieuchi_thoigian; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuchi_thoigian ON public.phieuchi USING btree (thoigian);


--
-- Name: ix_phieuthukhac_luot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthukhac_luot ON public.phieuthukhac USING btree (luotbangiao);


--
-- Name: ix_phieuthukhac_tg; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthukhac_tg ON public.phieuthukhac USING btree (thoigiandong);


--
-- Name: ix_phieuthutien_chotluong; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthutien_chotluong ON public.phieuthutien USING btree (chotluongid);


--
-- Name: ix_phieuthutien_chuachot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthutien_chuachot ON public.phieuthutien USING btree (lopid) WHERE (chotluongid IS NULL);


--
-- Name: ix_phieuthutien_chualuong; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthutien_chualuong ON public.phieuthutien USING btree (lopid) WHERE ((chotluongid IS NULL) AND (COALESCE(datraluong, 0) = 0));


--
-- Name: ix_phieuthutien_hocsinh; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthutien_hocsinh ON public.phieuthutien USING btree (hocsinhid);


--
-- Name: ix_phieuthutien_lop; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthutien_lop ON public.phieuthutien USING btree (lopid);


--
-- Name: ix_phieuthutien_luot; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthutien_luot ON public.phieuthutien USING btree (luotbangiao);


--
-- Name: ix_phieuthutien_thoigian; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_phieuthutien_thoigian ON public.phieuthutien USING btree (thoigiandong);


--
-- Name: ix_recordtinhluonggv_gv; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_recordtinhluonggv_gv ON public.recordtinhluonggv USING btree (giaovienid);


--
-- Name: ix_tkb_tamthoi_lop; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_tkb_tamthoi_lop ON public.tkb_tamthoi USING btree (lopid, ngay);


--
-- Name: ix_tkb_tamthoi_ngay; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_tkb_tamthoi_ngay ON public.tkb_tamthoi USING btree (ngay);


--
-- Name: ix_xepphong_ngay; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_xepphong_ngay ON public.tkb_xepphong USING btree (ngay);


--
-- Name: ix_yeucau_ck_hs; Type: INDEX; Schema: public; Owner: -
--

CREATE INDEX ix_yeucau_ck_hs ON public.yeucau_ck USING btree (hocsinhid, trangthai);


--
-- Name: ux_bangiao_anh_luot; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX ux_bangiao_anh_luot ON public.bangiao_anh USING btree (COALESCE(luotid, 0), vitri);


--
-- Name: ux_baocaotoi_tudong; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX ux_baocaotoi_tudong ON public.baocaotoi USING btree (ngay) WHERE tudong;


--
-- Name: ux_hocsinh_mahocvien; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX ux_hocsinh_mahocvien ON public.hocsinh USING btree (mahocvien);


--
-- Name: ux_xepphong_ngay; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX ux_xepphong_ngay ON public.tkb_xepphong USING btree (lopid, ngay, bd) WHERE (ngay IS NOT NULL);


--
-- Name: ux_xepphong_tuan; Type: INDEX; Schema: public; Owner: -
--

CREATE UNIQUE INDEX ux_xepphong_tuan ON public.tkb_xepphong USING btree (lopid, thu, bd) WHERE (ngay IS NULL);


--
-- Name: danhsachlop tg_dsl_biendong; Type: TRIGGER; Schema: public; Owner: -
--

CREATE TRIGGER tg_dsl_biendong BEFORE INSERT OR UPDATE OF ngaykthoc ON public.danhsachlop FOR EACH ROW EXECUTE FUNCTION public.tps_dsl_biendong();


--
-- Name: chuyenluong_dong chuyenluong_dong_loid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.chuyenluong_dong
    ADD CONSTRAINT chuyenluong_dong_loid_fkey FOREIGN KEY (loid) REFERENCES public.chuyenluong_lo(loid);


--
-- Name: kb_binhluan kb_binhluan_theid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_binhluan
    ADD CONSTRAINT kb_binhluan_theid_fkey FOREIGN KEY (theid) REFERENCES public.kb_the(theid) ON DELETE CASCADE;


--
-- Name: kb_cot kb_cot_bangid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_cot
    ADD CONSTRAINT kb_cot_bangid_fkey FOREIGN KEY (bangid) REFERENCES public.kb_bang(bangid) ON DELETE CASCADE;


--
-- Name: kb_lichsu kb_lichsu_theid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_lichsu
    ADD CONSTRAINT kb_lichsu_theid_fkey FOREIGN KEY (theid) REFERENCES public.kb_the(theid) ON DELETE CASCADE;


--
-- Name: kb_tep kb_tep_theid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_tep
    ADD CONSTRAINT kb_tep_theid_fkey FOREIGN KEY (theid) REFERENCES public.kb_the(theid) ON DELETE CASCADE;


--
-- Name: kb_thanhvien kb_thanhvien_bangid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_thanhvien
    ADD CONSTRAINT kb_thanhvien_bangid_fkey FOREIGN KEY (bangid) REFERENCES public.kb_bang(bangid) ON DELETE CASCADE;


--
-- Name: kb_the kb_the_bangid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_the
    ADD CONSTRAINT kb_the_bangid_fkey FOREIGN KEY (bangid) REFERENCES public.kb_bang(bangid) ON DELETE CASCADE;


--
-- Name: kb_the kb_the_cotid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_the
    ADD CONSTRAINT kb_the_cotid_fkey FOREIGN KEY (cotid) REFERENCES public.kb_cot(cotid);


--
-- Name: kb_viec kb_viec_theid_fkey; Type: FK CONSTRAINT; Schema: public; Owner: -
--

ALTER TABLE ONLY public.kb_viec
    ADD CONSTRAINT kb_viec_theid_fkey FOREIGN KEY (theid) REFERENCES public.kb_the(theid) ON DELETE CASCADE;


--
-- PostgreSQL database dump complete
--


