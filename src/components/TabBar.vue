<script setup lang="ts">
// Tabs with item count and orange new badge, Refresh (spins while polling), Settings, bell (FR-030).
import { useAppState } from "../store";
import Icon from "./Icon.vue";
import BellPopup from "./BellPopup.vue";

const { state, activeTabId, selectTab, refresh, openSettings, openBell, closeBell, hide } = useAppState();

function toggleBell() {
  if (state.bellOpen) closeBell();
  else void openBell();
}
</script>

<template>
  <header class="bar" data-tauri-drag-region="deep">
    <nav class="tabs" role="tablist">
      <button
        v-for="t in state.view?.tabs ?? []"
        :key="t.tab.id"
        class="tab"
        role="tab"
        :aria-selected="t.tab.id === activeTabId"
        :class="{ active: t.tab.id === activeTabId, errored: !!t.error }"
        :title="t.tab.name"
        @click="selectTab(t.tab.id)"
      >
        <span class="name">{{ t.tab.name }}</span>
        <span class="cnt">{{ t.loaded ? t.itemCount : "–" }}</span>
        <span v-if="t.newCount > 0" class="badge" :aria-label="`${t.newCount} new`">{{ t.newCount }}</span>
      </button>
    </nav>
    <div class="actions">
      <button class="icon-btn" title="Refresh now" aria-label="Refresh now" :disabled="state.refreshing" @click="refresh">
        <Icon name="refresh" :size="15" :class="{ spin: state.refreshing }" />
      </button>
      <button class="icon-btn" title="Settings" aria-label="Settings" @click="openSettings">
        <Icon name="gear" :size="15" />
      </button>
      <div class="bell-wrap">
        <button
          class="icon-btn bell"
          :title="state.view?.bellUnread ? `${state.view.bellUnread} unread notifications` : 'Notifications'"
          aria-label="Notifications"
          :aria-expanded="state.bellOpen"
          @click="toggleBell"
        >
          <Icon name="bell" :size="15" />
          <span v-if="state.view?.bellUnread" class="unread">{{ state.view.bellUnread > 99 ? "99+" : state.view.bellUnread }}</span>
        </button>
        <BellPopup v-if="state.bellOpen" />
      </div>
      <button class="icon-btn" title="Close window" aria-label="Close window" @click="hide">
        <Icon name="close" :size="15" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.bar {
  display: flex;
  align-items: stretch;
  border-bottom: 1px solid var(--border);
  background: var(--bg-subtle);
  min-height: 36px;
}
.tabs {
  display: flex;
  flex: 1;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none;
}
.tabs::-webkit-scrollbar {
  display: none;
}
.tab {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: none;
  border-bottom: 2px solid transparent;
  border-radius: 0;
  background: transparent;
  padding: 6px 10px 4px;
  white-space: nowrap;
  color: var(--text-muted);
  max-width: 160px;
}
.tab:hover:not(:disabled) {
  background: var(--bg-hover);
}
.tab.active {
  color: var(--text);
  font-weight: 600;
  border-bottom-color: var(--blue);
}
.tab.errored .name {
  text-decoration: underline wavy var(--red);
  text-underline-offset: 3px;
}
.name {
  overflow: hidden;
  text-overflow: ellipsis;
}
.cnt {
  font-size: var(--fs-xs);
  font-weight: 400;
  background: var(--bg-strong);
  border-radius: var(--radius-pill);
  padding: 0 6px;
  line-height: 16px;
}
.badge {
  font-size: var(--fs-xs);
  font-weight: 700;
  background: var(--orange-strong);
  color: var(--on-orange);
  border-radius: var(--radius-pill);
  padding: 0 6px;
  line-height: 16px;
}
.actions {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 0 6px;
  border-left: 1px solid var(--border);
}
.bell-wrap {
  position: relative;
}
.bell {
  position: relative;
}
.unread {
  position: absolute;
  top: -2px;
  right: -4px;
  min-width: 15px;
  height: 15px;
  padding: 0 3px;
  border-radius: var(--radius-pill);
  background: var(--red);
  color: var(--on-red);
  font-size: 9px;
  font-weight: 700;
  line-height: 15px;
  text-align: center;
}
</style>
