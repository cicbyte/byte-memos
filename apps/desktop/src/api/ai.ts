/**
 * AI API（byte-memos AI 域）
 *
 * - 供应商管理：ai_providers 表（api_key 不回显，仅 api_key_set 布尔）
 * - 流式动作：Tauri v2 Channel 逐增量回调；返回 StreamHandle{ done, cancel }
 *   取消 = ai_stream_cancel（Rust 侧 token 边界中断）+ 本地立即 reject
 */
import { invoke, Channel } from "@tauri-apps/api/core";

// ===== 类型 =====

export interface AiProviderItem {
  id: number;
  name: string;
  provider_key: string;
  base_url: string;
  api_key_set: boolean;
  model: string;
  max_tokens: number;
  temperature: number;
  is_enabled: boolean;
  sort_order: number;
}

export interface AiAddProviderInput {
  name: string;
  provider_key: string;
  base_url: string;
  api_key: string;
  model: string;
  max_tokens: number;
  temperature: number;
}

export interface AiUpdateProviderInput {
  id: number;
  name: string;
  base_url: string;
  api_key: string;
  model: string;
  max_tokens: number;
  temperature: number;
  is_enabled: boolean;
}

export interface AiCheckResult {
  ok: boolean;
  message: string;
}

export interface AiActionInfo {
  action: string;
  is_json: boolean;
}

/** Channel 信封（与 core StreamChunk 一一对应） */
export type AiStreamChunk =
  | { type: "token"; content: string }
  | { type: "thinking"; content: string }
  | { type: "done"; full: string }
  | { type: "error"; message: string };

export interface AiStreamInput {
  action: string;
  content: string;
  title: string;
  lang: string;
}

export interface StreamHandle {
  /** resolve = 聚合全文；reject = 错误或取消（取消消息为「已取消」） */
  done: Promise<string>;
  cancel: () => void;
}

export const CANCELLED_MSG = "已取消";

// ===== 供应商管理 =====

export const aiProviderApi = {
  list: () => invoke<AiProviderItem[]>("ai_provider_list"),
  add: (input: AiAddProviderInput) => invoke<number>("ai_provider_add", { input }),
  update: (input: AiUpdateProviderInput) => invoke<void>("ai_provider_update", { input }),
  delete: (id: number) => invoke<void>("ai_provider_delete", { id }),
  check: (id: number) => invoke<AiCheckResult>("ai_provider_check", { id }),
  actions: () => invoke<AiActionInfo[]>("ai_actions"),
};

// ===== 流式动作 =====

/** 发起流式 AI 动作；onChunk 逐增量回调（token/thinking/done/error） */
export function streamAi(input: AiStreamInput, onChunk: (chunk: AiStreamChunk) => void): StreamHandle {
  const requestId = crypto.randomUUID();
  const channel = new Channel<AiStreamChunk>();
  channel.onmessage = onChunk;

  let localReject: ((reason: unknown) => void) | null = null;
  const done = new Promise<string>((resolve, reject) => {
    localReject = reject;
    invoke<string>("ai_stream", {
      action: input.action,
      content: input.content,
      title: input.title,
      lang: input.lang,
      requestId,
      onChunk: channel,
    })
      .then(resolve)
      .catch((e) => reject(e instanceof Error ? e : new Error(String(e))));
  });

  return {
    done,
    cancel: () => {
      // Rust 侧 token 边界中断 + 本地立即 reject（不等 Rust 返回）
      void invoke("ai_stream_cancel", { requestId }).catch(() => {});
      localReject?.(new Error(CANCELLED_MSG));
    },
  };
}

/** 判断错误是否为用户取消（非真错误，不弹错误提示） */
export function isCancelledError(e: unknown): boolean {
  return e instanceof Error && e.message === CANCELLED_MSG;
}
