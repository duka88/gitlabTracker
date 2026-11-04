<script setup lang="ts">
// "Updated 1m ago · every 2 min" + Quit (FR-030 / T055).
import { computed } from "vue";
import { useAppState } from "../store";
import { useNow } from "../composables";
import { formatDateTime, relativeTime } from "../util/time";

const { state, quit } = useAppState();
const now = useNow();
const text = computed(() => {
  const v = state.view;
  if (!v) return "";
  const every = `every ${v.intervalMinutes} min`;
  if (state.refreshing) return `Checking… · ${every}`;
  return v.lastPollAt ? `Updated ${relativeTime(v.lastPollAt, now.value)} · ${every}` : `Not checked yet · ${every}`;
});
</script>

<template>
  <footer class="foot">
    <span class="status" :title="formatDateTime(state.view?.lastPollAt)">{{ text }}</span>
    <span v-if="state.view?.user" class="user">@{{ state.view.user.username }}</span>
    <button class="quit" @click="quit">Quit</button>
  </footer>
</template>

<style scoped>
.foot {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 10px;
  border-top: 1px solid var(--border);
  background: var(--bg-subtle);
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
.status {
  flex: 1;
}
.quit {
  font-size: var(--fs-xs);
  padding: 1px 8px;
}
</style>
