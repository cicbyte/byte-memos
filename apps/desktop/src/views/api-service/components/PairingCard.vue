<script setup lang="ts">
/**
 * 移动端扫码配对卡片
 *
 * 展示 bmemos://pair 二维码（qrcode 库渲染 canvas）+ 配对串文本。
 * 移动端 App「我的 → 添加连接 → 扫码配对」扫描即得连接配置；
 * 服务未运行时给出提示（二维码仍可见，但连不上）。
 */
import { ref, onMounted, watch } from "vue";
import QRCode from "qrcode";
import { apiServerApi, buildPairingString } from "@/api/api-server";

const emit = defineEmits<{ copy: [text: string] }>();
const props = defineProps<{ runningKey: number }>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const pairingText = ref("");
const hostDisplay = ref("");
const portDisplay = ref(0);
const running = ref(false);
const loading = ref(true);

async function refresh() {
  loading.value = true;
  try {
    const info = await apiServerApi.mobilePairingInfo();
    pairingText.value = buildPairingString(info);
    hostDisplay.value = info.host;
    portDisplay.value = info.port;
    running.value = info.running;
    if (canvasRef.value) {
      await QRCode.toCanvas(canvasRef.value, pairingText.value, {
        width: 190,
        margin: 1,
        color: { dark: "#111827ff", light: "#ffffffff" },
        errorCorrectionLevel: "M",
      });
    }
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);
watch(() => props.runningKey, refresh);
</script>

<template>
  <div class="pairing-card">
    <div class="head">
      <div class="title">移动端配对</div>
      <div class="sub">byte-memos App → 我的 → 添加连接 → 扫码配对</div>
    </div>
    <div class="body">
      <canvas ref="canvasRef" class="qr" :class="{ dim: !running }" />
      <div class="meta">
        <div class="row"><span class="k">地址</span><span class="v mono">http://{{ hostDisplay }}:{{ portDisplay }}/api/v1</span></div>
        <div class="row"><span class="k">设备</span><span class="v">{{ running ? '服务运行中' : '服务未启动（先启动再扫码）' }}</span></div>
        <button class="copy-btn" @click="emit('copy', pairingText)">复制配对串</button>
        <div class="hint">同一 Wi-Fi 下手机直连；配对串含 API Key，请勿外传。</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.pairing-card {
  border: 1px solid var(--border-light);
  border-radius: 10px;
  padding: 14px 16px;
  background: var(--bg-secondary, rgba(127, 127, 127, 0.06));
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.head .title { font-size: 13px; font-weight: 600; color: var(--text-primary); }
.head .sub { font-size: 12px; color: var(--text-tertiary); margin-top: 2px; }
.body { display: flex; gap: 16px; align-items: flex-start; }
.qr { border-radius: 8px; border: 1px solid var(--border-light); }
.qr.dim { opacity: 0.45; filter: grayscale(0.6); }
.meta { display: flex; flex-direction: column; gap: 6px; flex: 1; min-width: 0; }
.row { display: flex; gap: 8px; font-size: 12px; align-items: baseline; }
.row .k { color: var(--text-tertiary); flex-shrink: 0; }
.row .v { color: var(--text-primary); }
.mono { font-family: var(--font-mono, monospace); word-break: break-all; }
.copy-btn {
  align-self: flex-start; margin-top: 4px;
  padding: 4px 12px; font-size: 12px; border-radius: 6px; cursor: pointer;
  border: 1px solid var(--border-light); background: transparent; color: var(--color-primary);
  transition: background var(--transition-fast);
}
.copy-btn:hover { background: var(--color-primary); color: #fff; }
.hint { font-size: 11px; color: var(--text-tertiary); }
</style>
