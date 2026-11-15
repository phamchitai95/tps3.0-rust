import { invoke } from "@tauri-apps/api/core";

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

async function api<T = Json>(name: string, args: Record<string, Json> = {}): Promise<T> {
  return invoke<T>("api_call", { name, args });
}

const app = document.querySelector<HTMLDivElement>("#app")!;

type Me = { hotennv?: string; tendangnhap?: string; chucvu?: number } | null;

let me: Me = null;
let tab: "hs" | "congno" | "ck" | "luong" | "hethong" = "hs";
let selectedHs: number | null = null;
let lastMsg = "";
let lastErr = "";
let busy = false;
let lastReceiptIds: number[] = [];
let lastReceiptText = "";

function money(n: number): string {
  return new Intl.NumberFormat("vi-VN").format(n) + " đ";
}

async function refreshMe() {
  try {
    me = await api<Me>("auth.me");
  } catch {
    me = null;
  }
}

function setStatus(ok: string, err = "") {
  lastMsg = ok;
  lastErr = err;
}

async function withBusy(fn: () => Promise<void>) {
  if (busy) return;
  busy = true;
  render();
  try {
    await fn();
  } catch (e) {
    setStatus("", String(e));
  } finally {
    busy = false;
    render();
  }
}

async function doLogin(user: string, pass: string) {
  await withBusy(async () => {
    await api("auth.login", { tendangnhap: user, matkhau: pass });
    await refreshMe();
    setStatus("Đăng nhập thành công.");
  });
}

async function doLogout() {
  await withBusy(async () => {
    await api("auth.logout");
    me = null;
    selectedHs = null;
    setStatus("Đã đăng xuất.");
  });
}

