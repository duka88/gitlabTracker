<script setup lang="ts">
import { computed } from "vue";
import type { PipelineStatus } from "../types";
import Icon, { type IconName } from "./Icon.vue";

const props = defineProps<{ status: PipelineStatus }>();
const MAP: Record<PipelineStatus, { icon: IconName; color: string; text: string }> = {
  success: { icon: "pipeline-passed", color: "var(--green)", text: "Pipeline passed" },
  failed: { icon: "pipeline-failed", color: "var(--red)", text: "Pipeline failed" },
  running: { icon: "pipeline-running", color: "var(--blue)", text: "Pipeline running" },
  pending: { icon: "pipeline-pending", color: "var(--orange-strong)", text: "Pipeline pending" },
  created: { icon: "pipeline-pending", color: "var(--text-faint)", text: "Pipeline created" },
  scheduled: { icon: "clock", color: "var(--text-faint)", text: "Pipeline scheduled" },
  canceled: { icon: "pipeline-canceled", color: "var(--text-muted)", text: "Pipeline canceled" },
  skipped: { icon: "pipeline-skipped", color: "var(--text-faint)", text: "Pipeline skipped" },
  manual: { icon: "pipeline-manual", color: "var(--text-muted)", text: "Manual action needed" },
  other: { icon: "pipeline-pending", color: "var(--text-faint)", text: "Pipeline" },
};
const info = computed(() => MAP[props.status] ?? MAP.other);
</script>

<template>
  <span class="pipe" :style="{ color: info.color }" :title="info.text">
    <Icon :name="info.icon" :size="15" />
  </span>
</template>

<style scoped>
.pipe {
  display: inline-flex;
}
</style>
