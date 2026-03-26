/**
 * REST API 服务启停 / 状态轮询逻辑
 *
 * 所有响应式 ref 由本 composable 统一创建，主入口组合后以 props/v-model 下发子组件，
 * 保证状态唯一来源、响应式不断链。
 */
import { ref } from "vue";
import { message } from "ant-design-vue";
import { api } from "@/api";

export function useApiServer() {
  const apiRunning = ref(false);
  const apiUrl = ref("");
  const apiPort = ref(8318);
  const apiHost = ref("0.0.0.0");
  const apiUseAuth = ref(false);
  const apiKey = ref("");
  const apiKeyInput = ref("");
  const apiKeyVisible = ref(false);
  const apiKeySaving = ref(false);
  const apiToggling = ref(false);
  const docsLoading = ref(true);

  async function checkStatus() {
    try {
      const s = await api.apiServerStatus();
      apiRunning.value = s.running;
      apiUrl.value = s.url;
      apiKey.value = s.api_key ?? "";
      apiKeyInput.value = s.api_key ?? "";
      if (s.running) apiPort.value = s.port;
    } catch { /* 忽略 */ }
  }

  async function toggleServer() {
    apiToggling.value = true;
    try {
      if (apiRunning.value) {
        await api.apiServerStop();
        apiRunning.value = false;
        apiUrl.value = "";
        message.success("REST API 服务已停止");
      } else {
        const s = await api.apiServerStart(apiPort.value, apiHost.value, apiUseAuth.value ? (apiKeyInput.value || undefined) : "");
        apiRunning.value = true;
        apiUrl.value = s.url;
        if (s.api_key) apiKey.value = s.api_key;
        message.success("REST API 服务已启动");
      }
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "操作失败");
    } finally {
      apiToggling.value = false;
    }
  }

  function copyText(text: string) {
    navigator.clipboard.writeText(text).then(
      () => message.success("已复制"),
      () => message.error("复制失败")
    );
  }

  async function saveApiKey() {
    if (apiKeyInput.value === apiKey.value) return;
    apiKeySaving.value = true;
    try {
      await api.setApiKeys(apiKeyInput.value, null);
      apiKey.value = apiKeyInput.value;
      message.success("API Key 已保存");
    } catch (e: any) {
      message.error(typeof e === "string" ? e : "保存失败");
    } finally {
      apiKeySaving.value = false;
    }
  }

  function openDocsInBrowser() {
    if (!apiUrl.value) return;
    const url = apiUrl.value + "/docs";
    api.openUrl(url);
  }

  return {
    apiRunning,
    apiUrl,
    apiPort,
    apiHost,
    apiUseAuth,
    apiKey,
    apiKeyInput,
    apiKeyVisible,
    apiKeySaving,
    apiToggling,
    docsLoading,
    checkStatus,
    toggleServer,
    copyText,
    saveApiKey,
    openDocsInBrowser,
  };
}
