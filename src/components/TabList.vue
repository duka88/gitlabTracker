<script setup lang="ts">
// Active tab's rows. Plain keyed v-for and a single shared clock keep 500 rows smooth (T098).
import { nextTick, ref, watch } from "vue";
import { useAppState } from "../store";
import { useNow } from "../composables";
import MrRow from "./MrRow.vue";
import WorkItemRow from "./WorkItemRow.vue";
import IssueBar from "./IssueBar.vue";

const { state, activeTab } = useAppState();
const now = useNow();
const listEl = ref<HTMLElement | null>(null);

// Scroll to a highlighted row (nav://row or a bell entry), once its tab is rendered.
watch(
  () => state.highlight,
  async (h) => {
    if (!h) return;
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
      <div ref="listEl" class="list">
        <div v-if="!activeTab.loaded && activeTab.items.length === 0" class="empty">Loading…</div>
        <div v-else-if="activeTab.items.length === 0" class="empty">
          {{ activeTab.error ? "Nothing to show" : "Nothing here — this list is empty in GitLab" }}
        </div>
        <template v-for="item in activeTab.items" :key="item.gid">
          <MrRow v-if="item.kind === 'mergeRequests'" :item="item" :tab-view="activeTab" :now="now" />
          <WorkItemRow v-else :item="item" :tab-view="activeTab" :now="now" />
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
.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
}
.empty {
  padding: 40px 16px;
  text-align: center;
  color: var(--text-muted);
}
</style>
