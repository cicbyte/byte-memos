/**
 * 设置页：服务地址 / API Key（chrome.storage.local 持久化）
 * 非 localhost 地址在保存时申请对应源的可选 host 权限（MV3 optional_host_permissions）
 */

const DEFAULT_SERVER = "http://127.0.0.1:8318/api/v1";
const $ = (id) => document.getElementById(id);

async function load() {
  const { serverUrl, apiKey } = await chrome.storage.local.get(["serverUrl", "apiKey"]);
  $("serverUrl").value = serverUrl || DEFAULT_SERVER;
  $("apiKey").value = apiKey || "";
}

function normalizeUrl(raw) {
  let u = raw.trim().replace(/\/+$/, "");
  if (!u) return "";
  if (!/^https?:\/\//i.test(u)) u = `http://${u}`;
  return u;
}

/** 非本机源需申请 optional host 权限，否则 fetch 会被拦 */
async function ensureOriginPermission(url) {
  try {
    const origin = new URL(url).origin;
    if (/^https?:\/\/(127\.0\.0\.1|localhost)(:\d+)?$/i.test(origin)) return true;
    return await chrome.permissions.request({ origins: [`${origin}/*`] });
  } catch {
    return false;
  }
}

async function ping(url, key) {
  const headers = {};
  if (key) headers["X-API-Key"] = key;
  try {
    const resp = await fetch(`${url}/memos?limit=1`, { headers });
    if (resp.ok) return { ok: true, msg: "连接成功 ✓" };
    if (resp.status === 401) return { ok: false, msg: "已连通，但 API Key 不对（401）" };
    return { ok: false, msg: `服务响应异常（HTTP ${resp.status}）` };
  } catch {
    return { ok: false, msg: "无法连接：请确认 byte-memos 已开启 REST API 服务，端口正确" };
  }
}

function show(ok, msg) {
  const box = $("result");
  box.hidden = false;
  box.className = `result ${ok ? "ok" : "fail"}`;
  box.textContent = msg;
}

$("toggleKey").addEventListener("click", () => {
  const el = $("apiKey");
  const showIt = el.type === "password";
  el.type = showIt ? "text" : "password";
  $("toggleKey").textContent = showIt ? "隐藏" : "显示";
});

$("test").addEventListener("click", async () => {
  const url = normalizeUrl($("serverUrl").value) || DEFAULT_SERVER;
  show(false, "测试中…");
  const r = await ping(url, $("apiKey").value.trim());
  show(r.ok, r.msg);
});

$("save").addEventListener("click", async () => {
  const url = normalizeUrl($("serverUrl").value) || DEFAULT_SERVER;
  const key = $("apiKey").value.trim();
  const granted = await ensureOriginPermission(url);
  if (!granted) {
    show(false, `未授予访问 ${url} 的权限，保存后无法使用`);
  }
  await chrome.storage.local.set({ serverUrl: url, apiKey: key });
  const r = await ping(url, key);
  show(r.ok, r.ok ? `已保存，${r.msg}` : `已保存，但${r.msg}`);
});

load();
