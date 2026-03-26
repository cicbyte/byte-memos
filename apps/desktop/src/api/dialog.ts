import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { TagStat, UnifiedSearchHit } from "./types";

export const dialogApi = {
  ping: () => invoke<string>("ping"),

  /** 数据目录绝对路径（附件、备份等基础路径） */
  dataDir: () => invoke<string>("data_dir"),

  /** 在系统文件管理器中打开指定路径（目录直接打开，文件则定位并选中） */
  openInFolder: (path: string) => invoke<void>("open_in_folder", { path }),

  /** 标签使用统计（按 count 降序） */
  tagStats: (limit = 50) => invoke<TagStat[]>("tag_stats", { limit }),

  /** 跨类型统一搜索（备忘录命中，中文 LIKE 兜底） */
  searchAll: (query: string, limit = 20) =>
    invoke<UnifiedSearchHit[]>("search_all", { query, limit }),

  /** 让用户选一个目录（dialog 插件），返回绝对路径或 null */
  pickDirectory: async (): Promise<string | null> => {
    const selected = await open({ directory: true, multiple: false });
    if (Array.isArray(selected)) return selected[0] ?? null;
    return selected ?? null;
  },

  /** 让用户选单个文件，返回绝对路径或 null */
  pickFile: async (): Promise<string | null> => {
    const selected = await open({ directory: false, multiple: false });
    if (Array.isArray(selected)) return selected[0] ?? null;
    return selected ?? null;
  },

  /** 保存对话框：让用户选择导出路径（默认带日期戳的 zip 文件名），返回绝对路径或 null（取消） */
  pickSavePath: async (): Promise<string | null> => {
    const d = new Date();
    const stamp = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
    const path = await save({
      defaultPath: `byte-memos-backup-${stamp}.zip`,
      filters: [{ name: "ZIP 备份", extensions: ["zip"] }],
    });
    if (!path) return null;
    // 兜底：用户清空扩展名时补回 .zip（Rust 端按 .zip 扩展名判定为文件路径）
    return path.endsWith(".zip") ? path : `${path}.zip`;
  },

  /** 保存对话框（通用）：默认文件名 + 扩展名兜底，返回绝对路径或 null（取消） */
  pickExportPath: async (defaultName: string, ext: string): Promise<string | null> => {
    const path = await save({
      defaultPath: defaultName,
      filters: [{ name: ext.toUpperCase(), extensions: [ext] }],
    });
    if (!path) return null;
    return path.toLowerCase().endsWith(`.${ext}`) ? path : `${path}.${ext}`;
  },
};
