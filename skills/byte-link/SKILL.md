---
name: byte-link-cli
description: 通过 `bs` 命令行管理 byte-link 本地书签。当用户想保存/整理浏览器书签（URL + 标题 + 描述 + 标签 + 分类），或搜索、打开、导入导出书签时使用——提到 "byte-link"、"bs 命令"、"存个书签"、"把这个链接存一下"、"找我存的链接/书签" 时触发。所有命令支持 `--json` 输出，适合脚本与 AI 工作流。
---

# byte-link CLI（`bs`）

`bs` 是 byte-link 的命令行工具——纯本地、单人、零部署的书签管理应用。核心两件事：**存**（收藏 URL）和**查**（搜索、打开、整理）。所有命令支持 `--json`，天然适合脚本与 AI 工作流。

## 何时使用

- 用户说"存个书签"、"收藏这个链接"、"把这个网址存一下"
- 用户想搜索或打开自己存的书签
- 用户要整理书签（打标签、分类、归档、收藏）
- 从浏览器导入/导出书签（JSON 或 Netscape HTML）
- 自动化脚本需要结构化读写书签库

## 找到 bs 二进制

CLI 可执行文件名为 `bs`。如果不在 PATH：

- `~/.cargo/bin/bs`（`cargo install` 安装后）
- `<仓库>/target/debug/bs` 或 `target/release/bs`（源码编译）
- 或在 byte-link 仓库内 `cargo build --workspace` 后用 `target/debug/bs`

找不到就问用户怎么调用 `bs`，别假设不存在。

## `--json` 标志（务必先读）

每个命令都支持 `--json`，输出机器可读 JSON，放在子命令前：

```bash
bs --json <命令> [选项]
```

JSON 模式行为：输出合法 JSON（add/get 返回对象，list/search 返回数组，美化缩进）；错误走 stderr + 非零退出码，stdout 始终干净。配合 `jq` 提取：

```bash
# 存书签并捕获 shortId
bs --json bookmark add --url https://example.com --title "示例" | jq -r .short_id
# 列书签只取标题
bs --json bookmark list | jq '.[].title'
```

## 存储（写入书签）★ 第一重点

### 添加书签

```bash
bs bookmark add --url <网址> --title <标题> [--description 描述] [--tag 标签]... [--category 分类] [--no-favicon]
```

- `--url` 必填（活跃行内唯一；同 url 软删除后可复用）
- `--tag` 可多次（`--tag rust --tag tools`）
- `--category` 分类名（不存在自动创建）
- 默认自动抓取站点 favicon 缓存到本地（离线仍可见）；`--no-favicon` 跳过
- 创建后用 `--json` 捕获 `short_id`（8 字符，后续 get/open/update/delete 的主键）

```bash
bs --json bookmark add --url https://rustup.rs --title "rustup" --tag rust --tag tools --category 开发
```

## 查阅（读取书签）★ 第二重点

### 列出

```bash
bs bookmark list [--tag 标签] [--category 分类] [--favorited] [--archived] [--limit N] [--offset M]
```

### 读取详情

```bash
bs bookmark get <shortId>
```

### 搜索（支持中文模糊匹配）

```bash
bs bookmark search "<关键词>" [--limit N] [--offset M]    # 范围：标题+URL+描述+标签
```

### 打开（系统浏览器 + 计访问次数）

```bash
bs bookmark open <shortId>    # 用系统默认浏览器打开 url，visit_count +1
```

## 修改与删除

### 更新

```bash
bs bookmark update <shortId> [--title 标题] [--url 网址] [--description 描述] [--archive | --unarchive]
```

未提供的字段保留原值。

### 标签（增量打 / 去）

```bash
bs bookmark tag <shortId> --add 标签 [--add 标签2]... [--remove 旧标签]...
```

### 删除（软删除→回收站，可恢复）

```bash
bs bookmark delete <shortId>
```

## 导入 / 导出

```bash
bs bookmark export --file <路径> [--format json|html]    # html = Netscape 格式，兼容 Chrome/Firefox/Edge
bs bookmark import --file <路径> [--format json|html]    # 导入并按 url 去重
```

JSON 与备份格式互通；HTML 是浏览器通用书签格式，中文标题往返一致。

## favicon 抓取

```bash
bs bookmark fetch-favicon [<shortId>]    # 指定 id 抓单个；省略则批量补全所有缺失 favicon 的书签
```

## 分类管理

```bash
bs category list                                # → 列表（含每分类下书签数）
bs category create <名称>
bs category rename --id <id> --name <新名称>
bs category delete --id <id>                    # 书签保留，仅解除分类绑定
```

## 备份与恢复

```bash
bs backup [--output <路径>] [--no-compress]     # 默认 zip（DB + favicons）；省略输出走 data_root/backups/
bs restore <备份文件或目录> [-y]                  # 覆盖当前数据
```

备份前自动 WAL checkpoint 确保一致性。恢复后桌面端在运行需重启。

## 数据位置

默认 `~/.cicbyte/apps/byte-link/`。`BYTE_LINK_HOME` 覆盖 app 根目录（测试/隔离用）：

```bash
BYTE_LINK_HOME=/tmp/my-bm bs bookmark list
```

```
~/.cicbyte/apps/byte-link/
├── config/config.toml     # data_dir / 偏好 / API Key（固定在 app 根）
└── data/                  # 可在设置页移到任意路径（如外置硬盘、网盘）
    ├── db/byte-link.db    # SQLite（书签 / 标签 / 分类 / FTS 索引）
    └── favicons/{前2字符}/{第3-4字符}/{shortId}.{ext}   # favicon 本地缓存（前缀分桶）
```

config/ 固定 app 根；data/ 可移动。favicon 用前缀分桶避免单目录万级文件平铺。

## AI 代理注意事项

- **要解析输出就用 `--json`**；人类可读模式含 ANSI 颜色/emoji，难解析。
- `short_id` 是 8 字符字符串（如 `abc12345`），get/open/update/delete 的主键。创建后从输出捕获。
- `delete` 是**软删除**（进回收站，桌面端可恢复）。别警告用户"不可恢复"。
- `url` 在活跃书签中唯一；软删除后可重新添加同 url。
- 标签是自由文本，可多个（`--tag a --tag b`），无独立标签管理命令，靠书签隐式存在。
- **搜索支持中文**：FTS5 之外有 LIKE 模糊兜底，中文关键词（如"数据库设计"）能命中标题/URL/描述/标签。
- **导入导出**：JSON 与备份互通；HTML 兼容浏览器（Chrome/Firefox/Edge 导出后可直接 import，中文标题不乱码）。
- **favicon**：add 默认抓取缓存（best-effort，离线可见）；失败不阻断（降级为空，前端用在线占位）；`--no-favicon` 跳过。

## 实测踩过的坑

- **`bs bookmark add` 默认抓 favicon**（需联网）；离线或 `--no-favicon` 时 favicon_path 为空。
- **选项名是 `--category` / `--description`**，不是 `--cat` / `--desc`。
- **搜索结果用 `--offset` 翻页**，默认 20 条，`--offset 20` 取第二页。
- **HTML 导入**：浏览器导出的 bookmarks.html 可直接 `bs bookmark import --file x.html --format html`，中文标题往返一致。
