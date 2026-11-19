<script lang="ts">
import { reactive as moduleReactive } from "vue";
import type { TabSort } from "../types";
// Sort picked in the list view, per tab, until restart. Dropped when the tab's default
// (Settings) changes, so the new default takes over.
const chosenSort = moduleReactive(new Map<string, { base: TabSort; sort: TabSort }>());
</script>

<script setup lang="ts">
// Active tab's rows. Plain keyed v-for and a single shared clock keep 500 rows smooth (T098).
import { computed, nextTick, reactive, ref, watch } from "vue";
import { useAppState } from "../store";
import { useNow } from "../composables";
import MrRow from "./MrRow.vue";
import WorkItemRow from "./WorkItemRow.vue";
import IssueBar from "./IssueBar.vue";
import { groupByMonthWeek, monthsBack, periodKeys } from "../util/groups";
import { SORT_OPTIONS, sortItems } from "../util/sort";

const { state, activeTab } = useAppState();
const now = useNow();
const listEl = ref<HTMLElement | null>(null);

// "Group by month and week" tabs (FR-038). Only the current month and week start expanded;
// clicks flip a heading away from that default. Only the last 12 months show until
// "Show more". Both are kept per tab until restart.
const baseSort = computed<TabSort>(() => activeTab.value?.tab.sort ?? "gitlab");
const sort = computed<TabSort>({
  get() {
    const c = chosenSort.get(activeTab.value?.tab.id ?? "");
    return c && c.base === baseSort.value ? c.sort : baseSort.value;
  },
  set(v) {
    if (activeTab.value) chosenSort.set(activeTab.value.tab.id, { base: baseSort.value, sort: v });
  },
});
const items = computed(() => sortItems(activeTab.value?.items ?? [], sort.value));

const MONTH_STEP = 12;
const groups = computed(() => (activeTab.value?.tab.groupByWeek ? groupByMonthWeek(items.value) : null));
const current = computed(() => periodKeys(new Date(now.value)));
const toggled = reactive(new Set<string>());
const monthsShown = reactive(new Map<string, number>());
const tabId = () => activeTab.value?.tab.id ?? "";
const ck = (key: string) => `${tabId()}:${key}`;
const openByDefault = (key: string) => key === current.value.month || key === current.value.week;
const isCollapsed = (key: string) => openByDefault(key) === toggled.has(ck(key));
function setOpen(key: string, open: boolean) {
  if (open === openByDefault(key)) toggled.delete(ck(key));
  else toggled.add(ck(key));
}
const toggle = (key: string) => setOpen(key, isCollapsed(key));

const monthLimit = computed(() => monthsShown.get(tabId()) ?? MONTH_STEP);
const cutoff = computed(() => monthsBack(current.value.month, monthLimit.value));
const visibleGroups = computed(() => groups.value?.filter((m) => m.key >= cutoff.value) ?? []);
const hiddenCount = computed(() =>
  (groups.value ?? []).filter((m) => m.key < cutoff.value).reduce((n, m) => n + m.count, 0),
);
const showMore = () => monthsShown.set(tabId(), monthLimit.value + MONTH_STEP);

/** Shows and expands the month and week holding a row, so a highlighted row can be scrolled to. */
function reveal(gid: string) {
  for (const m of groups.value ?? []) {
    const w = m.weeks.find((w) => w.items.some((i) => i.gid === gid));
    if (!w) continue;
    while (m.key < cutoff.value) showMore();
    setOpen(m.key, true);
    setOpen(w.key, true);
    return;
  }
}

// Scroll to a highlighted row (nav://row or a bell entry), once its tab is rendered.
watch(
  () => state.highlight,
  async (h) => {
    if (!h) return;
    await nextTick();
    reveal(h.gid);
    await nextTick();
    const el = listEl.value?.querySelector<HTMLElement>(`[data-gid="${CSS.escape(h.gid)}"]`);
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
  },
);
// New tab → start at the top.
watch(
  () => activeTab.value?.tab.id,
  () => listEl.value?.scrollTo({ top: 0 }),
);
</script>

