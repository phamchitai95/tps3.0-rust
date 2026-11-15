import { invoke } from "@tauri-apps/api/core";

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

async function api<T = Json>(name: string, args: Record<string, Json> = {}): Promise<T> {
  return invoke<T>("api_call", { name, args });
}

const app = document.querySelector<HTMLDivElement>("#app")!;

type Me = { hotennv?: string; tendangnhap?: string; chucvu?: number } | null;

let me: Me = null;
let tab: "hs" | "congno" | "ck" = "hs";
let selectedHs: number | null = null;
let lastMsg = "";
let lastErr = "";
let busy = false;

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
  else await renderCk(main);
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
          <pre class="out" id="out"></pre>
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
        const r = await api("hocphi.collect", {
          hocsinhid: selectedHs!,
          hinhthuc: "TM",
          items: [{ kyid: item.kyid, sotien: item.sotien }],
          ghichu: "UI TPS 3.0",
        });
        out.textContent = JSON.stringify(r, null, 2);
        setStatus(`Đã thu ${money(item.sotien)}.`);
      });
    });
  } catch (e) {
    main.innerHTML = `<p class="err">${escapeHtml(String(e))}</p>`;
  }
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
