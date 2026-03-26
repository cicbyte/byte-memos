/**
 * 附件 API（byte-memos 附件管理域）
 *
 * 封装 src-tauri/src/commands/attachments.rs 的 6 个命令，
 * 并提供预览用的 ObjectURL 缓存（同一附件只跨 IPC 拉一次字节）。
 */
import { invoke } from "@tauri-apps/api/core";
import type { AttachmentContent, AttachmentInfo, AttachmentOverview } from "./types";

export const attachmentsApi = {
  /** 上传附件（FILE 模式：字节落盘 data/attachments/{shortId}/） */
  attachmentUpload: (memoShortId: string, filename: string, mimeType: string, content: Uint8Array) =>
    invoke<AttachmentInfo>("attachment_upload", {
      memoShortId,
      filename,
      mimeType,
      content: Array.from(content),
    }),

  /** 列出某备忘录的全部附件 */
  attachmentList: (memoShortId: string) =>
    invoke<AttachmentInfo[]>("attachment_list", { memoShortId }),

  /** 列出全部附件（资源库，带 orphan 游离标记） */
  attachmentListAll: () => invoke<AttachmentOverview[]>("attachment_list_all"),

  /** 读取附件字节内容（预览/下载用） */
  attachmentContent: (id: number) => invoke<AttachmentContent>("attachment_content", { id }),

  /** 删除附件（连带磁盘文件） */
  attachmentDelete: (id: number) => invoke<void>("attachment_delete", { id }),

  /** 附件另存为（返回写入路径） */
  attachmentSave: (id: number, path: string) => invoke<string>("attachment_save", { id, path }),
};

// ===== 预览 ObjectURL 缓存 =====

const urlCache = new Map<number, string>();

/** 获取附件的 blob ObjectURL（缓存；删除附件后请 revokeAttachmentUrl） */
export async function attachmentObjectUrl(id: number): Promise<string> {
  const hit = urlCache.get(id);
  if (hit) return hit;
  const c = await invoke<AttachmentContent>("attachment_content", { id });
  const url = URL.createObjectURL(
    new Blob([new Uint8Array(c.bytes)], { type: c.mime_type || "application/octet-stream" }),
  );
  urlCache.set(id, url);
  return url;
}

/** 释放缓存的 ObjectURL（删除附件时调用） */
export function revokeAttachmentUrl(id: number) {
  const u = urlCache.get(id);
  if (u) {
    URL.revokeObjectURL(u);
    urlCache.delete(id);
  }
}

/** 是否图片（可缩略图预览） */
export function isImageMime(mime: string): boolean {
  return mime.startsWith("image/");
}

/** 是否音频 */
export function isAudioMime(mime: string): boolean {
  return mime.startsWith("audio/");
}

/** 附件大小友好格式 */
export function fmtSize(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(1)} MB`;
}