<template>
  <section class="wrap">
    <template v-if="activeTab">
      <IssueBar v-if="activeTab.error" :issue="activeTab.error" />
      <IssueBar v-if="activeTab.warning" :issue="activeTab.warning" warning />
      <div v-if="activeTab.items.length > 0" class="toolbar">
        <label for="tab-sort">Sort</label>
        <select id="tab-sort" v-model="sort">
          <option v-for="o in SORT_OPTIONS" :key="o.value" :value="o.value">{{ o.label }}</option>
        </select>
      </div>
      <div ref="listEl" class="list">
        <div v-if="!activeTab.loaded && activeTab.items.length === 0" class="empty">Loading…</div>
        <div v-else-if="activeTab.items.length === 0" class="empty">
          {{ activeTab.error ? "Nothing to show" : "Nothing here — this list is empty in GitLab" }}
        </div>
        <template v-else-if="groups">
          <section v-for="m in visibleGroups" :key="m.key" class="month">
            <button class="group-head month-head" :aria-expanded="!isCollapsed(m.key)" @click="toggle(m.key)">
              <span class="caret" :class="{ shut: isCollapsed(m.key) }" aria-hidden="true">▾</span>
              <span class="group-label">{{ m.label }}</span>
              <span class="count">{{ m.count }}</span>
            </button>
            <template v-if="!isCollapsed(m.key)">
              <div v-for="w in m.weeks" :key="w.key" class="week">
                <button class="group-head week-head" :aria-expanded="!isCollapsed(w.key)" @click="toggle(w.key)">
                  <span class="caret" :class="{ shut: isCollapsed(w.key) }" aria-hidden="true">▾</span>
                  <span class="group-label">{{ w.label }}</span>
                  <span class="count">{{ w.items.length }}</span>
                </button>
                <template v-if="!isCollapsed(w.key)">
                  <WorkItemRow v-for="item in w.items" :key="item.gid" :item="item" :tab-view="activeTab" :now="now" />
                </template>
              </div>
            </template>
          </section>
          <button v-if="hiddenCount > 0" class="more" @click="showMore">
            Show more · {{ hiddenCount }} older {{ hiddenCount === 1 ? "ticket" : "tickets" }}
          </button>
        </template>
        <template v-else>
          <template v-for="item in items" :key="item.gid">
            <MrRow v-if="item.kind === 'mergeRequests'" :item="item" :tab-view="activeTab" :now="now" />
            <WorkItemRow v-else :item="item" :tab-view="activeTab" :now="now" />
          </template>
        </template>
      </div>
    </template>
    <div v-else class="empty">No tabs yet — add one in Settings</div>
  </section>
</template>

<style scoped>
.wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 6px;
  padding: 4px 12px;
  border-bottom: 1px solid var(--border);
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.toolbar select {
  font: inherit;
  font-size: var(--fs-sm);
}
.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
}
.group-head {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  border: 0;
  border-bottom: 1px solid var(--border);
  border-radius: 0;
  background: var(--bg-subtle);
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
}
.group-head:hover {
  background: var(--bg-hover);
}
.month-head {
  position: sticky;
  top: 0;
  z-index: 1;
  padding: 6px 12px;
  font-weight: 600;
  background: var(--bg-strong);
}
.week-head {
  padding: 4px 12px 4px 26px;
  font-size: var(--fs-sm);
  color: var(--text-muted);
}
.group-label {
  flex: 1;
}
.count {
  min-width: 20px;
  padding: 0 7px;
  border-radius: var(--radius-pill);
  background: var(--bg);
  border: 1px solid var(--border);
  font-size: var(--fs-xs);
  font-weight: 600;
  color: var(--text);
  text-align: center;
}
.caret {
  display: inline-block;
  width: 10px;
  font-size: var(--fs-xs);
  color: var(--text-faint);
  transition: transform 0.1s;
}
.caret.shut {
  transform: rotate(-90deg);
}
.more {
  display: block;
  width: calc(100% - 24px);
  margin: 10px 12px;
  padding: 6px;
  font-size: var(--fs-sm);
}
.empty {
  padding: 40px 16px;
  text-align: center;
  color: var(--text-muted);
}
</style>
