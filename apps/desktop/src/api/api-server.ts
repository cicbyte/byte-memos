import { invoke } from "@tauri-apps/api/core";

export const apiServerApi = {
  // ============ REST API 服务管理 ============

  /** 启动 REST API 服务 */
  apiServerStart: (port: number, host?: string, customKey?: string) =>
    invoke<{
      running: boolean;
      port: number;
      url: string;
      api_key?: string;
    }>("start_api_server", { host: host ?? null, port, customKey: customKey ?? null }),

  /** 停止 REST API 服务 */
  apiServerStop: () => invoke<void>("stop_api_server"),

  /** 查询 REST API 服务状态 + API Key */
  apiServerStatus: () =>
    invoke<{
      running: boolean;
      port: number;
      url: string;
      api_key?: string;
    }>("api_server_status"),

  // ============ 移动端扫码配对 ============

  /** 配对信息（局域网 IP + 端口 + Key），生成 bmemos://pair 二维码用 */
  mobilePairingInfo: () =>
    invoke<{
      v: number;
      name: string;
      host: string;
      port: number;
      key: string;
      running: boolean;
    }>("mobile_pairing_info"),
};

/** 组装配对串（与移动端 parsePairing 对齐） */
export function buildPairingString(p: {
  v: number;
  name: string;
  host: string;
  port: number;
  key: string;
}): string {
  const q = new URLSearchParams({
    v: String(p.v),
    name: p.name,
    host: p.host,
    port: String(p.port),
    key: p.key,
  });
  return `bmemos://pair?${q.toString()}`;
}
