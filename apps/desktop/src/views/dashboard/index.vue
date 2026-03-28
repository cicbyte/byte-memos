<script setup lang="ts">
/**
 * 仪表盘 /（备忘录数据概览中心）
 *
 * 三层结构：
 * 1. 概览统计卡（4 个核心指标：备忘录总数 / 置顶数 / 标签数 / 分类数）
 * 2. 标签词云（按使用频率缩放字号）
 * 3. 最近活动（最近更新 + 置顶备忘录）
 *
 * 统计来自后端 stats_overview 命令；列表数据用 list_memos 聚合。
 */
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { message } from "ant-design-vue";
import {
  PlusOutlined,
  FileTextOutlined,
  PushpinOutlined,
  ReadOutlined,
  RestOutlined,
  EditOutlined,
  UndoOutlined,
} from "@ant-design/icons-vue";
import EmptyState from "@/components/common/EmptyState.vue";
import { api } from "@/api";
import type { Memo, TagStat } from "@/api/types";
import { useLayoutStore } from "@/stores/layout";
import { useHeroStore, HERO_DEFAULTS, type HeroCtaAction } from "@/stores/hero";

const router = useRouter();
const layout = useLayoutStore();
const hero = useHeroStore();

/** hero 编辑弹窗（表单副本，确认才落入 store） */
const heroEditOpen = ref(false);
const heroForm = ref({ ...HERO_DEFAULTS });

const CTA_META: Record<HeroCtaAction, { label: string; icon: typeof PlusOutlined }> = {
  new: { label: "新建备忘录", icon: PlusOutlined },
  random: { label: "随手看一条", icon: ReadOutlined },
  trash: { label: "打开回收站", icon: RestOutlined },
};
const ctaMeta = computed(() => CTA_META[hero.config.ctaAction]);

/** CTA 动作分发 */
function runCta() {
  if (hero.config.ctaAction === "new") {
    router.push("/memos?new=1");
  } else if (hero.config.ctaAction === "trash") {
    router.push("/trash");
  } else {
    // 随机打开一条（从已加载的最近 500 条中抽，避免额外请求）
    const pool = recent.value.length ? recent.value : allMemos.value;
    if (!pool.length) {
      message.info("还没有备忘录，先新建一条吧");
      router.push("/memos?new=1");
      return;
    }
    const pick = pool[Math.floor(Math.random() * pool.length)];
    router.push(`/memos?id=${pick.short_id}`);
  }
}

function openHeroEdit() {
  heroForm.value = { ...hero.config };
  heroEditOpen.value = true;
}
function saveHeroEdit() {
  hero.update({
    titleMain: heroForm.value.titleMain.trim() || HERO_DEFAULTS.titleMain,
    titleHl: heroForm.value.titleHl,
    subtitle: heroForm.value.subtitle,
    ctaAction: heroForm.value.ctaAction,
  });
  heroEditOpen.value = false;
}
function resetHero() {
  heroForm.value = { ...HERO_DEFAULTS };
}

const memoCount = ref(0);
const pinnedCount = ref(0);
const tagCount = ref(0);
const categoryCount = ref(0);
const tags = ref<TagStat[]>([]);
const recent = ref<Memo[]>([]);
const pinnedList = ref<Memo[]>([]);
const loading = ref(true);
/** 全量池（随机 CTA 兜底用） */
const allMemos = ref<Memo[]>([]);
/** 分类列表（色点映射用） */
const catList = ref<Awaited<ReturnType<typeof api.listCategories>>>([]);

/** 词云展示的标签（最多 50 个，避免拥挤） */
const cloudTags = computed(() => tags.value.slice(0, 50));

/** 分类名 → 主题色（最近更新行的分类徽标用） */
const catColor = computed(() => {
  const m = new Map<string, string>();
  for (const c of catList.value) if (c.color) m.set(c.name, c.color);
  return m;
});

/**
 * 词云样式：按 count 在最大/最小之间的比例缩放字号（12-24px），
 * 透明度也按热度区分（低频淡、高频浓）。
 */
function cloudStyle(count: number) {
  const max = Math.max(1, ...tags.value.map((t) => t.count));
  const ratio = count / max;
  return {
    fontSize: `${(12 + ratio * 12).toFixed(0)}px`,
    opacity: (0.55 + ratio * 0.45).toFixed(2),
    fontWeight: ratio > 0.5 ? 600 : 400,
  };
}

