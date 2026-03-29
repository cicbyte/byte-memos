/**
 * popup：快速记录 + 连接状态 + 附当前页链接
 * 保存动作统一经 background 消息路由（配置与 fetch 收口在 service worker）
 */

const $ = (id) => document.getElementById(id);
let currentPage = null;

async function checkStatus() {
  const el = $("status");
  el.className = "status checking";
  $("statusText").textContent = "检测连接…";
  chrome.runtime.sendMessage({ type: "ping" }, (r) => {
    if (r && r.ok) {
      el.className = "status ok";
      $("statusText").textContent = "已连接 byte-memos";
    } else if (r && r.status === 401) {
      el.className = "status warn";
      $("statusText").textContent = "已连接但 API Key 不对（去设置检查）";
    } else {
      el.className = "status fail";
      $("statusText").textContent = "无法连接（确认应用已开启 REST API 服务）";
    }
  });
}

async function loadPage() {
  try {
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab && tab.url && /^https?:/.test(tab.url)) {
      currentPage = { title: tab.title || tab.url, url: tab.url };
      $("pageInfo").hidden = false;
      $("pageTitle").textContent = `当前页：${currentPage.title}`;
    } else {
      currentPage = null;
      $("pageInfo").hidden = true;
    }
  } catch {
    currentPage = null;
  }
}

async function save() {
  const text = $("note").value.trim();
  if (!text) return;
  $("save").disabled = true;

  let content = text;
  let title = "";
  if ($("attachPage").checked && currentPage) {
    const t = currentPage.title.replace(/[[\]]/g, "");
    content += `\n\n> 来源：[${t}](${currentPage.url})`;
    title = t;
  }

  chrome.runtime.sendMessage(
    { type: "save", payload: { title, content, tags: ["快记"] } },
    (r) => {
      const box = $("result");
      box.hidden = false;
      if (r && r.ok) {
        box.className = "result ok";
        box.textContent = "已保存 ✓";
        $("note").value = "";
        setTimeout(() => window.close(), 650);
      } else {
        box.className = "result fail";
        box.textContent = `保存失败：${(r && r.error) || "服务不可用"}`;
        $("save").disabled = false;
      }
    },
  );
}

$("save").addEventListener("click", save);
$("note").addEventListener("keydown", (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
    e.preventDefault();
    save();
  }
});
$("openOptions").addEventListener("click", () => chrome.runtime.openOptionsPage());

checkStatus();
loadPage();
