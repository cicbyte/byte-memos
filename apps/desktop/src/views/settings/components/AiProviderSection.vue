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
  SyncOutlined,
  CloseOutlined,
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

// 模型集合（Cherry Studio 式：获取/手动添加维护，点 chip 设默认）
const models = ref<string[]>([]);
const fetching = ref(false);
const fetchOpen = ref(false);
const fetchedModels = ref<string[]>([]);
const fetchSearch = ref("");
const selectedFetch = ref<string[]>([]);
const addModelsOpen = ref(false);
const addModelsText = ref("");

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
  models.value = [...(p.models ?? [])];
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

// ===== 添加（自定义） / 删除 / 启用 / 测试 =====

/** 添加自定义供应商：内置常用商已常驻列表（后端启动种子），此处只加自定义 */
async function addProvider() {
  try {
    const id = await api.add({
      name: "自定义供应商",
      provider_key: "openai-compatible",
      base_url: "",
      api_key: "",
      model: "",
      max_tokens: 4096,
      temperature: 0.7,
    });
    checkResult.value = null;
    await load(id);
    message.success("已添加自定义供应商，请在右侧填写配置");
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

// ===== 模型集合管理（参考 Cherry Studio「获取模型列表」/ byte-voice ModelSettings） =====

/** 全量覆写模型集合 */
async function saveModels(list: string[]) {
  const p = current.value;
  if (!p) return;
  const clean = [...new Set(list.map((m) => m.trim()).filter(Boolean))];
  try {
    await api.update({
      id: p.id,
      name: form.name.trim(),
      base_url: form.base_url.trim(),
      api_key: "",
      model: form.model.trim(),
      models: clean,
      max_tokens: form.max_tokens,
      temperature: form.temperature,
      is_enabled: p.is_enabled,
    });
    models.value = clean;
    p.models = clean;
  } catch (e) {
    message.error(`保存模型列表失败：${e}`);
  }
}

function removeModel(name: string) {
  const rest = models.value.filter((m) => m !== name);
  if (form.model === name) {
    form.model = rest[0] ?? "";
  }
  void saveModels(rest);
}

/** 拉取远端 /models：先静默保存，保证用最新 Key 请求（byte-voice 同款） */
async function openFetch() {
  const p = current.value;
  if (!p) return;
  await saveNow();
  fetching.value = true;
  fetchOpen.value = true;
  fetchedModels.value = [];
  selectedFetch.value = [];
  fetchSearch.value = "";
  try {
    fetchedModels.value = await api.models(p.id);
    selectedFetch.value = models.value.filter((m) => fetchedModels.value.includes(m));
  } catch (e) {
    message.error(`获取模型失败：${e}`);
  } finally {
    fetching.value = false;
  }
}

function toggleFetchModel(m: string) {
  const i = selectedFetch.value.indexOf(m);
  if (i >= 0) selectedFetch.value.splice(i, 1);
  else selectedFetch.value.push(m);
}

function toggleGroupAll(groupList: string[]) {
  const all = groupList.every((m) => selectedFetch.value.includes(m));
  if (all) {
    selectedFetch.value = selectedFetch.value.filter((m) => !groupList.includes(m));
  } else {
    for (const m of groupList) if (!selectedFetch.value.includes(m)) selectedFetch.value.push(m);
  }
}

function confirmFetch() {
  const list = [...selectedFetch.value].sort();
  if (list.length && !list.includes(form.model)) {
    form.model = list[0];
  }
  void saveModels(list);
  fetchOpen.value = false;
  message.success(`已选择 ${list.length} 个模型`);
}

function confirmAddModels() {
  const names = addModelsText.value.split(/[,，\n]/).map((s) => s.trim()).filter(Boolean);
  const merged = [...models.value];
  let added = 0;
  for (const n of names) {
    if (!merged.includes(n)) {
      merged.push(n);
      added++;
    }
  }
  addModelsOpen.value = false;
  if (added === 0) {
    message.warning("没有新模型可添加");
    return;
  }
  void saveModels(merged);
  message.success(`已添加 ${added} 个模型`);
}

/** 模型分组（按 id 前缀，移植 byte-voice） */
function modelGroup(modelId: string): string {
  const parts = modelId.split(/[-_./]/);
  if (parts.length <= 1) return "其他";
  const prefix = (parts[0] ?? "").toLowerCase();
  const map: Record<string, string> = {
    gpt: "GPT", o1: "OpenAI o-series", o3: "OpenAI o-series", o4: "OpenAI o-series",
    claude: "Claude", deepseek: "DeepSeek", qwen: "Qwen", glm: "GLM",
    llama: "LLaMA", gemma: "Gemma", mistral: "Mistral", mixtral: "Mixtral",
    yi: "Yi", moonshot: "Moonshot", kimi: "Kimi", mimo: "MiMo", longcat: "LongCat",
    gemini: "Gemini", command: "Command", ernie: "文心", hunyuan: "混元",
  };
  for (const [k, v] of Object.entries(map)) {
    if (prefix.includes(k)) return v;
  }
  return prefix.charAt(0).toUpperCase() + prefix.slice(1);
}

function groupOf(list: string[]): Record<string, string[]> {
  const groups: Record<string, string[]> = {};
  for (const m of list) {
    const g = modelGroup(m);
    (groups[g] ??= []).push(m);
  }
  return groups;
}

const fetchedFiltered = computed(() => {
  const kw = fetchSearch.value.toLowerCase();
  return kw ? fetchedModels.value.filter((m) => m.toLowerCase().includes(kw)) : fetchedModels.value;
});

onMounted(() => load());
</script>

<template>
  <section class="pane pane-ai">
    <div class="section-head">
      <h2 class="pane-title">AI 供应商</h2>
      <span class="section-hint">
        常用服务商已内置（GLM / DeepSeek / 小米 MiMo 等），选中填 Key 即用；API Key 加密存放、不会回显，同一时间只有一个供应商生效。
      </span>
    </div>

    <div class="ai-columns">
      <!-- 左：供应商列表 -->
      <aside class="provider-panel">
        <div class="panel-header">
          <span class="panel-title">供应商</span>
          <span class="provider-count">{{ items.length }}</span>
          <span class="flex1" />
          <button class="add-btn" title="添加自定义供应商" :disabled="loading" @click="addProvider">
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
          <div class="setting-title-row">
            <span>模型</span>
            <span class="flex1" />
            <button class="op tight" :disabled="fetching" @click="openFetch">
              <LoadingOutlined v-if="fetching" /><SyncOutlined v-else />
              {{ fetching ? "获取中…" : "获取模型" }}
            </button>
            <button class="op tight" @click="addModelsOpen = true"><PlusOutlined /> 添加</button>
          </div>
          <a-auto-complete
            v-model:value="form.model"
            :options="models.map((m) => ({ value: m }))"
            :filter-option="(input: string, opt: any) => String(opt.value).toLowerCase().includes(input.toLowerCase())"
            placeholder="默认模型，如 deepseek-chat"
            size="small"
            allow-clear
          />
          <div v-if="models.length" class="model-chips">
            <span
              v-for="m in models"
              :key="m"
              class="model-chip"
              :class="{ on: m === form.model }"
              :title="m === form.model ? '默认模型' : '点击设为默认'"
              @click="form.model = m"
            >
              <span class="chip-name">{{ m }}</span>
              <CheckCircleFilled v-if="m === form.model" class="chip-on" />
              <CloseOutlined class="chip-x" @click.stop="removeModel(m)" />
            </span>
          </div>
          <div v-else class="model-empty">还没有模型——点「获取模型」拉取，或手动添加</div>
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

      <!-- 获取模型（远端 /models → 搜索 + 分组多选） -->
      <a-modal
        v-model:open="fetchOpen"
        title="获取模型列表"
        :width="560"
        ok-text="确认选择"
        cancel-text="取消"
        @ok="confirmFetch"
      >
        <a-spin :spinning="fetching">
          <div class="fm-search">
            <a-input v-model:value="fetchSearch" placeholder="搜索模型…" size="small" allow-clear />
          </div>
          <div class="fm-hint">已选 {{ selectedFetch.length }} / {{ fetchedModels.length }} 个模型</div>
          <div v-if="Object.keys(groupOf(fetchedFiltered)).length" class="fm-list">
            <div v-for="(list, g) in groupOf(fetchedFiltered)" :key="g" class="fm-group">
              <label class="fm-group-head">
                <a-checkbox
                  :checked="list.every((m) => selectedFetch.includes(m))"
                  :indeterminate="list.some((m) => selectedFetch.includes(m)) && !list.every((m) => selectedFetch.includes(m))"
                  @change.prevent="toggleGroupAll(list)"
                />
                <span class="fm-group-name">{{ g }}</span>
                <span class="fm-count">{{ list.length }}</span>
              </label>
              <label v-for="m in list" :key="m" class="fm-item">
                <a-checkbox :checked="selectedFetch.includes(m)" @change="toggleFetchModel(m)" />
                <span class="fm-name">{{ m }}</span>
              </label>
            </div>
          </div>
          <div v-else-if="!fetching" class="model-empty" style="margin-top: 8px">
            未获取到模型——请检查 API Key 与端点，或手动添加
          </div>
        </a-spin>
      </a-modal>

      <!-- 手动添加模型（逗号/换行批量） -->
      <a-modal
        v-model:open="addModelsOpen"
        title="添加模型"
        ok-text="添加"
        cancel-text="取消"
        @ok="confirmAddModels"
      >
        <div class="fm-hint" style="margin-bottom: 8px">支持逗号或换行分隔批量添加</div>
        <a-textarea
          v-model:value="addModelsText"
          placeholder="deepseek-chat, deepseek-reasoner"
          :auto-size="{ minRows: 2, maxRows: 5 }"
        />
      </a-modal>

      <div v-if="!current" class="empty-state">
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
.op.tight {
  height: 22px;
  padding: 0 8px;
  font-size: 11px;
  gap: 3px;
}

/* 模型 chips */
.model-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}
.model-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 8px;
  font-size: 11px;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-primary) 5%, transparent);
  border: 1px solid var(--border-light);
  border-radius: 999px;
  cursor: pointer;
  transition: all var(--transition-fast);
  max-width: 100%;
}
.model-chip:hover {
  border-color: var(--text-tertiary);
}
.model-chip.on {
  color: var(--color-primary);
  border-color: color-mix(in srgb, var(--color-primary) 45%, var(--border-light));
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
}
.chip-name {
  font-family: "JetBrains Mono", Consolas, monospace;
  font-size: 10.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.chip-on {
  font-size: 10px;
  flex-shrink: 0;
}
.chip-x {
  font-size: 9px;
  opacity: 0.55;
  flex-shrink: 0;
}
.chip-x:hover {
  opacity: 1;
  color: #ef4444;
}
.model-empty {
  margin-top: 8px;
  padding: 14px;
  text-align: center;
  font-size: 11.5px;
  color: var(--text-tertiary);
  border: 1px dashed var(--border-light);
  border-radius: var(--radius-sm);
}

/* 获取模型弹框 */
.fm-search {
  margin-bottom: 8px;
}
.fm-hint {
  font-size: 11.5px;
  color: var(--text-tertiary);
  margin-bottom: 8px;
}
.fm-list {
  max-height: 46vh;
  overflow-y: auto;
}
.fm-group {
  margin-bottom: 6px;
}
.fm-group-head {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 4px;
  cursor: pointer;
  user-select: none;
  border-bottom: 1px solid var(--border-light);
}
.fm-group-name {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
}
.fm-count {
  font-size: 10px;
  color: var(--text-tertiary);
  background: color-mix(in srgb, var(--text-primary) 6%, transparent);
  border-radius: 999px;
  padding: 0 7px;
  line-height: 15px;
}
.fm-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 4px 4px 14px;
  cursor: pointer;
  border-radius: var(--radius-sm);
}
.fm-item:hover {
  background: color-mix(in srgb, var(--text-primary) 4%, transparent);
}
.fm-name {
  font-size: 12px;
  font-family: "JetBrains Mono", Consolas, monospace;
  color: var(--text-primary);
  word-break: break-all;
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
