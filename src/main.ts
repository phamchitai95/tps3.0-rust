import { invoke } from "@tauri-apps/api/core";

const app = document.querySelector<HTMLDivElement>("#app")!;

app.innerHTML = `
  <main class="wrap">
    <p class="brand">TPS 3.0</p>
    <h1>Rust + Tauri</h1>
    <p class="sub">Scaffold S0 — core RPC + logic tests. UI nghiệp vụ sẽ làm sau.</p>
    <button id="ping" type="button">Gọi app.ping</button>
    <pre id="out"></pre>
  </main>
`;

const out = document.querySelector<HTMLPreElement>("#out")!;
document.querySelector("#ping")!.addEventListener("click", async () => {
  try {
    const v = await invoke("api_call", { name: "app.version", args: {} });
    out.textContent = JSON.stringify(v, null, 2);
  } catch (e) {
    out.textContent = String(e);
  }
});
