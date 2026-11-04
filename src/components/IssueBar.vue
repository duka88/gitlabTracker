<script setup lang="ts">
// Red bar for errors, orange for warnings (FR-031).
import type { Issue } from "../types";
import { useAppState } from "../store";

withDefaults(defineProps<{ issue: Issue; warning?: boolean }>(), { warning: false });
const { openSettings } = useAppState();
</script>

<template>
  <div class="bar" :class="warning ? 'warn' : 'err'" role="alert">
    <span class="msg">{{ issue.message }}</span>
    <button v-if="issue.kind === 'tokenRejected' || issue.kind === 'noToken'" class="act" @click="openSettings">
      Open Settings
    </button>
  </div>
</template>

<style scoped>
.bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  font-size: var(--fs-sm);
  border-bottom: 1px solid var(--border);
}
.err {
  background: var(--red-bg);
  color: var(--red);
}
.warn {
  background: var(--orange-bg);
  color: var(--orange);
}
.msg {
  flex: 1;
}
.act {
  padding: 1px 8px;
  font-size: var(--fs-xs);
  color: inherit;
  border-color: currentColor;
  background: transparent;
}
</style>
