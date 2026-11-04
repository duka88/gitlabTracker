<script setup lang="ts">
import { computed } from "vue";
import Icon, { type IconName } from "./Icon.vue";

const props = defineProps<{ typeName: string; iconName?: string | null }>();
const icon = computed<IconName>(() => {
  const key = `${props.iconName ?? ""} ${props.typeName}`.toLowerCase();
  if (key.includes("task")) return "task";
  if (key.includes("bug")) return "bug";
  if (key.includes("epic")) return "epic";
  if (key.includes("incident")) return "incident";
  return "issue";
});
const color = computed(() => {
  switch (icon.value) {
    case "bug":
    case "incident":
      return "var(--red)";
    case "epic":
      return "var(--purple)";
    case "task":
      return "var(--blue)";
    default:
      return "var(--green)";
  }
});
</script>

<template>
  <span class="type" :style="{ color }" :title="typeName"><Icon :name="icon" :size="14" /></span>
</template>

<style scoped>
.type {
  display: inline-flex;
}
</style>
