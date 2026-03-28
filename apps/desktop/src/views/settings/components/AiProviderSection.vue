<script setup lang="ts">
/**
 * 设置 · AI 供应商分区（参考 byte-voice ModelSettings：多栏 master-detail）
 *
 * - 左栏：供应商列表（单选启用 radio + 选中态左边条），头部「+ 添加」
 * - 右栏：就地编辑详情（无弹框），600ms 防抖自动保存（api_key 空串=保留原值）
 * - 连通测试内联结果行；删除带确认
 */
import { onMounted, ref, reactive, computed, watch } from "vue";
import { message } from "ant-design-vue";
import {
  PlusOutlined,
  DeleteOutlined,
  ApiOutlined,
  CheckCircleFilled,
  CloseCircleFilled,
  LoadingOutlined,
} from "@ant-design/icons-vue";
import { api } from "@/api";
import type { AiProviderItem } from "@/api/ai";

const PROVIDER_OPTIONS = [
  { value: "openai-compatible", label: "OpenAI 兼容（含中转站）" },
  { value: "anthropic", label: "Anthropic Claude" },
  { value: "gemini", label: "Google Gemini" },
  { value: "ollama", label: "Ollama（本地）" },
];

const items = ref<AiProviderItem[]>([]);
const loading = ref(false);
const selectedId = ref<number | null>(null);
const checking = ref(false);
const checkResult = ref<{ ok: boolean; message: string } | null>(null);
const saving = ref(false);

const form = reactive({
  name: "",
  base_url: "",
  api_key: "",
  model: "",
  max_tokens: 4096,
  temperature: 0.7,
});

const current = computed(() => items.value.find((p) => p.id === selectedId.value) ?? null);

// ===== 加载 / 选择 =====

async function load(select?: number) {
  loading.value = true;
  try {
    items.value = await api.list();
    if (select != null && items.value.some((p) => p.id === select)) {
      selectedId.value = select;
    } else if (!items.value.some((p) => p.id === selectedId.value)) {
      selectedId.value = items.value[0]?.id ?? null;
    }
    syncForm();
  } catch {
    message.error("加载 AI 供应商失败");
  } finally {
    loading.value = false;
  }
}

function selectProvider(id: number) {
  if (id === selectedId.value) return;
  void saveNow(); // 切走前冲刷未保存的编辑
  selectedId.value = id;
  checkResult.value = null;
  syncForm();
}

/** 把当前选中供应商同步进表单（syncing 抑制自动保存） */
let syncing = false;
function syncForm() {
  const p = current.value;
  if (!p) return;
  syncing = true;
  form.name = p.name;
  form.base_url = p.base_url;
  form.api_key = ""; // 空 = 保留原值
  form.model = p.model;
  form.max_tokens = p.max_tokens;
  form.temperature = p.temperature;
  void Promise.resolve().then(() => {
    syncing = false;
  });
}

// ===== 自动保存（600ms 防抖，沿袭 byte-voice） =====

let saveTimer: ReturnType<typeof setTimeout> | null = null;
let dirty = false;

function scheduleSave() {
  if (syncing) return;
  dirty = true;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => void saveNow(), 600);
}

async function saveNow() {
  const id = selectedId.value;
  if (id == null || !dirty || saving.value) return;
  if (!form.name.trim()) return; // 名称为空不保存，避免存出空名
  saving.value = true;
  dirty = false;
  try {
    const wasEnabled = items.value.find((p) => p.id === id)?.is_enabled ?? false;
    await api.update({
      id,
      name: form.name.trim(),
      base_url: form.base_url.trim(),
      api_key: form.api_key, // 空串 = 保留原值（后端约定）
      model: form.model.trim(),
      max_tokens: form.max_tokens,
      temperature: form.temperature,
      is_enabled: wasEnabled,
    });
    // 本地同步列表显示（名称/模型/Key 标记）
    const it = items.value.find((p) => p.id === id);
    if (it) {
      it.name = form.name.trim();
      it.model = form.model.trim();
      it.base_url = form.base_url.trim();
      if (form.api_key) it.api_key_set = true;
    }
  } catch (e) {
    message.error(`保存失败：${e}`);
  } finally {
    saving.value = false;
  }
}