function loginView() {
  app.innerHTML = `
    <div class="shell">
      <p class="brand">TPS 3.0</p>
      <p class="tagline">Quản lý học phí trung tâm — Rust + Tauri. Đăng nhập bằng tài khoản nhân viên.</p>
      <form class="panel" id="login">
        <div class="row">
          <label>Tên đăng nhập
            <input name="user" autocomplete="username" value="admin" required />
          </label>
          <label>Mật khẩu
            <input name="pass" type="password" autocomplete="current-password" value="admin" required />
          </label>
        </div>
        <div class="row" style="margin-top:1rem">
          <button type="submit" ${busy ? "disabled" : ""}>Đăng nhập</button>
        </div>
        ${lastErr ? `<p class="err">${escapeHtml(lastErr)}</p>` : ""}
        ${lastMsg ? `<p class="ok">${escapeHtml(lastMsg)}</p>` : ""}
      </form>
    </div>
  `;
  document.querySelector<HTMLFormElement>("#login")!.addEventListener("submit", (ev) => {
    ev.preventDefault();
    const fd = new FormData(ev.target as HTMLFormElement);
    void doLogin(String(fd.get("user") || ""), String(fd.get("pass") || ""));
  });
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

async function loadHs(): Promise<Json[]> {
  const r = await api<Json>("hocsinh.list", {});
  return Array.isArray(r) ? r : [];
}

async function loadDebt(hsid: number): Promise<Json[]> {
  const r = await api<Json>("congno.list", { hocsinhid: hsid });
  return Array.isArray(r) ? r : [];
}

async function loadGd(): Promise<Json[]> {
  const r = await api<Json>("ck.gdList", { limit: 30 });
  return Array.isArray(r) ? r : [];
}

async function shellView() {
  const name = (me && (me.hotennv || me.tendangnhap)) || "—";
  app.innerHTML = `
    <div class="shell">
      <p class="brand">TPS 3.0</p>
      <div class="userbar">
        <p class="tagline">Xin chào, <strong>${escapeHtml(String(name))}</strong></p>
        <button class="ghost" type="button" id="logout">Đăng xuất</button>
      </div>
      <div class="nav">
        <button type="button" data-tab="hs" class="${tab === "hs" ? "active" : ""}">Học viên</button>
        <button type="button" data-tab="congno" class="${tab === "congno" ? "active" : ""}">Công nợ / Thu TM</button>
        <button type="button" data-tab="ck" class="${tab === "ck" ? "active" : ""}">Chuyển khoản</button>
        <button type="button" data-tab="luong" class="${tab === "luong" ? "active" : ""}">Lương GV</button>
        <button type="button" data-tab="hethong" class="${tab === "hethong" ? "active" : ""}">Hệ thống</button>
      </div>
      <div class="panel" id="main"></div>
      ${lastErr ? `<p class="err">${escapeHtml(lastErr)}</p>` : ""}
      ${lastMsg ? `<p class="ok">${escapeHtml(lastMsg)}</p>` : ""}
    </div>
  `;
  document.querySelector("#logout")!.addEventListener("click", () => void doLogout());
  document.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((b) => {
    b.addEventListener("click", () => {
      tab = b.dataset.tab as typeof tab;
      void render();
    });
  });
  const main = document.querySelector<HTMLDivElement>("#main")!;
  if (tab === "hs") await renderHs(main);
  else if (tab === "congno") await renderCongno(main);
  else if (tab === "ck") await renderCk(main);
  else if (tab === "luong") await renderLuong(main);
  else await renderHeThong(main);
}

async function renderHs(main: HTMLDivElement) {
  main.innerHTML = `<p class="meta">Đang tải…</p>`;
  try {
    const rows = await loadHs();
    main.innerHTML = `
      <h2 style="margin:0 0 .75rem;font:600 1.1rem Literata,serif">Danh sách học viên</h2>
      <ul class="list" id="hslist">
        ${rows
          .map((r) => {
            const o = r as Record<string, Json>;
            const id = Number(o.hocsinhid);
            const sel = selectedHs === id ? "sel" : "";
            return `<li class="${sel}" data-id="${id}">
              <strong>${escapeHtml(String(o.hotenhs || ""))}</strong>
              <div class="meta">${escapeHtml(String(o.mahocvien || ""))} · id ${id}</div>
            </li>`;
          })
          .join("")}
      </ul>
    `;
    document.querySelectorAll<HTMLLIElement>("#hslist li").forEach((li) => {
      li.addEventListener("click", () => {
        selectedHs = Number(li.dataset.id);
        tab = "congno";
        void render();
      });
    });
  } catch (e) {
    main.innerHTML = `<p class="err">${escapeHtml(String(e))}</p>`;
  }
}

async function renderCongno(main: HTMLDivElement) {
  if (!selectedHs) {
    main.innerHTML = `<p class="meta">Chọn học viên ở tab Học viên trước.</p>`;
    return;
  }
  main.innerHTML = `<p class="meta">Đang tải công nợ HV #${selectedHs}…</p>`;
  try {
    const debt = await loadDebt(selectedHs);
    const quy = await api<Record<string, Json>>("quy.get", {});
    main.innerHTML = `
      <div class="grid two">
        <div>
          <h2 style="margin:0 0 .75rem;font:600 1.1rem Literata,serif">Công nợ HV #${selectedHs}</h2>
          <ul class="list" id="debt">
            ${debt
              .map((r) => {
                const o = r as Record<string, Json>;
                return `<li data-ky="${Number(o.kyid)}" data-so="${Number(o.conlai)}">
                  <strong>${escapeHtml(String(o.makyhp || o.thang || ""))}</strong>
                  <div class="meta">${escapeHtml(String(o.tenlop || ""))} · còn ${money(Number(o.conlai || 0))}</div>
                </li>`;
              })
              .join("") || `<li class="meta">Không còn nợ.</li>`}
          </ul>
        </div>
        <div>
          <h2 style="margin:0 0 .75rem;font:600 1.1rem Literata,serif">Thu tiền mặt</h2>
          <p class="meta">Quỹ hiện tại: ${money(Number(quy.tienhientai || 0))}</p>
          <p class="meta" id="picked">Chưa chọn kỳ.</p>
          <button type="button" id="collect" ${busy ? "disabled" : ""}>Thu đúng số còn lại (TM)</button>
          <button type="button" id="receipt" class="ghost" ${busy || !lastReceiptIds.length ? "disabled" : ""}>Xem biên lai</button>
          <pre class="out" id="out"></pre>
          ${lastReceiptText ? `<pre class="out" id="receiptout">${escapeHtml(lastReceiptText)}</pre>` : ""}
        </div>
      </div>
    `;
    let pick: { kyid: number; sotien: number } | null = null;
    const picked = document.querySelector("#picked")!;
    const out = document.querySelector("#out")!;
    document.querySelectorAll<HTMLLIElement>("#debt li[data-ky]").forEach((li) => {
      li.addEventListener("click", () => {
        document.querySelectorAll("#debt li").forEach((x) => x.classList.remove("sel"));
        li.classList.add("sel");
        pick = { kyid: Number(li.dataset.ky), sotien: Number(li.dataset.so) };
        picked.textContent = `Kỳ ${pick.kyid} · ${money(pick.sotien)}`;
      });
    });
    document.querySelector("#collect")!.addEventListener("click", () => {
      if (!pick || !selectedHs) {
        setStatus("", "Chọn một kỳ còn nợ.");
        void render();
        return;
      }
      const item = pick;
      void withBusy(async () => {
        const r = (await api("hocphi.collect", {
          hocsinhid: selectedHs!,
          hinhthuc: "TM",
          items: [{ kyid: item.kyid, sotien: item.sotien }],
          ghichu: "UI TPS 3.0",
        })) as Record<string, Json>;
        out.textContent = JSON.stringify(r, null, 2);
        const ids = extractReceiptIds(r);
        lastReceiptIds = ids;
        if (ids.length) {
          const bl = (await api("hocphi.receipt", { ids })) as Record<string, Json>;
          lastReceiptText = String(bl.text || "");
        }
        setStatus(`Đã thu ${money(item.sotien)}.`);
      });
    });
    document.querySelector("#receipt")?.addEventListener("click", () => {
      if (!lastReceiptIds.length) return;
      void withBusy(async () => {
        const bl = (await api("hocphi.receipt", { ids: lastReceiptIds })) as Record<string, Json>;
        lastReceiptText = String(bl.text || "");
        out.textContent = JSON.stringify(bl, null, 2);
        setStatus("Đã tải biên lai (text).");
      });
    });
  } catch (e) {
    main.innerHTML = `<p class="err">${escapeHtml(String(e))}</p>`;
  }
}

function extractReceiptIds(r: Record<string, Json>): number[] {
  const candidates = [r.ids, r.lineids, r.phieuthu];
  for (const c of candidates) {
    if (Array.isArray(c)) {
      const ids = c
        .map((x) => {
          if (typeof x === "number") return x;
          if (x && typeof x === "object") {
            const o = x as Record<string, Json>;
            return Number(o.lineid ?? o.id ?? 0);
          }
          return Number(x);
        })
        .filter((n) => Number.isFinite(n) && n > 0);
      if (ids.length) return ids;
    }
  }
  const one = Number(r.lineid ?? 0);
  return one > 0 ? [one] : [];
}

async function renderHeThong(main: HTMLDivElement) {
  main.innerHTML = `
    <h2 style="margin:0 0 .75rem;font:600 1.1rem Literata,serif">Hệ thống (S6)</h2>
    <p class="meta">Phiên bản · kiểm tra cập nhật GitHub Releases · biên lai text gần nhất.</p>
    <div class="row">
      <button type="button" id="ver">Xem phiên bản</button>
      <button type="button" id="ota" ${busy ? "disabled" : ""}>Kiểm tra cập nhật</button>
    </div>
    <pre class="out" id="sysout" style="margin-top:1rem"></pre>
    ${lastReceiptText ? `<h2 style="margin:1rem 0 .5rem;font:600 1.05rem Literata,serif">Biên lai gần nhất</h2><pre class="out">${escapeHtml(lastReceiptText)}</pre>` : ""}
  `;
  const out = () => document.querySelector("#sysout")!;
  document.querySelector("#ver")!.addEventListener("click", () => {
    void withBusy(async () => {
      const r = await api("app.version");
      out().textContent = JSON.stringify(r, null, 2);
      setStatus("Đã tải app.version.");
    });
  });
  document.querySelector("#ota")!.addEventListener("click", () => {
    void withBusy(async () => {
      const r = (await api("app.checkUpdate", {})) as Record<string, Json>;
      out().textContent = JSON.stringify(r, null, 2);
      setStatus(String(r.message || "Đã kiểm tra cập nhật."));
    });
  });
  void withBusy(async () => {
    const r = await api("app.version");
    out().textContent = JSON.stringify(r, null, 2);
  });
}

async function renderCk(main: HTMLDivElement) {
  main.innerHTML = `<p class="meta">Đang tải…</p>`;
  try {
    const bank = await api<Json>("ck.bankInfo", {});
    const gd = await loadGd();
    main.innerHTML = `
      <div class="grid two">
        <div>
          <h2 style="margin:0 0 .75rem;font:600 1.1rem Literata,serif">Tài khoản nhận CK</h2>
          <pre class="out">${escapeHtml(JSON.stringify(bank, null, 2))}</pre>
          <div class="row" style="margin-top:1rem">
            <label>Số tiền thử QR
              <input id="sotien" type="number" value="300000" />
            </label>
            <button type="button" id="qr">Tạo QR thử</button>
          </div>
          <pre class="out" id="qrout"></pre>
        </div>
        <div>
          <h2 style="margin:0 0 .75rem;font:600 1.1rem Literata,serif">Giao dịch Pay2S gần đây</h2>
          <ul class="list">
            ${gd
              .map((r) => {
                const o = r as Record<string, Json>;
                return `<li>
                  <strong>${escapeHtml(String(o.magd || ""))}</strong>
                  <div class="meta">${escapeHtml(String(o.trangthai || ""))} · ${money(Number(o.sotien || 0))}</div>
                </li>`;
              })
              .join("") || `<li class="meta">Chưa có giao dịch.</li>`}
          </ul>
        </div>
      </div>
    `;
    document.querySelector("#qr")!.addEventListener("click", () => {
      const sotien = Number((document.querySelector("#sotien") as HTMLInputElement).value || 0);
      void withBusy(async () => {
        const r = await api("ck.qrPreview", { sotien, noidung: "TT59 DEMO000001" });
        document.querySelector("#qrout")!.textContent = JSON.stringify(r, null, 2);
        setStatus("Đã tạo payload VietQR.");
      });
    });
  } catch (e) {
    main.innerHTML = `<p class="err">${escapeHtml(String(e))}</p>`;
  }
}

async function renderLuong(main: HTMLDivElement) {
  const now = new Date();
  const vn = new Date(now.getTime() + 7 * 3600 * 1000);
  const thangDefault = `${String(vn.getUTCMonth() + 1).padStart(2, "0")}-${vn.getUTCFullYear()}`;
  main.innerHTML = `
    <h2 style="margin:0 0 .75rem;font:600 1.1rem Literata,serif">Lương giáo viên (S4b)</h2>
    <p class="meta">Danh sách · chi tiết · khấu trừ · chốt · chuyển khoản (CSV).</p>
    <div class="row">
      <label>Tháng (MM-YYYY)
        <input id="thangluong" value="${thangDefault}" />
      </label>
      <button type="button" id="loadluong">Xem bảng</button>
      <button type="button" id="loadck">DS chuyển khoản</button>
    </div>
    <div class="row" style="margin-top:.75rem">
      <label>giaovienid
        <input id="gvid" type="number" placeholder="id GV" />
      </label>
      <button type="button" id="loadgv">Chi tiết GV</button>
      <button type="button" id="chotgv">Chốt</button>
      <button type="button" id="mogv">Mở lại</button>
    </div>
    <div class="row" style="margin-top:.75rem">
      <label>Khấu trừ
        <select id="ktloai">
          <option value="tamung">Tạm ứng</option>
          <option value="khac">Khác</option>
        </select>
      </label>
      <label>Số tiền
        <input id="kttien" type="number" value="50000" />
      </label>
      <label>Lý do
        <input id="ktlydo" value="Ứng giữa tháng" />
      </label>
      <button type="button" id="addkt">Thêm KT</button>
    </div>
    <pre class="out" id="luongout" style="margin-top:1rem"></pre>
  `;
  const out = () => document.querySelector("#luongout")!;
  const thangOf = () => (document.querySelector("#thangluong") as HTMLInputElement).value.trim();
  const gvidOf = () => Number((document.querySelector("#gvid") as HTMLInputElement).value || 0);
  const load = () => {
    const thang = thangOf();
    void withBusy(async () => {
      const ds = await api("bl.ds", { thang });
      out().textContent = JSON.stringify(ds, null, 2);
      setStatus(`Đã tải lương tháng ${thang}.`);
    });
  };
  document.querySelector("#loadluong")!.addEventListener("click", load);
  document.querySelector("#loadck")!.addEventListener("click", () => {
    void withBusy(async () => {
      const r = await api("bl.ck.ds", { thang: thangOf() });
      out().textContent = JSON.stringify(r, null, 2);
      setStatus("Đã tải DS chuyển khoản.");
    });
  });
  document.querySelector("#loadgv")!.addEventListener("click", () => {
    const gvid = gvidOf();
    if (!gvid) {
      setStatus("", "Nhập giaovienid.");
      void render();
      return;
    }
    void withBusy(async () => {
      const r = await api("bl.gv", { thang: thangOf(), giaovienid: gvid });
      out().textContent = JSON.stringify(r, null, 2);
      setStatus(`Chi tiết GV #${gvid}.`);
    });
  });
  document.querySelector("#chotgv")!.addEventListener("click", () => {
    const gvid = gvidOf();
    if (!gvid) {
      setStatus("", "Nhập giaovienid.");
      void render();
      return;
    }
    void withBusy(async () => {
      const r = await api("bl.chot", { thang: thangOf(), giaovienid: gvid });
      out().textContent = JSON.stringify(r, null, 2);
      setStatus(`Đã chốt GV #${gvid}.`);
    });
  });
  document.querySelector("#mogv")!.addEventListener("click", () => {
    const gvid = gvidOf();
    if (!gvid) {
      setStatus("", "Nhập giaovienid.");
      void render();
      return;
    }
    void withBusy(async () => {
      const r = await api("bl.mo", {
        thang: thangOf(),
        giaovienid: gvid,
        lydo: "Mở lại từ UI TPS 3.0",
      });
      out().textContent = JSON.stringify(r, null, 2);
      setStatus(`Đã mở lại GV #${gvid}.`);
    });
  });
  document.querySelector("#addkt")!.addEventListener("click", () => {
    const gvid = gvidOf();
    if (!gvid) {
      setStatus("", "Nhập giaovienid.");
      void render();
      return;
    }
    void withBusy(async () => {
      const r = await api("bl.khautru", {
        thang: thangOf(),
        giaovienid: gvid,
        loai: (document.querySelector("#ktloai") as HTMLSelectElement).value,
        sotien: Number((document.querySelector("#kttien") as HTMLInputElement).value || 0),
        lydo: (document.querySelector("#ktlydo") as HTMLInputElement).value.trim(),
      });
      out().textContent = JSON.stringify(r, null, 2);
      setStatus("Đã thêm khấu trừ.");
    });
  });
  load();
}

async function render() {
  if (!me) {
    loginView();
    return;
  }
  await shellView();
}

void (async () => {
  try {
    await api("app.init");
  } catch {
    /* migrate may already be applied */
  }
  await refreshMe();
  await render();
})();
