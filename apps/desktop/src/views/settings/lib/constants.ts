/**
 * 设置页静态常量：默认排除目录。
 *
 * 说明：原 settings 历史 MCP 区块（readTools/writeTools/mcpGuide 等工具定义与配置文案）
 * 已随该区块一并移除——MCP 服务功能由独立的 @/views/mcp-service 页面承接。
 */

/** 偏好：预置默认排除目录列表 */
export const DEFAULT_EXCLUDE_DIRS = [
  "node_modules", ".git", "target", "dist", "build",
  ".next", ".nuxt", "out", ".output", "__pycache__",
  ".venv", "venv", ".idea", ".vscode", ".DS_Store",
];