watch(form, scheduleSave, { deep: true });

// ===== 添加 / 删除 / 启用 / 测试 =====

async function addProvider() {
  try {
    const id = await api.add({
      name: "新供应商",
      provider_key: "openai-compatible",
      base_url: "",
      api_key: "",
      model: "",
      max_tokens: 4096,
      temperature: 0.7,
    });
    checkResult.value = null;
    await load(id);
    message.success("已添加，请在右侧填写配置");
  } catch (e) {
    message.error(`添加失败：${e}`);
  }
}

async function removeProvider() {
  const p = current.value;
  if (!p) return;
  try {
    await api.delete(p.id);
    message.success("已删除");
    checkResult.value = null;
    await load();
  } catch (e) {
    message.error(`删除失败：${e}`);
  }
}

/** 启用为单选语义：开后端自动停用其余；关闭 = 仅停用自身 */
async function setEnabled(p: AiProviderItem, enabled: boolean) {
  try {
    await api.update({
      id: p.id,
      name: p.name,
      base_url: p.base_url,
      api_key: "",
      model: p.model,
      max_tokens: p.max_tokens,
      temperature: p.temperature,
      is_enabled: enabled,
    });
    await load(p.id);
  } catch (e) {
    message.error(`操作失败：${e}`);
  }
}

async function check() {
  const p = current.value;
  if (!p) return;
  await saveNow(); // 用最新表单值检测
  checking.value = true;
  checkResult.value = null;
  try {
    checkResult.value = await api.check(p.id);
  } catch (e) {
    checkResult.value = { ok: false, message: `检测失败：${e}` };
  } finally {
    checking.value = false;
  }
}

function providerLabel(key: string): string {
  return PROVIDER_OPTIONS.find((o) => o.value === key)?.label ?? key;
}

onMounted(() => load());
</script>

