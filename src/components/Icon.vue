<script setup lang="ts">
// Inline SVG icon set (16×16 grid). Strokes use currentColor so callers set the colour.
export type IconName =
  | "pipeline-passed" | "pipeline-failed" | "pipeline-running" | "pipeline-pending"
  | "pipeline-canceled" | "pipeline-skipped" | "pipeline-manual"
  | "issue" | "task" | "bug" | "epic" | "incident"
  | "lock" | "comment" | "thumb-up" | "thumb-down" | "bell" | "gear" | "refresh"
  | "clock" | "milestone" | "iteration" | "weight" | "branch" | "check" | "close"
  | "arrow-up" | "arrow-down" | "trash" | "plus" | "back" | "health" | "child";

withDefaults(defineProps<{ name: IconName; size?: number }>(), { size: 14 });
</script>

<template>
  <svg
    class="icon"
    :width="size"
    :height="size"
    viewBox="0 0 16 16"
    fill="none"
    stroke="currentColor"
    stroke-width="1.5"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <template v-if="name === 'pipeline-passed'">
      <circle cx="8" cy="8" r="6.5" /><path d="M5 8.2l2 2 4-4.4" />
    </template>
    <template v-else-if="name === 'pipeline-failed'">
      <circle cx="8" cy="8" r="6.5" /><path d="M5.8 5.8l4.4 4.4M10.2 5.8l-4.4 4.4" />
    </template>
    <template v-else-if="name === 'pipeline-running'">
      <circle cx="8" cy="8" r="6.5" /><path d="M8 1.5a6.5 6.5 0 0 1 0 13z" fill="currentColor" stroke="none" />
    </template>
    <template v-else-if="name === 'pipeline-pending'">
      <circle cx="8" cy="8" r="6.5" /><path d="M6.5 5.5v5M9.5 5.5v5" />
    </template>
    <template v-else-if="name === 'pipeline-canceled'">
      <circle cx="8" cy="8" r="6.5" /><path d="M4.5 11.5l7-7" />
    </template>
    <template v-else-if="name === 'pipeline-skipped'">
      <circle cx="8" cy="8" r="6.5" /><path d="M5.5 5.5l3 2.5-3 2.5M9 5.5l3 2.5-3 2.5" />
    </template>
    <template v-else-if="name === 'pipeline-manual'">
      <circle cx="8" cy="8" r="6.5" /><path d="M6 10l4-4M8.5 6H10v1.5" />
    </template>
    <template v-else-if="name === 'issue'">
      <circle cx="8" cy="8" r="6.5" /><circle cx="8" cy="8" r="1.5" fill="currentColor" />
    </template>
    <template v-else-if="name === 'task'">
      <rect x="2" y="2" width="12" height="12" rx="2.5" /><path d="M5 8.2l2 2 4-4.4" />
    </template>
    <template v-else-if="name === 'bug'">
      <rect x="5" y="4.5" width="6" height="9" rx="3" /><path d="M8 7v6.5M2.5 9h2.5M11 9h2.5M3 5l2 1.5M13 5l-2 1.5M3 13l2-1.5M13 13l-2-1.5M6 3l1 1.5M10 3L9 4.5" />
    </template>
    <template v-else-if="name === 'epic'">
      <path d="M8 1.5l6.5 4v5L8 14.5 1.5 10.5v-5z" /><path d="M8 5v6M5 8h6" />
    </template>
    <template v-else-if="name === 'incident'">
      <path d="M8 1.8l6.5 11.7h-13z" /><path d="M8 6.5v3M8 11.5v.1" />
    </template>
    <template v-else-if="name === 'lock'">
      <rect x="3" y="7" width="10" height="7" rx="1.5" /><path d="M5 7V5a3 3 0 0 1 6 0v2" />
    </template>
    <template v-else-if="name === 'comment'">
      <path d="M2 3.5A1.5 1.5 0 0 1 3.5 2h9A1.5 1.5 0 0 1 14 3.5v6a1.5 1.5 0 0 1-1.5 1.5H7l-3.5 3v-3h0A1.5 1.5 0 0 1 2 9.5z" />
    </template>
    <template v-else-if="name === 'thumb-up'">
      <path d="M5 7l2.5-5c1 0 1.8.8 1.8 1.8V6h3.4a1.3 1.3 0 0 1 1.3 1.5l-.9 5.4A1.3 1.3 0 0 1 11.8 14H5zM2 7h3v7H2z" />
    </template>
    <template v-else-if="name === 'thumb-down'">
      <path d="M11 9l-2.5 5c-1 0-1.8-.8-1.8-1.8V10H3.3A1.3 1.3 0 0 1 2 8.5l.9-5.4A1.3 1.3 0 0 1 4.2 2H11zM14 9h-3V2h3z" />
    </template>
    <template v-else-if="name === 'bell'">
      <path d="M4 11V7a4 4 0 0 1 8 0v4l1.5 1.5h-11zM6.5 14h3" />
    </template>
    <template v-else-if="name === 'gear'">
      <circle cx="8" cy="8" r="2.2" /><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" />
    </template>
    <template v-else-if="name === 'refresh'">
      <path d="M13.5 8A5.5 5.5 0 1 1 11.9 4.1M13.5 2v3.5H10" />
    </template>
    <template v-else-if="name === 'clock'">
      <circle cx="8" cy="8" r="6.5" /><path d="M8 4.5V8l2.5 1.5" />
    </template>
    <template v-else-if="name === 'milestone'">
      <path d="M8 1.5v13M3 3.5h8l2 2-2 2H3z" />
    </template>
    <template v-else-if="name === 'iteration'">
      <path d="M2.5 8a5.5 5.5 0 0 1 9.4-3.9M13.5 8a5.5 5.5 0 0 1-9.4 3.9M12 1.8v2.5H9.5M4 14.2v-2.5h2.5" />
    </template>
    <template v-else-if="name === 'weight'">
      <path d="M4.5 6h7l2 8h-11zM8 6a1.7 1.7 0 1 0 0-3.4A1.7 1.7 0 0 0 8 6z" />
    </template>
    <template v-else-if="name === 'branch'">
      <circle cx="4.5" cy="3.5" r="1.5" /><circle cx="4.5" cy="12.5" r="1.5" /><circle cx="11.5" cy="5" r="1.5" /><path d="M4.5 5v6M11.5 6.5c0 3-7 2-7 4.5" />
    </template>
    <template v-else-if="name === 'check'">
      <path d="M3 8.5l3.2 3.2L13 4.5" />
    </template>
    <template v-else-if="name === 'close'">
      <path d="M4 4l8 8M12 4l-8 8" />
    </template>
    <template v-else-if="name === 'arrow-up'">
      <path d="M8 13V3M4 7l4-4 4 4" />
    </template>
    <template v-else-if="name === 'arrow-down'">
      <path d="M8 3v10M4 9l4 4 4-4" />
    </template>
    <template v-else-if="name === 'trash'">
      <path d="M2.5 4h11M6 4V2.5h4V4M4 4l.7 9.5h6.6L12 4M6.8 6.5v4.5M9.2 6.5v4.5" />
    </template>
    <template v-else-if="name === 'plus'">
      <path d="M8 3v10M3 8h10" />
    </template>
    <template v-else-if="name === 'back'">
      <path d="M10 3L5 8l5 5" />
    </template>
    <template v-else-if="name === 'health'">
      <path d="M8 13.5S2 10 2 5.8A3.2 3.2 0 0 1 8 4.2a3.2 3.2 0 0 1 6 1.6C14 10 8 13.5 8 13.5z" />
    </template>
    <template v-else-if="name === 'child'">
      <path d="M3 2.5v8h4M3 6.5h4" /><rect x="8" y="4.5" width="5.5" height="4" rx="1" /><rect x="8" y="8.5" width="5.5" height="4" rx="1" />
    </template>
  </svg>
</template>

<style scoped>
.icon {
  flex-shrink: 0;
  vertical-align: -2px;
}
</style>
