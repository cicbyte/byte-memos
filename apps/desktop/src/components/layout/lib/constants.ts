/**
 * Sidebar 静态菜单数据 + 类型 + 纯工具函数
 *
 * 从 Sidebar.vue 抽出，数据为只读共享（非响应式状态，无副本问题）。
 */
import {
  DashboardOutlined,
  AppstoreOutlined,
  TagsOutlined,
  StarOutlined,
  SettingOutlined,
  FolderOutlined,
  DeleteOutlined,
  CloudServerOutlined,
  GlobalOutlined,
  LinkOutlined,
} from "@ant-design/icons-vue";

export type IconComp = typeof DashboardOutlined;
export interface MenuItem {
  key: string;
  icon: IconComp;
}
export interface MenuGroup {
  key: string;
  label: string;
  icon: IconComp;
  children: MenuItem[];
}

// 独立顶层项（不属任何分组，置于菜单最顶部）
export const topItems: MenuItem[] = [
  { key: "/", icon: DashboardOutlined },
];

export const menuGroups: MenuGroup[] = [
  {
    key: "metadata",
    label: "元数据",
    icon: TagsOutlined,
    children: [
      { key: "/categories", icon: FolderOutlined },
      { key: "/tags", icon: TagsOutlined },
    ],
  },
  {
    key: "browse",
    label: "内容",
    icon: AppstoreOutlined,
    children: [
      { key: "/bookmarks", icon: LinkOutlined },
      { key: "/favorites", icon: StarOutlined },
      { key: "/trash", icon: DeleteOutlined },
    ],
  },
  {
    key: "services",
    label: "服务",
    icon: CloudServerOutlined,
    children: [
      { key: "/mcp-service", icon: GlobalOutlined },
      { key: "/api-service", icon: CloudServerOutlined },
    ],
  },
  {
    key: "system",
    label: "系统",
    icon: SettingOutlined,
    children: [{ key: "/settings", icon: SettingOutlined }],
  },
];

export const labelOf: Record<string, string> = {
  "/": "仪表盘",
  "/bookmarks": "书签",
  "/tags": "标签",
  "/categories": "分类",
  "/favorites": "收藏",
  "/trash": "回收站",
  "/mcp-service": "MCP 服务",
  "/api-service": "REST API",
  "/settings": "设置",
};

export function groupOf(path: string): string | null {
  for (const g of menuGroups) {
    if (g.children.some((c) => c.key === path)) return g.key;
  }
  return null;
}