onMounted(async () => {
  try {
    const [stats, ms, ts, cats] = await Promise.all([
      api.statsOverview(),
      api.listMemos({ limit: 500 }),
      api.tagStats(20),
      api.listCategories(),
    ]);
    memoCount.value = stats.memo_count;
    pinnedCount.value = stats.pinned_count;
    tagCount.value = ts.length;
    categoryCount.value = cats.length;
    tags.value = ts;
    catList.value = cats;
    allMemos.value = ms;
    // 最近更新（按 updated_at 降序取前 6）
    recent.value = [...ms]
      .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
      .slice(0, 6);
    // 置顶备忘录（前 5）
    pinnedList.value = ms.filter((m) => m.pinned).slice(0, 5);
    layout.setStatusItems([
      { key: "count", text: `共 ${stats.memo_count} 篇备忘录` },
    ]);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div class="dashboard">
    <!-- 英雄区：渐变底 + 主题色网格/光斑（参考 byte-writer 设计语言）；右上角可个性化 -->
    <div class="hero">
      <button class="hero-edit" title="个性化首页标语" @click="openHeroEdit">
        <EditOutlined />
      </button>
      <div class="hero-bg">
        <div class="hero-grid"></div>
        <div class="hero-glow glow-1"></div>
        <div class="hero-glow glow-2"></div>
      </div>
      <div class="hero-content">
        <span class="hero-badge">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="4" y="3" width="16" height="18" rx="3"/><line x1="8.5" y1="8" x2="15.5" y2="8"/><line x1="8.5" y1="12" x2="15.5" y2="12"/><line x1="8.5" y1="16" x2="12.5" y2="16"/></svg>
          byte-memos
        </span>
        <h1 class="hero-title">{{ hero.config.titleMain }}<span class="hero-hl">{{ hero.config.titleHl }}</span></h1>
        <p class="hero-sub">{{ hero.config.subtitle }}</p>
        <div class="hero-actions">
          <button class="hero-cta" @click="runCta">
            <component :is="ctaMeta.icon" />
            {{ ctaMeta.label }}
          </button>
        </div>
        <div class="hero-stats">
          <div class="hs-item">
            <span class="hs-value">{{ memoCount }}</span>
            <span class="hs-label">备忘录</span>
          </div>
          <span class="hs-divider"></span>
          <div class="hs-item">
            <span class="hs-value">{{ pinnedCount }}</span>
            <span class="hs-label">置顶</span>
          </div>
          <span class="hs-divider"></span>
          <div class="hs-item">
            <span class="hs-value">{{ tagCount }}</span>
            <span class="hs-label">标签</span>
          </div>
          <span class="hs-divider"></span>
          <div class="hs-item">
            <span class="hs-value">{{ categoryCount }}</span>
            <span class="hs-label">分类</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 2. 标签词云 -->
    <div v-reveal="0.28" class="chart-row">
      <div class="panel chart-panel">
        <div class="panel-header">
          <h2 class="panel-title">标签</h2>
          <a-button v-if="tags.length" type="link" size="small" @click="router.push('/tags')">全部 →</a-button>
        </div>
        <div v-if="tags.length" class="word-cloud">
          <span
            v-for="t in cloudTags"
            :key="t.tag"
            class="cloud-word"
            :style="cloudStyle(t.count)"
            :title="`${t.tag} · ${t.count} 项`"
            @click="router.push('/tags')"
          >#{{ t.tag }}</span>
        </div>
        <EmptyState v-else name="tag" text="暂无标签" size="small" inline class="chart-empty" />
      </div>
    </div>

    <!-- 3. 最近活动（最近更新 + 置顶备忘录） -->
    <div v-reveal="0.42" class="chart-row">
      <div class="panel">
        <div class="panel-header">
          <h2 class="panel-title">最近更新</h2>
          <a-button type="link" size="small" @click="router.push('/memos')">全部 →</a-button>
        </div>
        <div v-if="recent.length" class="bm-list">
          <div
            v-for="b in recent"
            :key="b.short_id"
            class="bm-row"
            :title="b.title"
            @click="router.push(`/memos?id=${b.short_id}`)"
          >
            <FileTextOutlined class="bm-icon" />
            <span class="bm-title">{{ b.title || "(无标题)" }}</span>
            <span v-if="b.category" class="bm-meta">
              <span
                v-if="catColor.get(b.category)"
                class="bm-cat-dot"
                :style="{ background: catColor.get(b.category) }"
              ></span>{{ b.category }}
            </span>
          </div>
        </div>
        <EmptyState v-else-if="!loading" name="memo" text="还没有备忘录" size="small" inline class="chart-empty" />
      </div>

      <div class="panel">
        <div class="panel-header">
          <h2 class="panel-title">置顶备忘录</h2>
        </div>
        <div class="view-list">
          <div
            v-for="(b, i) in pinnedList"
            :key="b.short_id"
            class="view-row"
            :title="b.title"
            @click="router.push(`/memos?id=${b.short_id}`)"
          >
            <span class="rank">{{ i + 1 }}</span>
            <span class="view-name">{{ b.title || "(无标题)" }}</span>
            <span class="view-count"><PushpinOutlined /></span>
          </div>
          <EmptyState v-if="!pinnedList.length" name="star" text="暂无置顶" size="small" inline class="chart-empty" />
        </div>
      </div>
    </div>

    <!-- hero 个性化编辑弹窗（纯前端，localStorage 持久） -->
    <a-modal
      v-model:open="heroEditOpen"
      title="个性化首页"
      ok-text="保存"
      cancel-text="取消"
      :width="520"
      @ok="saveHeroEdit"
    >
      <div class="hero-form">
        <div class="hf-row">
          <label class="hf-label">标题主句</label>
          <a-input v-model:value="heroForm.titleMain" placeholder="念头随手记" :maxlength="20" show-count />
        </div>
        <div class="hf-row">
          <label class="hf-label">高亮句</label>
          <a-input v-model:value="heroForm.titleHl" placeholder="，条条可寻回" :maxlength="20" />
        </div>
        <div class="hf-row">
          <label class="hf-label">副文案</label>
          <a-textarea v-model:value="heroForm.subtitle" :rows="2" :maxlength="80" show-count />
        </div>
        <div class="hf-row">
          <label class="hf-label">按钮动作</label>
          <a-radio-group v-model:value="heroForm.ctaAction" class="hf-cta">
            <a-radio value="new">新建备忘录</a-radio>
            <a-radio value="random">随手看一条</a-radio>
            <a-radio value="trash">打开回收站</a-radio>
          </a-radio-group>
        </div>
        <div class="hf-footer">
          <a-button size="small" @click="resetHero">
            <template #icon><UndoOutlined /></template>
            恢复默认
          </a-button>
        </div>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.dashboard {
  display: flex;
  flex-direction: column;
  gap: var(--spacing-md);
  height: 100%;
  padding: var(--spacing-md);
  overflow-y: auto;
}
/* ===== 英雄区（参考 byte-writer：渐变 + 网格 + 光斑；明暗双态变量见 derivation/token） ===== */
.hero {
  position: relative;
  flex-shrink: 0; /* overflow:hidden 会使 flex 子项失去自动最小高度，须防压缩 */
  border-radius: 14px;
  overflow: hidden;
  /* 明暗双态：浅色=纸感浅底+墨字，暗色=深空渐变+亮字 */
  background: linear-gradient(135deg, var(--hero-from, #191a1c) 0%, var(--hero-to, #232527) 100%);
  border: var(--hero-border, none);
  padding: 36px 36px 32px;
}
.hero-bg {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.hero-grid {
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(var(--color-primary-rgb, 27, 28, 31), var(--hero-grid-alpha, 0.06)) 1px, transparent 1px),
    linear-gradient(90deg, rgba(var(--color-primary-rgb, 27, 28, 31), var(--hero-grid-alpha, 0.06)) 1px, transparent 1px);
  background-size: 48px 48px;
  -webkit-mask-image: radial-gradient(ellipse at 70% 20%, #000 30%, transparent 75%);
  mask-image: radial-gradient(ellipse at 70% 20%, #000 30%, transparent 75%);
}
.hero-glow {
  position: absolute;
  border-radius: 50%;
  filter: blur(90px);
}
.glow-1 {
  width: 420px;
  height: 420px;
  background: rgba(var(--color-primary-rgb, 27, 28, 31), var(--hero-glow-alpha, 0.07));
  top: -30%;
  right: -8%;
}
.glow-2 {
  width: 320px;
  height: 320px;
  background: rgba(var(--color-primary-rgb, 27, 28, 31), calc(var(--hero-glow-alpha, 0.07) * 0.6));
  bottom: -40%;
  left: -6%;
}
.hero-content {
  position: relative;
  z-index: 1;
  max-width: 640px;
}
/* 右上角个性化入口：低存在感，悬停提亮 */
.hero-edit {
  position: absolute;
  top: 12px;
  right: 12px;
  z-index: 2;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: none;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--hero-stat-label);
  cursor: pointer;
  transition: all 0.2s ease-out;
}
.hero-edit:hover {
  background: rgba(var(--color-primary-rgb), 0.12);
  color: var(--hero-stat-value);
}
.hero-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 13px;
  border-radius: 20px;
  background: rgba(var(--color-primary-rgb, 27, 28, 31), 0.12);
  border: 1px solid rgba(var(--color-primary-rgb, 27, 28, 31), 0.28);
  color: var(--color-primary);
  font-size: 12.5px;
  font-weight: 500;
  letter-spacing: 0.3px;
  margin-bottom: 18px;
}
.hero-title {
  font-size: 30px;
  font-weight: 700;
  color: var(--hero-title);
  margin: 0 0 10px;
  letter-spacing: 0.5px;
}
.hero-hl {
  color: var(--hero-title-hl, var(--color-primary));
  /* 纸墨浅色下主色=墨黑与标题同色：短横装饰线补主题色锚点 */
  border-bottom: 3px solid rgba(var(--color-primary-rgb, 27, 28, 31), 0.35);
  padding-bottom: 2px;
}
.hero-sub {
  font-size: 14px;
  color: var(--hero-sub);
  margin: 0 0 22px;
  line-height: 1.7;
}
.hero-cta {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 44px;
  padding: 0 24px;
  font-size: 14.5px;
  font-weight: 600;
  border-radius: 10px;
  background: var(--color-primary);
  border: none;
  color: var(--hero-cta-text, #ffffff);
  cursor: pointer;
  transition: all 0.2s ease-out;
  letter-spacing: 0.3px;
}
.hero-cta:hover {
  filter: brightness(1.1);
  transform: translateY(-1px);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.25);
}
.hero-stats {
  display: flex;
  align-items: center;
  gap: 24px;
  margin-top: 26px;
}
.hs-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.hs-value {
  font-size: 22px;
  font-weight: 700;
  color: var(--hero-stat-value);
  font-variant-numeric: tabular-nums;
}
.hs-label {
  font-size: 12px;
  color: var(--hero-stat-label);
}
.hs-divider {
  width: 1px;
  height: 28px;
  background: var(--hero-divider);
}

/* hero 个性化表单 */
.hero-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding-top: 4px;
}
.hf-row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.hf-label {
  font-size: 13px;
  color: var(--text-secondary);
}
.hf-cta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 16px;
}
.hf-footer {
  display: flex;
  justify-content: flex-end;
}

/* ============ 图表 / 列表面板通用 ============ */
.chart-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: var(--spacing-md);
}
.panel {
  padding: 16px;
  background: var(--bg-surface);
  border: 1px solid var(--border-light);
  border-radius: var(--radius-lg);
}
.panel-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 12px;
}
.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.panel-header .panel-title {
  margin-bottom: 0;
}
.chart-empty {
  height: 200px;
}

/* 标签词云 */
.word-cloud {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px 10px;
  min-height: 200px;
  padding: 4px 0;
}
.cloud-word {
  color: var(--color-primary);
  cursor: pointer;
  line-height: 1.6;
  transition: opacity var(--transition-fast), color var(--transition-fast);
  user-select: none;
}
.cloud-word:hover {
  opacity: 1 !important;
  color: var(--color-primary-active);
}

/* 最近更新（备忘录行） */
.bm-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.bm-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 12px;
  transition: background var(--transition-fast);
}
.bm-row:hover {
  background: var(--bg-elevated);
}
.bm-icon {
  color: var(--color-primary);
  flex-shrink: 0;
}
.bm-title {
  flex: 1;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.bm-meta {
  color: var(--text-tertiary);
  font-size: 11px;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.bm-cat-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}

/* 最常查看 */
.view-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.view-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 12px;
  transition: background var(--transition-fast);
}
.view-row:hover {
  background: var(--bg-elevated);
}
.rank {
  color: var(--text-tertiary);
  width: 16px;
  font-family: "JetBrains Mono", monospace;
}
.view-name {
  flex: 1;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.view-count {
  color: var(--text-tertiary);
  font-family: "JetBrains Mono", monospace;
}
</style>