<template>
  <section class="pane pane-ai">
    <div class="section-head">
      <h2 class="pane-title">AI 供应商</h2>
      <span class="section-hint">
        API Key 加密存放在本机、不会回显；同一时间只有一个供应商生效（radio 点亮）。
      </span>
    </div>

    <div class="ai-columns">
      <!-- 左：供应商列表 -->
      <aside class="provider-panel">
        <div class="panel-header">
          <span class="panel-title">供应商</span>
          <span class="provider-count">{{ items.length }}</span>
          <span class="flex1" />
          <button class="add-btn" title="添加供应商" :disabled="loading" @click="addProvider">
            <PlusOutlined />
          </button>
        </div>
        <a-spin :spinning="loading">
          <div class="provider-list">
            <div
              v-for="p in items"
              :key="p.id"
              class="provider-card"
              :class="{ active: selectedId === p.id }"
              @click="selectProvider(p.id)"
            >
              <span
                class="radio-dot"
                :class="{ on: p.is_enabled }"
                title="设为启用"
                @click.stop="setEnabled(p, true)"
              />
              <div class="p-info">
                <div class="p-name">{{ p.name || "（未命名）" }}</div>
                <div class="p-meta">
                  {{ providerLabel(p.provider_key) }}<template v-if="p.model"> · {{ p.model }}</template>
                </div>
              </div>
              <CheckCircleFilled v-if="p.is_enabled" class="on-mark" title="当前启用" />
            </div>
            <div v-if="!items.length" class="provider-empty">还没有供应商<br />点右上 + 添加</div>
          </div>
        </a-spin>
      </aside>

      <!-- 右：详情（就地编辑 + 自动保存） -->
      <div v-if="current" class="detail-panel">
        <div class="detail-header">
          <input v-model="form.name" class="name-input" placeholder="供应商名称" />
          <div class="header-ops">
            <button class="op" :disabled="checking" @click="check">
              <ApiOutlined />
              <LoadingOutlined v-if="checking" /> {{ checking ? "检测中…" : "测试连通" }}
            </button>
            <a-popconfirm title="删除该供应商？" @confirm="removeProvider">
              <button class="op danger"><DeleteOutlined /></button>
            </a-popconfirm>
          </div>
        </div>

        <div v-if="checkResult" class="connect-result" :class="checkResult.ok ? 'ok' : 'bad'">
          <CheckCircleFilled v-if="checkResult.ok" />
          <CloseCircleFilled v-else />
          <span>{{ checkResult.message }}</span>
        </div>

        <div class="setting-group">
          <div class="setting-title-row">
            <span>类型</span>
            <span class="type-chip" :title="'类型创建后不可改，如需更换请新建'">{{ providerLabel(current.provider_key) }}</span>
          </div>
        </div>

        <div class="setting-group">
          <div class="setting-title">API 端点</div>
          <a-input
            v-model:value="form.base_url"
            placeholder="留空 = 官方地址；OpenAI 兼容可填 https://api.deepseek.com 或完整端点"
            size="small"
          />
        </div>

        <div class="setting-group">
          <div class="setting-title-row">
            <span>API 密钥</span>
            <span class="key-flag" :class="{ ok: current.api_key_set }">
              {{ current.api_key_set ? "已设置" : "未设置" }}
            </span>
          </div>
          <a-input-password
            v-model:value="form.api_key"
            :placeholder="current.api_key_set ? '留空 = 保留原值' : '输入 API 密钥'"
            size="small"
            autocomplete="new-password"
          />
        </div>

        <div class="setting-group">
          <div class="setting-title">模型</div>
          <a-input v-model:value="form.model" placeholder="如 deepseek-chat / gpt-4o-mini / qwen2.5:7b" size="small" />
        </div>

        <div class="setting-group pair">
          <label class="half">
            <span class="setting-title">max_tokens</span>
            <a-input-number v-model:value="form.max_tokens" :min="256" :max="200000" :step="256" size="small" style="width: 100%" />
          </label>
          <label class="half">
            <span class="setting-title">temperature</span>
            <a-input-number v-model:value="form.temperature" :min="0" :max="2" :step="0.1" size="small" style="width: 100%" />
          </label>
        </div>

        <div class="setting-group row-inline">
          <div>
            <div class="row-label">启用此供应商</div>
            <div class="row-desc">停用后备忘录的 AI 动作将不可用</div>
          </div>
          <a-switch
            :checked="current.is_enabled"
            size="small"
            @change="(v: any) => setEnabled(current!, Boolean(v))"
          />
        </div>

        <div class="autosave-note">{{ saving ? "保存中…" : "更改自动保存" }}</div>
      </div>

      <div v-else class="empty-state">
        <ApiOutlined class="empty-icon" />
        <p>选择左侧供应商查看配置</p>
      </div>
    </div>
  </section>
</template>

