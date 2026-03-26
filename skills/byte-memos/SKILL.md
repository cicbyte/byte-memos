---
name: byte-memos-cli
description: 通过 `bm` 命令行管理 byte-memos 本地备忘录。当用户想创建/整理备忘录（Markdown 笔记 + 标签 + 分类 + 可见性 + 置顶），或搜索、导出、备份恢复备忘录时使用——提到 "byte-memos"、"bm 命令"、"记个备忘"、"存个笔记"、"找我记得那条笔记/备忘录" 时触发。所有命令支持 `--json` 输出，适合脚本与 AI 工作流。
---

# byte-memos CLI（`bm`）

`bm` 是 byte-memos 的命令行工具——纯本地、单人、零部署的备忘录管理应用。核心两件事：**记**（创建 Markdown 备忘录）和**查**（全文搜索、标签分类过滤）。所有命令支持 `--json`，天然适合脚本与 AI 工作流。

## 何时使用

- 用户说"记个备忘"、"存个笔记"、"把这个记下来"
- 用户想搜索或翻阅自己存的备忘录
- 用户要整理备忘录（打标签、分类、置顶、可见性）
- 导出单篇备忘录（Markdown/HTML）或整机备份恢复
- 自动化脚本需要结构化读写备忘录库
- 启动 MCP / REST 服务供 AI 工具对接

## 找到 bm 二进制

CLI 可执行文件名为 `bm`。如果不在 PATH：

- `~/.cargo/bin/bm`（`cargo install` 安装后）
- `<仓库>/target/debug/bm` 或 `target/release/bm`（源码编译）
- 或在 byte-memos 仓库内 `cargo build --workspace` 后用 `target/debug/bm`

找不到就问用户怎么调用 `bm`，别假设不存在。

## `--json` 标志（务必先读）

每个命令都支持 `--json`，输出机器可读 JSON，放在子命令前：

```bash
bm --json memo add --title "示例" --content "内容" 
```

JSON 模式行为：输出合法 JSON（add/get 返回对象，list/search 返回数组）；错误走 stderr + 非零退出码，stdout 始终干净。

## 记录（写入备忘录）★ 第一重点

### 新增

```bash
bm memo add --title <标题> --content <Markdown 内容> [--visibility PRIVATE|PROTECTED|PUBLIC] [--category 分类] [--tag 标签]...
```

- `--title` 与 `--content` 不能同时为空
- `--tag` 可多次（`--tag rust --tag tools`），支持层级标签 `--tag "工作/项目A"`（自动建父标签"工作"）
- `--category` 分类名（不存在自动创建）
- 创建后用 `--json` 捕获 `short_id`（8 字符，后续 get/update/delete 的主键）

```bash
bm --json memo add --title "rustup 安装笔记" --content "curl …" --tag rust --tag tools --category 开发
```

## 查阅（读取备忘录）★ 第二重点

### 列出（过滤 + 分页）

```bash
bm memo list [--category 分类] [--tag 标签] [--pinned] [--limit N] [--offset M]
```

`--pinned` 仅列置顶的备忘录。

### 读取详情（完整 Markdown 内容）

```bash
bm memo get <shortId>
```

### 搜索（FTS5 + 中文 LIKE 兜底）

```bash
bm memo search "<关键词>" [--limit N] [--offset M]    # 范围：标题+内容+标签，高亮片段
```

中文关键词可直接命中（FTS 之外有 LIKE 模糊兜底）。

## 修改与整理

### 更新（未提供的字段保留原值）

```bash
bm memo update <shortId> [--title 标题] [--content 内容] [--visibility 可见性] [--category 分类] [--tag 标签]...
```

### 标签（增量打 / 去）

```bash
bm memo tag <shortId> --add 标签 [--add 标签2]... [--remove 旧标签]...
```

### 置顶 / 可见性

```bash
bm memo pin <shortId>                # 置顶/取消置顶（切换）
bm memo visibility <shortId> --set PUBLIC|PROTECTED|PRIVATE
```

### 关联（备忘录之间的双向引用）

```bash
bm memo link <shortIdA> <shortIdB>       # 建立关联
bm memo unlink <shortIdA> <shortIdB>     # 解除关联
bm memo relations <shortId>              # 查看该备忘录的全部关联
```

### 版本历史（内容变更自动快照）

```bash
bm memo history <shortId>                          # 版本列表（含旧内容，时间倒序）
bm memo restore-version <shortId> <versionId>      # 回滚到指定版本（历史只增不减）
```

### 删除（软删除→回收站，可恢复）

```bash
bm memo delete <shortId> [-y]
```

## 导出 / 备份

```bash
bm memo export-md <shortId> [-o <路径>]    # 导出 Markdown（无 -o 则打印 stdout）
bm memo export-html <shortId> [-o <路径>]  # 导出 HTML（可用浏览器打印为 PDF）
bm backup [--output <路径>] [--no-compress]  # 整机备份（zip，默认 data_root/backups/）
bm restore <备份文件或目录> [-y]              # 覆盖当前数据
```

备份前自动 WAL checkpoint 确保一致性。恢复后桌面端在运行需重启。

## 元数据管理

```bash
bm category list                       # → 分类列表（含每分类下备忘录数）
bm category create <名称>
bm category rename --id <id> --name <新名称>
bm category delete --id <id>           # 备忘录保留，仅解除分类绑定
bm tag list                            # → 标签使用统计
```

标签是自由文本，靠备忘录隐式存在；`tag` 子命令只做统计展示。

## MCP / REST 服务

```bash
bm mcp serve [--transport stdio|http] [--port 8217] [--host 127.0.0.1]   # MCP 服务器（Claude Desktop/Cursor）
bm api serve [--port 8318] [--host 127.0.0.1]                           # REST API（/api/v1 + Swagger 文档）
```

MCP 5 工具：search / list / get / stats / delete。

## 数据位置

默认 `~/.cicbyte/apps/byte-memos/`。`BYTE_MEMOS_HOME` 覆盖 app 根目录（测试/隔离用）：

```
~/.cicbyte/apps/byte-memos/
├── config/config.toml     # data_dir / 偏好 / API Key（固定在 app 根）
└── data/
    ├── db/byte-memos.db   # SQLite（备忘录 / 标签 / 分类 / 版本 / 附件 / FTS 索引）
    └── attachments/       # 附件文件存储（按 shortId 分目录）
```

## AI 代理注意事项

- **要解析输出就用 `--json`**；人类可读模式含颜色/emoji，难解析。
- **`short_id` 是 8 字符字符串**，get/update/delete/pin/tag 的主键。创建后从输出捕获。
- **`delete` 是软删除**（进回收站，桌面端可恢复）。别警告用户"不可恢复"。
- **搜索支持中文**：FTS5 之外有 LIKE 兜底，"数据库设计"这类中文关键词能命中标题/内容/标签。
- **层级标签用 `/`**：`--tag "工作/项目A"` 自动建父标签。
- **版本历史自动记录**：每次内容变更自动快照旧内容，回滚不丢历史。
- **更新是字段级可选的**：只传 `--title` 则内容/标签等保持原值；`--tag` 传了则是全量替换语义（配合 `memo tag` 做增量）。

## 实测踩过的坑

- `memo add` 的 `--title`/`--content` 均可省略其一，但不能同时为空（业务报错）。
- 搜索结果默认 20 条，`--offset 20` 取第二页。
- `memo visibility` 的值是大小写不敏感的枚举（PRIVATE/PROTECTED/PUBLIC）。
- 备份恢复是**覆盖式**，恢复前先确认当前数据不需要保留。
