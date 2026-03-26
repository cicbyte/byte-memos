/**
 * MCP 服务页静态常量
 *
 * 监听地址选项、MCP 工具定义、配置说明文案等不随交互变化的常量。
 * 从 index.vue 抽出的大块静态数据，保持值一字不动。
 */

/** 监听地址下拉选项 */
export const hostOptions = [
  { value: "127.0.0.1", label: "127.0.0.1" },
  { value: "0.0.0.0", label: "0.0.0.0" },
];

/** MCP 工具参数定义 */
export interface ToolParam {
  name: string;
  type: string;
  required: boolean;
  desc: string;
}
export interface McpToolDef {
  name: string;
  desc: string;
  params: ToolParam[];
}

/** MCP 工具列表（读取类）——对齐 crates/core/src/mcp/server.rs 实际工具定义 */
export const readTools: McpToolDef[] = [
  { name: "search", desc: "搜索备忘录（FTS5 + 中文模糊匹配）", params: [
    { name: "query", type: "string", required: true, desc: "搜索关键词" },
    { name: "type", type: "string", required: false, desc: "搜索范围：memo / all（默认 all 跨类型合并，支持中文）" },
    { name: "limit", type: "number", required: false, desc: "结果上限（默认 20）" },
    { name: "offset", type: "number", required: false, desc: "偏移量（默认 0，翻页用）" },
  ]},
  { name: "list", desc: "列出备忘录 / 标签 / 分类", params: [
    { name: "type", type: "string", required: true, desc: "列出哪种：memo / tag / category" },
    { name: "limit", type: "number", required: false, desc: "数量上限（默认 20）" },
    { name: "offset", type: "number", required: false, desc: "偏移量（默认 0，tag/category 不支持）" },
  ]},
  { name: "get", desc: "读取备忘录详情（标题/内容/标签/分类/置顶/可见性）", params: [
    { name: "short_id", type: "string", required: true, desc: "备忘录 shortId" },
    { name: "type", type: "string", required: true, desc: "内容类型：memo" },
  ]},
  { name: "stats", desc: "统计概览（备忘录/标签/分类数量与存储占用）", params: [] },
];

/** MCP 工具列表（修改类） */
export const writeTools: McpToolDef[] = [
  { name: "delete", desc: "删除备忘录（移入回收站，可恢复）", params: [
    { name: "short_id", type: "string", required: true, desc: "备忘录 shortId" },
    { name: "type", type: "string", required: true, desc: "删除类型：memo" },
  ]},
];

/** MCP 配置说明（Markdown，MdPreview 渲染） */
export const mcpGuide = `
## stdio 配置（Claude Desktop / Cursor）

适用于 AI 工具与 bm 在同一台机器：

\`\`\`json
{
  "mcpServers": {
    "byte-memos": {
      "command": "bm",
      "args": ["mcp", "serve", "--transport", "stdio"]
    }
  }
}
\`\`\`

## HTTP 配置（远程 / 多客户端）

适用于远程 AI 工具通过 URL 连接（端点 \`/mcp\`）：

\`\`\`json
{
  "mcpServers": {
    "byte-memos": {
      "url": "http://your-server:8217/mcp"
    }
  }
}
\`\`\`

> ⚠️ \`--host 0.0.0.0\` 暴露到公网时建议启用 API Key 鉴权，或仅在内网使用。
`;
