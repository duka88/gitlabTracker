<script setup lang="ts">
// Bell history, newest first (FR-024). Opening it marks everything read (done by store.openBell).
import { computed, onBeforeUnmount, onMounted } from "vue";
import { useAppState } from "../store";
import { relativeTime } from "../util/time";
import { useNow } from "../composables";
import type { NotificationEntry } from "../types";

const { state, openEntry, closeBell } = useAppState();
const now = useNow();
const tabNames = computed(() => new Map((state.view?.tabs ?? []).map((t) => [t.tab.id, t.tab.name])));

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") closeBell();
}
onMounted(() => window.addEventListener("keydown", onKey));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey));

function tabName(e: NotificationEntry): string {
  if (e.kind === "mention") return "Mention";
  return (e.tabId && tabNames.value.get(e.tabId)) || "";
}
</script>

<template>
  <div class="backdrop" data-tauri-drag-region="false" @click="closeBell" />
  <div class="popup" role="dialog" aria-label="Notifications" data-tauri-drag-region="false">
    <div class="head">Notifications</div>
    <div v-if="state.bellEntries.length === 0" class="empty">No notifications yet</div>
    <ul v-else class="list">
      <li v-for="e in state.bellEntries" :key="e.id">
        <button class="entry" :class="{ unread: !e.read }" @click="openEntry(e)">
          <span class="text">{{ e.text }}</span>
          <span class="meta">
            <span v-if="tabName(e)">{{ tabName(e) }} · </span>{{ relativeTime(e.createdAt, now) }}
          </span>
        </button>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.backdrop {
  position: fixed;
  inset: 0;
  z-index: 9;
}
.popup {
  position: absolute;
  right: -2px;
  top: calc(100% + 6px);
  z-index: 10;
  width: 340px;
  max-height: 420px;
  display: flex;
  flex-direction: column;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: var(--shadow);
  overflow: hidden;
}
.head {
  padding: 8px 12px;
  font-weight: 600;
  border-bottom: 1px solid var(--border);
}
.empty {
  padding: 24px 12px;
  text-align: center;
  color: var(--text-muted);
}
.list {
  list-style: none;
  margin: 0;
  padding: 0;
  overflow-y: auto;
}
.entry {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  width: 100%;
  text-align: left;
  border: none;
  border-bottom: 1px solid var(--border);
  border-radius: 0;
  padding: 8px 12px;
  background: transparent;
}
.entry.unread {
  background: var(--blue-bg);
}
.entry:hover {
  background: var(--bg-hover);
}
.text {
  overflow-wrap: anywhere;
}
.meta {
  font-size: var(--fs-xs);
  color: var(--text-muted);
}
</style>