<style scoped>
.section-head {
  display: flex;
  align-items: baseline;
  gap: 12px;
  margin-bottom: 10px;
}
.pane-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
  white-space: nowrap;
}
.section-hint {
  font-size: 11.5px;
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ===== 两栏骨架（参考 byte-voice ModelSettings） ===== */
.pane-ai {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.ai-columns {
  flex: 1;
  min-height: 0;
  display: flex;
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  background: var(--bg-surface);
  overflow: hidden;
}

/* 左栏：列表 */
.provider-panel {
  width: 216px;
  min-width: 216px;
  border-right: 1px solid var(--border-light);
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  align-self: stretch;
}
.panel-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 12px 8px;
  user-select: none;
}
.panel-title {
  font-size: 11.5px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-tertiary);
}
.provider-count {
  font-size: 10.5px;
  color: var(--text-tertiary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  border-radius: 999px;
  padding: 0 7px;
  line-height: 16px;
}
.flex1 {
  flex: 1;
}
.add-btn {
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  font-size: 12px;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: filter var(--transition-fast);
}
.add-btn:hover {
  filter: brightness(1.1);
}
.add-btn:disabled {
  opacity: 0.5;
  cursor: default;
}
.provider-list {
  flex: 1;
  overflow-y: auto;
  padding: 0 6px 8px;
}
.provider-card {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 8px 9px;
  cursor: pointer;
  border-radius: var(--radius-sm);
  border-left: 3px solid transparent;
  transition: background var(--transition-fast), border-color var(--transition-fast);
  margin-bottom: 2px;
}
.provider-card:hover {
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
}
.provider-card.active {
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  border-left-color: var(--color-primary);
}
.radio-dot {
  width: 13px;
  height: 13px;
  border-radius: 50%;
  border: 1.5px solid var(--text-tertiary);
  transition: all var(--transition-fast);
  flex-shrink: 0;
  cursor: pointer;
}
.radio-dot:hover {
  border-color: var(--color-primary);
}
.radio-dot.on {
  border-color: var(--color-primary);
  background: radial-gradient(circle, var(--color-primary) 40%, transparent 46%);
}
.p-info {
  flex: 1;
  min-width: 0;
}
.p-name {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.p-meta {
  font-size: 10.5px;
  color: var(--text-tertiary);
  margin-top: 1px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.on-mark {
  color: var(--color-primary);
  font-size: 12px;
  flex-shrink: 0;
}
.provider-empty {
  padding: 28px 10px;
  text-align: center;
  font-size: 12px;
  line-height: 1.8;
  color: var(--text-tertiary);
}

/* 右栏：详情 */
.detail-panel {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 16px 20px;
}
.detail-header {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--border-light);
  margin-bottom: 14px;
}
.name-input {
  flex: 1;
  min-width: 0;
  height: 30px;
  font-size: 15px;
  font-weight: 600;
  font-family: inherit;
  color: var(--text-primary);
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  padding: 0 8px;
  transition: border-color var(--transition-fast), background var(--transition-fast);
}
.name-input:hover {
  border-color: var(--border-light);
}
.name-input:focus {
  outline: none;
  border-color: var(--color-primary);
  background: var(--bg-input);
}
.name-input::placeholder {
  color: var(--text-tertiary);
  font-weight: 400;
}
.header-ops {
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.op {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 26px;
  padding: 0 11px;
  font-size: 12px;
  color: var(--text-tertiary);
  background: transparent;
  border: none;
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.op:hover {
  color: var(--text-primary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
}
.op:disabled {
  opacity: 0.5;
  cursor: default;
}
.op.danger:hover {
  color: #ef4444;
  background: color-mix(in srgb, #ef4444 10%, transparent);
}

.connect-result {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  margin-bottom: 12px;
}
.connect-result.ok {
  color: #16a34a;
  background: color-mix(in srgb, #16a34a 8%, transparent);
}
.connect-result.bad {
  color: #ef4444;
  background: color-mix(in srgb, #ef4444 8%, transparent);
}

/* 设置分组 */
.setting-group {
  margin-bottom: 16px;
}
.setting-group.pair {
  display: flex;
  gap: 14px;
}
.half {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.setting-title,
.setting-title-row {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 6px;
}
.setting-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
}
.type-chip {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  border-radius: 999px;
  padding: 1px 9px;
  cursor: default;
}
.key-flag {
  font-size: 11px;
  color: var(--text-tertiary);
}
.key-flag.ok {
  color: #16a34a;
}
.row-inline {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 12px;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-sm);
}
.row-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-primary);
}
.row-desc {
  font-size: 11px;
  color: var(--text-tertiary);
  margin-top: 2px;
}
.autosave-note {
  font-size: 10.5px;
  color: var(--text-tertiary);
  text-align: right;
  user-select: none;
}

/* 空态 */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--text-tertiary);
}
.empty-icon {
  font-size: 30px;
  opacity: 0.3;
}
.empty-state p {
  font-size: 12.5px;
  margin: 0;
}
</style>
