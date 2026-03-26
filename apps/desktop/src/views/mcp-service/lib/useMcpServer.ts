/**
 * MCP 服务启停 / 状态轮询 / 配置逻辑
 *
 * 所有响应式 ref 由本 composable 统一创建，主入口组合后以 props/v-model 下发子组件，
 * 保证状态唯一来源、响应式不断链。函数体与 API 调用一字不动。
 */
import { ref } from "vue";
import { message } from "ant-design-vue";
import { api } from "@/api";

export function useMcpServer() {
  /** MCP 服务状态 */
  const mcpRunning = ref(false);
  const mcpUrl = ref("");
  const mcpPort = ref(8217);
  const mcpHost = ref("0.0.0.0");
  const mcpUseAuth = ref(false);
  const mcpKey = ref("");
  const mcpKeyInput = ref("");
  const mcpKeyVisible = ref(false);
  const mcpKeySaving = ref(false);
  const mcpToggling = ref(false);

  async function checkMcpStatus() {
    try {
      const s = await api.mcpServerStatus();
      mcpRunning.value = s.running;
      mcpUrl.value = s.url;
      mcpKey.value = s.mcp_key ?? "";
      mcpKeyInput.value = s.mcp_key ?? "";
      if (s.running) mcpPort.value = s.port;
    } catch { /* 忽略 */ }
  }

  async function toggleMcpServer() {
    mcpToggling.value = true;
    try {
      if (mcpRunning.value) {
        await api.mcpServerStop();
        mcpRunning.value = false;
        mcpUrl.value = "";
        message.success("MCP 服务已停止");
      } else {
        const url = await api.mcpServerStart(mcpPort.value, mcpHost.value, mcpUseAuth.value);
        mcpRunning.value = true;
        mcpUrl.value = url;
        message.success("MCP 服务已启动：" + url);
      }
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "操作失败");
    } finally {
      mcpToggling.value = false;
    }
  }

  function copyMcpUrl() {
    if (!mcpUrl.value) return;
    navigator.clipboard.writeText(mcpUrl.value).then(
      () => message.success("已复制端点 URL"),
      () => message.error("复制失败")
    );
  }

  function copyText(text: string) {
    navigator.clipboard.writeText(text).then(
      () => message.success("已复制"),
      () => message.error("复制失败")
    );
  }

  async function saveMcpKey() {
    if (mcpKeyInput.value === mcpKey.value) return;
    mcpKeySaving.value = true;
    try {
      await api.setApiKeys(null, mcpKeyInput.value);
      mcpKey.value = mcpKeyInput.value;
      message.success("MCP Key 已保存");
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "保存失败");
    } finally {
      mcpKeySaving.value = false;
    }
  }

  return {
    mcpRunning,
    mcpUrl,
    mcpPort,
    mcpHost,
    mcpUseAuth,
    mcpKey,
    mcpKeyInput,
    mcpKeyVisible,
    mcpKeySaving,
    mcpToggling,
    checkMcpStatus,
    toggleMcpServer,
    copyMcpUrl,
    copyText,
    saveMcpKey,
  };
}
