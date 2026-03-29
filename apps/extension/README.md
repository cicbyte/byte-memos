# byte-memos 剪藏（浏览器扩展）

一键把网页内容保存到本地 byte-memos 备忘录。纯静态 Manifest V3 扩展，零构建、零依赖，Chrome / Edge 均可加载。

## 功能

- **右键菜单剪藏**
  - 选中文字 → 保存摘录（自动附来源链接）
  - 整页 → 保存标题 + 链接
  - 链接 / 图片 → 保存为 Markdown 链接 / 图片
- **弹窗快记**：随手记一条，可选附当前页面链接，Ctrl+Enter 快速保存
- **连接状态**：弹窗实时显示与 byte-memos 的连接情况（正常 / Key 不对 / 服务未开）
- **可配置**：服务地址、API Key；非本机地址保存时自动申请源权限

## 准备：在 byte-memos 中开启 REST API

1. 打开 byte-memos 桌面应用 → 左侧「REST API 服务」
2. 启动服务（默认端口 `8318`，监听 `0.0.0.0` 或 `127.0.0.1` 均可）
3. 如启用了 API Key 认证：记下 Key；同时建议开启「本机免 Key」（本机回环访问免鉴权），扩展里 Key 即可留空

## 安装扩展（开发者模式加载）

1. 打开 `chrome://extensions`（Edge 为 `edge://extensions`）
2. 右上角打开「开发者模式」
3. 点「加载已解密的扩展程序」/「加载已解压的扩展」→ 选择本目录 `apps/extension`
4. 点扩展图标 → 右上角 ⚙ 填服务地址（默认 `http://127.0.0.1:8318/api/v1`）和 API Key（可选）→ 测试连接 → 保存

## 使用

- 选中网页文字 → 右键 →「保存选中文字到 byte-memos」
- 或点工具栏图标 → 弹窗快记
- 保存成功会弹系统通知；内容进备忘录后自动带 `网页划线` / `网页剪藏` / `链接` / `图片` / `快记` 标签，来源以引用块附在正文末尾

## 目录结构

```
apps/extension/
├── manifest.json        # MV3 清单（host_permissions 默认仅本机）
├── background.js        # Service Worker：右键菜单 / 消息路由 / REST 调用
├── popup/               # 弹窗：快记 + 连接状态
├── options/             # 设置：服务地址 / API Key / 连接测试
├── icons/               # 图标
└── scripts/make_icons.py # 图标再生成脚本（Pillow）
```

## 对接的接口

| 用途 | 端点 |
| --- | --- |
| 保存备忘录 | `POST /api/v1/memos`（`title` / `content` / `tags` / `visibility`） |
| 连接测试 | `GET /api/v1/memos?limit=1` |

鉴权：`X-API-Key` 请求头（服务端开启「本机免 Key」且走 127.0.0.1 时可不带）。完整接口文档见应用内「REST API 服务 → API 文档」或 `http://localhost:8318/api/v1/docs`。

## 隐私

- 只与你自己配置的 byte-memos 服务地址通信，不收集、不上传任何数据
- 默认仅申请 `127.0.0.1` / `localhost` 的访问权限；自定地址需你在保存设置时显式授权
