# byte-memos

纯本地、单人、零部署的**备忘录管理应用**。统一管理 Markdown 备忘录：增删改查、标签分类、全文搜索（FTS5 + 中文模糊 + 正则）、置顶、可见性管理。通过 **CLI / Desktop / HTTP / MCP** 多端操作同一份本地 SQLite。

## 架构

```
┌────────────────────────────────────────────┐
│ apps/desktop (Tauri v2 + Vue 3)            │  桌面端
│ apps/cli       (bm)                        │  命令行
├────────────────────────────────────────────┤
│ crates/core    (byte-memos-core)           │  核心库：业务逻辑 + 数据层 + MCP
│                ProjectService 单一真相源     │  CLI / 桌面端 / REST 共同复用
├────────────────────────────────────────────┤
│ SQLite (rusqlite + FTS5)                   │  ~/.cicbyte/apps/byte-memos/data/
└────────────────────────────────────────────┘
```

- **crates/core** — 业务逻辑 + 数据层 + MCP 工具实现的单一真相源，不依赖任何 transport 或 UI 框架
- **apps/cli**（`bm`）— 子命令式 CLI，全局 `--json`，含 MCP stdio/http 与 REST API 服务器
- **apps/desktop** — Tauri v2 桌面端（Vue 3 + Ant Design Vue + md-editor-v3），复用 core 的全部业务

## 功能

- **备忘录**：Markdown 编辑（桌面端 md-editor-v3）、标题/内容/标签/分类/可见性（PRIVATE/PROTECTED/PUBLIC）/置顶
- **搜索**：FTS5 bm25 排名 + 中文 LIKE 兜底 + 正则表达式搜索
- **元数据**：分类管理、标签统计、层级标签（`工作/项目A`）
- **版本历史**：内容变更自动快照，可回滚（历史只增不减）
- **附件**：DB / 文件双存储模式
- **模板**：从模板建备忘录、备忘录存为模板
- **回收站**：软删除、恢复、彻底删除、清空
- **备份**：整机 zip 备份（自动 WAL checkpoint）、恢复
- **双服务**：MCP（stdio / StreamableHTTP，5 工具）+ REST API（`/api/v1` + Swagger）

## 构建

前置：Rust 1.75+、Node 20+、pnpm 9+。

```bash
# CLI（bm）与核心库
cargo build --release

# 测试
cargo test --workspace

# 桌面端开发
cd apps/desktop
pnpm install
pnpm tauri dev

# 桌面端打包（msi/nsis）
pnpm tauri build
```

## CLI 速览

```bash
bm memo add --title "标题" --content "# Markdown" --tag rust --category 开发
bm memo list --category 开发 --pinned
bm memo search "中文关键词"
bm memo pin <shortId>
bm memo history <shortId>            # 版本历史
bm backup                            # zip 备份到 data/backups/
bm mcp serve --transport stdio       # MCP 服务器（Claude Desktop / Cursor）
bm api serve --port 8318             # REST API + Swagger
```

全部命令支持 `--json`（脚本/AI 友好）。详见 `bm --help`。

## 数据位置

```
~/.cicbyte/apps/byte-memos/
├── config/config.toml     # data_dir / 偏好 / API Key
└── data/
    ├── db/byte-memos.db   # SQLite（含 FTS 索引）
    └── attachments/       # 附件
```

`BYTE_MEMOS_HOME` 环境变量可覆盖 app 根目录。数据目录可在桌面端设置页移动到任意路径（外置硬盘、网盘）。

## License

MIT
