<script setup lang="ts">
// Envs tab (FR-040): per tracked env, the latest successful deployment and the rows on it.
import { computed } from "vue";
import { useAppState } from "../store";
import { useNow } from "../composables";
import { formatDateTime, relativeTime } from "../util/time";
import type { DeepReadonly } from "vue";
import type { EnvStatus, ListItem } from "../types";

const { state, openUrl, navigateToRow } = useAppState();
const now = useNow();

interface OnEnv {
  item: DeepReadonly<ListItem>;
  tabId: string;
  upToDate: boolean;
}

/** Rows deployed on each env, once per item (first tab that has it), MRs first. */
const onEnv = computed(() => {
  const out = new Map<string, OnEnv[]>();
  const seen = new Set<string>();
  for (const t of state.view?.tabs ?? []) {
    for (const item of t.items) {
      for (const b of item.envs ?? []) {
        const k = `${b.env}|${item.gid}`;
        if (seen.has(k)) continue;
        seen.add(k);
        const list = out.get(b.env) ?? [];
        list.push({ item, tabId: t.tab.id, upToDate: b.upToDate });
        out.set(b.env, list);
      }
    }
  }
  for (const list of out.values()) list.sort((a, b) => (a.item.kind === b.item.kind ? 0 : a.item.kind === "mergeRequests" ? -1 : 1));
  return out;
});

const short = (sha: string) => sha.slice(0, 8);
const open = (url: string | null | undefined) => url && void openUrl(url);
function rowsFor(e: DeepReadonly<EnvStatus>) {
  return onEnv.value.get(e.env.name) ?? [];
}
</script>

<template>
  <section class="wrap">
    <div class="list">
      <article v-for="e in state.view?.envs ?? []" :key="`${e.env.project}#${e.env.name}`" class="env">
        <header class="head">
          <button class="name" :disabled="!e.webUrl" :title="`${e.env.project} — open environment in GitLab`" @click="open(e.webUrl)">{{ e.env.name }}</button>
          <span v-if="e.state && e.state !== 'available'" class="state">{{ e.state }}</span>
          <span class="spacer" />
          <span v-if="e.deployment" class="when" :title="formatDateTime(e.deployment.finishedAt)">{{ relativeTime(e.deployment.finishedAt, now) }}</span>
        </header>
        <p v-if="e.error" class="err">{{ e.error }}</p>
        <div v-if="e.deployment" class="dep">
          <span class="ref" :title="e.deployment.ref">{{ e.deployment.ref }}</span>
          <button class="sha" :disabled="!e.deployment.jobUrl" title="Open the deploy job" @click="open(e.deployment.jobUrl)">{{ short(e.deployment.sha) }}</button>
          <span v-if="e.deployment.user" class="by">by {{ e.deployment.user }}</span>
        </div>
        <p v-else-if="!e.error" class="muted">{{ e.checkedAt ? "No successful deployment yet" : "Loading…" }}</p>
        <ul v-if="rowsFor(e).length" class="items">
          <li v-for="r in rowsFor(e)" :key="r.item.gid">
            <button class="item" :title="`${r.item.reference} ${r.item.title}`" @click="navigateToRow({ tabId: r.tabId, gid: r.item.gid })">
              <span class="refno">{{ r.item.reference }}</span>
              <span class="title">{{ r.item.title }}</span>
              <span v-if="!r.upToDate" class="outdated" title="Newer commits aren't deployed here">outdated</span>
            </button>
          </li>
        </ul>
        <p v-else-if="e.deployment" class="muted small">None of the rows in your tabs are on {{ e.env.name }}</p>
      </article>
      <div v-if="!state.view?.envs.length" class="empty">No environments tracked — add one in Settings</div>
    </div>
  </section>
</template>

<style scoped>
.wrap {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overscroll-behavior: contain;
}
.env {
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
}
.head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.name {
  border: none;
  background: transparent;
  padding: 0;
  font-weight: 700;
  font-size: var(--fs-md);
  color: var(--text);
}
.name:hover:not(:disabled) {
  background: transparent;
  text-decoration: underline;
}
.state {
  font-size: var(--fs-xs);
  padding: 0 6px;
  line-height: 16px;
  border-radius: var(--radius-pill);
  background: var(--bg-strong);
  color: var(--text-muted);
}
.spacer {
  flex: 1;
}
.when,
.by,
.muted {
  color: var(--text-muted);
  font-size: var(--fs-sm);
}
.muted {
  margin: 4px 0 0;
}
.small {
  font-size: var(--fs-xs);
}
.err {
  color: var(--red);
  font-size: var(--fs-sm);
  margin: 4px 0 0;
}
.dep {
  margin-top: 4px;
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
  font-size: var(--fs-sm);
}
.ref {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
}
.sha {
  font-family: ui-monospace, monospace;
  font-size: var(--fs-xs);
  padding: 0 6px;
  line-height: 16px;
}
.items {
  list-style: none;
  margin: 6px 0 0;
  padding: 0;
}
.item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  border: none;
  border-radius: var(--radius, 4px);
  background: transparent;
  padding: 3px 4px;
  text-align: left;
  font-size: var(--fs-sm);
  color: var(--text);
  min-width: 0;
}
.item:hover:not(:disabled) {
  background: var(--bg-hover);
}
.refno {
  color: var(--text-muted);
  white-space: nowrap;
}
.title {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  min-width: 0;
  flex: 1;
}
.outdated {
  font-size: var(--fs-xs);
  font-weight: 600;
  padding: 0 6px;
  line-height: 16px;
  border-radius: var(--radius-pill);
  background: var(--orange-bg);
  color: var(--orange);
}
.empty {
  padding: 40px 16px;
  text-align: center;
  color: var(--text-muted);
}
</style>
