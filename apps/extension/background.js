/**
 * byte-memos 剪藏 - Service Worker
 *
 * - 右键菜单：选中文字 / 整页 / 链接 / 图片 → 保存到 byte-memos（REST API）
 * - 消息路由：popup 的 ping（连接测试）与 save（快速记录）
 * - 配置（chrome.storage.local）：{ serverUrl, apiKey }
 */

const DEFAULT_SERVER = "http://127.0.0.1:8318/api/v1";

/** 读配置（缺省填默认值） */
async function getConfig() {
  const { serverUrl, apiKey } = await chrome.storage.local.get(["serverUrl", "apiKey"]);
  return { serverUrl: serverUrl || DEFAULT_SERVER, apiKey: apiKey || "" };
}

/** 保存一条备忘录；返回 { ok, status, error } */
async function saveMemo({ title, content, tags }) {
  const { serverUrl, apiKey } = await getConfig();
  const headers = { "Content-Type": "application/json" };
  if (apiKey) headers["X-API-Key"] = apiKey;
  try {
    const resp = await fetch(`${serverUrl.replace(/\/$/, "")}/memos`, {
      method: "POST",
      headers,
      body: JSON.stringify({
        title: title || "",
        content,
        visibility: "PRIVATE",
        tags: tags || [],
      }),
    });
    if (resp.ok) return { ok: true };
    const text = await resp.text().catch(() => "");
    return { ok: false, status: resp.status, error: text.slice(0, 200) };
  } catch (e) {
    return { ok: false, error: String(e) };
  }
}

/** 连接测试（GET /memos?limit=1）；返回 { ok, status } */
async function ping() {
  const { serverUrl, apiKey } = await getConfig();
  const headers = {};
  if (apiKey) headers["X-API-Key"] = apiKey;
  try {
    const resp = await fetch(`${serverUrl.replace(/\/$/, "")}/memos?limit=1`, { headers });
    return { ok: resp.ok, status: resp.status };
  } catch {
    return { ok: false, status: 0 };
  }
}

/** 系统通知（后台右键保存时反馈） */
function notify(ok, message) {
  chrome.notifications.create({
    type: "basic",
    iconUrl: chrome.runtime.getURL("icons/icon128.png"),
    title: ok ? "已保存到 byte-memos" : "保存失败",
    message,
  });
}

// ===== 右键菜单 =====

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({ id: "bm-selection", title: "保存选中文字到 byte-memos", contexts: ["selection"] });
    chrome.contextMenus.create({ id: "bm-page", title: "保存此页面到 byte-memos", contexts: ["page"] });
    chrome.contextMenus.create({ id: "bm-link", title: "保存此链接到 byte-memos", contexts: ["link"] });
    chrome.contextMenus.create({ id: "bm-image", title: "保存此图片到 byte-memos", contexts: ["image"] });
  });
});

/** 链接文本里的方括号会破坏 Markdown 链接语法，剥掉 */
function cleanLinkText(s) {
  return (s || "").replace(/[[\]]/g, "").trim();
}

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const src = tab && tab.title ? tab.title : (tab && tab.url) || "";
  const url = info.pageUrl || (tab && tab.url) || "";
  const sourceLine = url ? `\n\n> 来源：[${cleanLinkText(src) || "网页"}](${url})` : "";

  let title = "";
  let content = "";
  let tags = [];

  if (info.menuItemId === "bm-selection") {
    title = src || "网页划线";
    content = `${info.selectionText || ""}${sourceLine}`;
    tags = ["网页划线"];
  } else if (info.menuItemId === "bm-page") {
    title = src || "网页剪藏";
    content = `[${cleanLinkText(src) || url}](${url})`;
    tags = ["网页剪藏"];
  } else if (info.menuItemId === "bm-link") {
    title = cleanLinkText(info.linkText) || "链接";
    content = `[${cleanLinkText(info.linkText) || info.linkUrl}](${info.linkUrl})${sourceLine}`;
    tags = ["链接"];
  } else if (info.menuItemId === "bm-image") {
    title = "图片";
    content = `![图片](${info.srcUrl})${sourceLine}`;
    tags = ["图片"];
  } else {
    return;
  }

  const r = await saveMemo({ title, content, tags });
  notify(r.ok, r.ok ? title : `状态 ${r.status ?? ""} ${r.error || "请确认 REST API 服务已开启"}`);
});

// ===== 消息路由（popup / options） =====

chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
  if (msg && msg.type === "ping") {
    ping().then(sendResponse);
    return true; // 异步响应
  }
  if (msg && msg.type === "save") {
    saveMemo(msg.payload).then(sendResponse);
    return true;
  }
  return false;
});
