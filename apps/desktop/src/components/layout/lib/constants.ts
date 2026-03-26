/**
 * Sidebar 静态菜单数据 + 类型 + 纯工具函数
 *
 * 从 Sidebar.vue 抽出，数据为只读共享（非响应式状态，无副本问题）。
 */
import {
  DashboardOutlined,
  TagsOutlined,
  PushpinOutlined,
  SettingOutlined,
  FolderOutlined,
  DeleteOutlined,
  CloudServerOutlined,
  GlobalOutlined,
  FileTextOutlined,
} from "@ant-design/icons-vue";

export type IconComp = typeof DashboardOutlined;
export interface MenuItem {
  key: string;
  icon: IconComp;
}
export interface MenuGroup {
  key: string;
  label: string;
  children: MenuItem[];
}

// 独立顶层项（不属任何分组，置于菜单最顶部）
export const topItems: MenuItem[] = [
  { key: "/", icon: DashboardOutlined },
];

export const menuGroups: MenuGroup[] = [
  {
    key: "browse",
    label: "内容",
    children: [
      { key: "/memos", icon: FileTextOutlined },
      { key: "/favorites", icon: PushpinOutlined },
      { key: "/trash", icon: DeleteOutlined },
    ],
  },
  {
    key: "metadata",
    label: "元数据",
    children: [
      { key: "/categories", icon: FolderOutlined },
      { key: "/tags", icon: TagsOutlined },
    ],
  },
  {
    key: "services",
    label: "服务",
    children: [
      { key: "/mcp-service", icon: GlobalOutlined },
      { key: "/api-service", icon: CloudServerOutlined },
    ],
  },
  {
    key: "system",
    label: "系统",
    children: [{ key: "/settings", icon: SettingOutlined }],
  },
];

export const labelOf: Record<string, string> = {
  "/": "仪表盘",
  "/memos": "备忘录",
  "/tags": "标签",
  "/categories": "分类",
  "/favorites": "置顶",
  "/trash": "回收站",
  "/mcp-service": "MCP 服务",
  "/api-service": "REST API",
  "/settings": "设置",
};
