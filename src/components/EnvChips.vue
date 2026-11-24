<script setup lang="ts">
// "qa-1" chips: tracked envs a row is deployed on (FR-040).
import type { EnvBadge } from "../types";

defineProps<{ envs?: EnvBadge[] }>();
</script>

<template>
  <span
    v-for="e in envs ?? []"
    :key="e.env"
    class="env"
    :class="e.upToDate ? 'ok' : 'warn'"
    :title="e.upToDate ? `Deployed on ${e.env}` : `On ${e.env}, but newer commits aren't deployed there`"
  >{{ e.upToDate ? e.env : `${e.env} (outdated)` }}</span>
</template>

<style scoped>
.env {
  font-size: var(--fs-xs);
  font-weight: 600;
  padding: 0 6px;
  line-height: 16px;
  border-radius: var(--radius-pill);
  white-space: nowrap;
}
.env.ok {
  background: var(--blue-bg, var(--bg-strong));
  color: var(--blue);
  box-shadow: inset 0 0 0 1px var(--blue);
}
.env.warn {
  background: var(--orange-bg);
  color: var(--orange);
}
</style>
