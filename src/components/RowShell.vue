<script setup lang="ts">
// Shared row behaviour: click to open, hover tooltip, NEW / new-comment / changed markers, highlight.
import { computed } from "vue";
import type { ListItem, TabView } from "../types";
import { useAppState } from "../store";
import { changeLabel } from "../util/text";

const props = defineProps<{ item: ListItem; tabView: TabView }>();
const { state, openUrl } = useAppState();

const isNew = computed(() => props.tabView.newItems.includes(props.item.gid));
const newComments = computed(() => props.tabView.newComments[props.item.gid] ?? 0);
const changes = computed(() => props.tabView.changes[props.item.gid] ?? []);
const changeText = computed(() => [...new Set(changes.value.map(changeLabel))].join(", "));
const marked = computed(() => isNew.value || newComments.value > 0 || changes.value.length > 0);
const highlighted = computed(
  () => state.highlight?.gid === props.item.gid && state.highlight?.tabId === props.tabView.tab.id,
);
const tooltip = computed(() =>
  props.item.descriptionPreview ? `${props.item.title}\n\n${props.item.descriptionPreview}` : props.item.title,
);

function open() {
  void openUrl(props.item.webUrl);
}
</script>

<template>
  <div
    class="row"
    :class="{ marked, highlighted }"
    :data-gid="item.gid"
    role="link"
    tabindex="0"
    :title="tooltip"
    :aria-label="`${item.reference} ${item.title}`"
    @click="open"
    @keydown.enter.prevent="open"
  >
    <slot :is-new="isNew" :new-comments="newComments" :change-text="changeText" />
  </div>
</template>

<style scoped>
.row {
  position: relative;
  padding: 8px 12px 8px 14px;
  border-bottom: 1px solid var(--border);
  border-left: 3px solid transparent;
  cursor: pointer;
}
.row:hover {
  background: var(--bg-hover);
}
.row:focus-visible {
  outline: 2px solid var(--blue);
  outline-offset: -2px;
}
.row.marked {
  border-left-color: var(--orange-strong);
}
.row.highlighted {
  animation: flash 2s ease-out;
}
@keyframes flash {
  0%,
  60% {
    background: var(--blue-bg);
  }
  100% {
    background: transparent;
  }
}
</style>
