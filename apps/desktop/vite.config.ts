import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import { fileURLToPath, URL } from "node:url";

// Tauri 期望固定端口（14321；51420 曾落在 Windows WinNAT 保留段 51397-51496 报 EACCES，
// 5173 常被其他 dev server 占用，故选冷门端口），且禁用自动打开浏览器
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [vue()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  // Tauri 使用固定端口，Vite HMR 通过 websockets
  clearScreen: false,
  server: {
    port: 14321,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 14322,
        }
      : undefined,
    watch: {
      // 不监听 src-tauri（Rust 端改动由 tauri dev 自己处理）
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Tauri webview 支持现代 esbuild
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
    // vendor chunk 是有意的大块（长缓存 + 懒加载），500KB 默认线对它们没有意义
    chunkSizeWarningLimit: 2000,
    rollupOptions: {
      output: {
        // vendor 拆分：大依赖独立 chunk，主入口瘦身 + 长缓存（依赖不变则 chunk hash 不变）
        manualChunks(id: string) {
          if (id.includes("node_modules")) {
            if (id.includes("md-editor-v3") || id.includes("codemirror") || id.includes("highlight.js")) {
              return "vendor-md";
            }
            if (id.includes("ant-design-vue") || id.includes("@ant-design") || id.includes("@ctrl") || id.includes("dayjs")) {
              return "vendor-antd";
            }
            if (id.includes("vue") || id.includes("pinia")) {
              return "vendor-vue";
            }
          }
          return undefined;
        },
      },
    },
  },
}));
