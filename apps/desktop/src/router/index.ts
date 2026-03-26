import { createRouter, createWebHashHistory } from "vue-router";

const routes = [
  {
    path: "/",
    name: "dashboard",
    component: () => import("@/views/dashboard/index.vue"),
    meta: { title: "仪表盘", fullsize: true },
  },
  {
    path: "/bookmarks",
    name: "bookmarks",
    component: () => import("@/views/bookmarks/index.vue"),
    meta: { title: "书签", group: "内容", fullsize: true },
  },
  {
    path: "/tags",
    name: "tags",
    component: () => import("@/views/tags/index.vue"),
    meta: { title: "标签", group: "元数据", fullsize: true },
  },
  {
    path: "/categories",
    name: "categories",
    component: () => import("@/views/categories/index.vue"),
    meta: { title: "分类", group: "元数据", fullsize: true },
  },
  {
    path: "/favorites",
    name: "favorites",
    component: () => import("@/views/favorites/index.vue"),
    meta: { title: "收藏", group: "整理", fullsize: true },
  },
  {
    path: "/trash",
    name: "trash",
    component: () => import("@/views/trash/index.vue"),
    meta: { title: "回收站", group: "整理", fullsize: true },
  },
  {
    path: "/mcp-service",
    name: "mcp-service",
    component: () => import("@/views/mcp-service/index.vue"),
    meta: { title: "MCP 服务", group: "服务", fullsize: true },
  },
  {
    path: "/api-service",
    name: "api-service",
    component: () => import("@/views/api-service/index.vue"),
    meta: { title: "REST API", group: "服务", fullsize: true },
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("@/views/settings/index.vue"),
    meta: { title: "设置", group: "系统", fullsize: true },
  },
];

const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

export default router;
