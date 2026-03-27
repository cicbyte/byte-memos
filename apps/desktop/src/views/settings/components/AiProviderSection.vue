<script setup lang="ts">
/**
 * 设置 · AI 供应商分区
 *
 * - 列表：单选启用（启用即自动停用其余，后端事务保证）、api_key 只显示「已设置」
 * - 新增/编辑 Modal：api_key 编辑时留空 = 保留原值
 * - 连通测试：发最小 prompt 验证
 */
import { onMounted, ref, reactive } from "vue";
import { message } from "ant-design-vue";
import {
  PlusOutlined,
  EditOutlined,
  DeleteOutlined,
  ApiOutlined,
  CheckCircleFilled,
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
const checkingId = ref<number | null>(null);

const modalOpen = ref(false);
const editingId = ref<number | null>(null);
const saving = ref(false);
const form = reactive({
  name: "",
  provider_key: "openai-compatible",
  base_url: "",
  api_key: "",
  model: "",
  max_tokens: 4096,
  temperature: 0.7,
});

async function load() {
  loading.value = true;
  try {
    items.value = await api.list();
  } catch {
    message.error("加载 AI 供应商失败");
  } finally {
    loading.value = false;
  }
}

function openAdd() {
  editingId.value = null;
  Object.assign(form, {
    name: "",
    provider_key: "openai-compatible",
    base_url: "",
    api_key: "",
    model: "",
    max_tokens: 4096,
    temperature: 0.7,
  });
  modalOpen.value = true;
}

function openEdit(p: AiProviderItem) {
  editingId.value = p.id;
  Object.assign(form, {
    name: p.name,
    provider_key: p.provider_key,
    base_url: p.base_url,
    api_key: "", // 空 = 保留原值
    model: p.model,
    max_tokens: p.max_tokens,
    temperature: p.temperature,
  });
  modalOpen.value = true;
}

async function save() {
  if (!form.name.trim()) return message.warning("请填写名称");
  if (!form.model.trim()) return message.warning("请填写模型名");
  if (editingId.value === null && !form.api_key.trim() && form.provider_key !== "ollama") {
    return message.warning("请填写 API Key");
  }
  saving.value = true;
  try {
    if (editingId.value === null) {
      await api.add({ ...form });
      message.success("已添加");
    } else {
      await api.update({ id: editingId.value, ...form, is_enabled: isEnabled(editingId.value) });
      message.success("已保存");
    }
    modalOpen.value = false;
    await load();
  } catch (e) {
    message.error(`保存失败：${e}`);
  } finally {
    saving.value = false;
  }
}

function isEnabled(id: number): boolean {
  return items.value.find((p) => p.id === id)?.is_enabled ?? false;
}

/** 切换启用（单选）：启用传 true（后端自动停用其余）；再点一次已启用的不取消 */
async function enable(p: AiProviderItem) {
  if (p.is_enabled) return;
  try {
    await api.update({
      id: p.id,
      name: p.name,
      base_url: p.base_url,
      api_key: "", // 保留原值
      model: p.model,
      max_tokens: p.max_tokens,
      temperature: p.temperature,
      is_enabled: true,
    });
    await load();
  } catch (e) {
    message.error(`启用失败：${e}`);
  }
}

async function remove(p: AiProviderItem) {
  try {
    await api.delete(p.id);
    message.success("已删除");
    await load();
  } catch (e) {
    message.error(`删除失败：${e}`);
  }
}

async function check(p: AiProviderItem) {
  checkingId.value = p.id;
  try {
    const r = await api.check(p.id);
    if (r.ok) message.success(`${p.name}：${r.message}`);
    else message.error(`${p.name}：${r.message}`);
  } catch (e) {
    message.error(`测试失败：${e}`);
  } finally {
    checkingId.value = null;
  }
}

function providerLabel(key: string): string {
  return PROVIDER_OPTIONS.find((o) => o.value === key)?.label ?? key;
}

onMounted(load);
</script>

<template>
  <section class="pane">
    <div class="section-head">
      <h2 class="pane-title">AI 供应商</h2>
      <button class="add-btn" @click="openAdd"><PlusOutlined /> 添加供应商</button>
    </div>
    <p class="section-hint">
      配置一个大模型服务即可使用备忘录的 AI 动作（整理 / 摘要 / 润色 / 翻译 / 标签建议…）。
      API Key 加密存放在本机，不会回显；同一时间只有一个供应商生效。
    </p>

    <a-spin :spinning="loading">
      <div v-if="!items.length" class="empty">还没有供应商——点右上角「添加供应商」开始配置。</div>
      <div v-else class="provider-list">
        <div
          v-for="p in items"
          :key="p.id"
          class="provider-card"
          :class="{ enabled: p.is_enabled }"
        >
          <button class="enable-hit" title="设为启用" @click="enable(p)">
            <span class="radio-dot" :class="{ on: p.is_enabled }" />
            <span class="p-name">{{ p.name }}</span>
            <CheckCircleFilled v-if="p.is_enabled" class="on-mark" />
          </button>
          <div class="p-meta">
            <span class="p-key">{{ providerLabel(p.provider_key) }}</span>
            <span class="p-model">{{ p.model || "（未填模型）" }}</span>
            <span v-if="p.base_url" class="p-url" :title="p.base_url">{{ p.base_url }}</span>
            <span class="p-keyflag" :class="{ ok: p.api_key_set }">
              {{ p.api_key_set ? "Key 已设置" : "Key 未设置" }}
            </span>
          </div>
          <div class="p-ops">
            <button class="op" title="连通测试" :disabled="checkingId === p.id" @click="check(p)">
              <ApiOutlined /> {{ checkingId === p.id ? "测试中…" : "测试" }}
            </button>
            <button class="op" title="编辑" @click="openEdit(p)"><EditOutlined /></button>
            <a-popconfirm title="删除该供应商？" @confirm="remove(p)">
              <button class="op danger" title="删除"><DeleteOutlined /></button>
            </a-popconfirm>
          </div>
        </div>
      </div>
    </a-spin>

    <!-- 新增 / 编辑 -->
    <a-modal
      v-model:open="modalOpen"
      :title="editingId === null ? '添加 AI 供应商' : '编辑 AI 供应商'"
      :width="520"
      :footer="null"
      destroy-on-close
    >
      <div class="prov-form">
        <label class="f-row">
          <span class="f-label">名称</span>
          <a-input v-model:value="form.name" placeholder="如：DeepSeek 官方" />
        </label>
        <label class="f-row">
          <span class="f-label">类型</span>
          <a-select v-model:value="form.provider_key" :options="PROVIDER_OPTIONS" />
        </label>
        <label class="f-row">
          <span class="f-label">Base URL</span>
          <a-input
            v-model:value="form.base_url"
            :placeholder="form.provider_key === 'openai-compatible' ? '留空 = https://api.openai.com，兼容 /v1 或完整端点' : '留空 = 官方地址'"
          />
        </label>
        <label class="f-row">
          <span class="f-label">API Key</span>
          <a-input-password
            v-model:value="form.api_key"
            :placeholder="editingId !== null && form.api_key === '' ? '留空 = 保留原值' : 'sk-…'"
            autocomplete="new-password"
          />
        </label>
        <label class="f-row">
          <span class="f-label">模型</span>
          <a-input v-model:value="form.model" placeholder="如：deepseek-chat / gpt-4o-mini / qwen2.5:7b" />
        </label>
        <div class="f-row pair">
          <label class="f-half">
            <span class="f-label">max_tokens</span>
            <a-input-number v-model:value="form.max_tokens" :min="256" :max="200000" :step="256" style="width: 100%" />
          </label>
          <label class="f-half">
            <span class="f-label">temperature</span>
            <a-input-number v-model:value="form.temperature" :min="0" :max="2" :step="0.1" style="width: 100%" />
          </label>
        </div>
        <div class="f-actions">
          <button class="btn-cancel" @click="modalOpen = false">取消</button>
          <button class="btn-save" :disabled="saving" @click="save">
            {{ saving ? "保存中…" : "保存" }}
          </button>
        </div>
      </div>
    </a-modal>
  </section>
</template>

<style scoped>
.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}
.pane-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  margin: 0;
}
.add-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 30px;
  padding: 0 14px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border: none;
  border-radius: 999px;
  cursor: pointer;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--color-primary) 26%, transparent);
  transition: filter var(--transition-fast);
}
.add-btn:hover {
  filter: brightness(1.08);
}
.section-hint {
  font-size: 12px;
  line-height: 1.6;
  color: var(--text-tertiary);
  margin: 0 0 16px;
}
.empty {
  padding: 40px 0;
  text-align: center;
  font-size: 13px;
  color: var(--text-tertiary);
}

