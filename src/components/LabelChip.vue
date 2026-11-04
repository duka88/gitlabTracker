<script setup lang="ts">
// GitLab label chip. Scoped labels (scope::value) are two-part: filled scope, outlined value (FR-027).
import { computed } from "vue";
import type { Label } from "../types";
import { labelTextColor } from "../util/labels";

const props = defineProps<{ label: Label }>();
const fg = computed(() => labelTextColor(props.label.color, props.label.textColor));
</script>

<template>
  <span
    v-if="label.scope"
    class="chip scoped"
    :style="{ borderColor: label.color }"
    :title="label.title"
  >
    <span class="part scope" :style="{ background: label.color, color: fg }">{{ label.scope }}</span>
    <span class="part value">{{ label.value }}</span>
  </span>
  <span v-else class="chip" :style="{ background: label.color, color: fg, borderColor: label.color }" :title="label.title">
    <span class="part">{{ label.title }}</span>
  </span>
</template>

<style scoped>
.chip {
  display: inline-flex;
  max-width: 100%;
  border: 1px solid;
  border-radius: var(--radius-pill);
  font-size: var(--fs-xs);
  line-height: 16px;
  overflow: hidden;
  white-space: nowrap;
}
.part {
  padding: 0 7px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.scoped .scope {
  padding-right: 5px;
}
.scoped .value {
  padding-left: 5px;
  background: var(--bg);
  color: var(--text);
}
</style>
