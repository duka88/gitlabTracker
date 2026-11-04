<script setup lang="ts">
defineProps<{ name: string; color?: string | null }>();
</script>

<template>
  <span class="status" :title="`Status: ${name}`">
    <span class="dot" :style="{ background: color || 'var(--text-faint)' }" />
    {{ name }}
  </span>
</template>

<style scoped>
.status {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: var(--fs-xs);
  color: var(--text-muted);
  white-space: nowrap;
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
}
</style>