.provider-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.provider-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--bg-input);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-md);
  transition: border-color var(--transition-fast);
}
.provider-card.enabled {
  border-color: color-mix(in srgb, var(--color-primary) 40%, var(--border-light));
}
.enable-hit {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 2px 4px;
  background: none;
  border: none;
  cursor: pointer;
  flex-shrink: 0;
}
.radio-dot {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 1.5px solid var(--text-tertiary);
  transition: all var(--transition-fast);
  flex-shrink: 0;
}
.radio-dot.on {
  border-color: var(--color-primary);
  background: radial-gradient(circle, var(--color-primary) 40%, transparent 45%);
}
.p-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
}
.on-mark {
  color: var(--color-primary);
  font-size: 13px;
}
.p-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
  min-width: 0;
  flex-wrap: wrap;
}
.p-key,
.p-model,
.p-url {
  font-size: 11.5px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  border-radius: 999px;
  padding: 1px 9px;
  white-space: nowrap;
}
.p-url {
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  font-family: "JetBrains Mono", monospace;
  font-size: 10.5px;
}
.p-keyflag {
  font-size: 11px;
  color: var(--text-tertiary);
}
.p-keyflag.ok {
  color: #16a34a;
}
.p-ops {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}
.op {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 26px;
  padding: 0 10px;
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

/* 表单 */
.prov-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-top: 4px;
}
.f-row {
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.f-row.pair {
  flex-direction: row;
  gap: 12px;
}
.f-half {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.f-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}
.f-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 4px;
}
.btn-cancel {
  height: 30px;
  padding: 0 16px;
  font-size: 12.5px;
  color: var(--text-secondary);
  background: transparent;
  border: 1px solid var(--border-light);
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-cancel:hover {
  color: var(--text-primary);
  border-color: var(--text-tertiary);
}
.btn-save {
  height: 30px;
  padding: 0 20px;
  font-size: 12.5px;
  font-weight: 600;
  color: var(--hero-cta-text, #fff);
  background: linear-gradient(135deg, var(--color-primary), color-mix(in srgb, var(--color-primary) 82%, #000));
  border: none;
  border-radius: 999px;
  cursor: pointer;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--color-primary) 26%, transparent);
  transition: filter var(--transition-fast);
}
.btn-save:hover {
  filter: brightness(1.08);
}
.btn-save:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
