<script setup lang="ts">
// Add, rename, reorder, delete tabs and toggle notify switches (FR-011..FR-014, US6).
import { computed, onBeforeUnmount, onMounted, reactive, ref } from "vue";
import { api } from "../../ipc";
import { useAppState } from "../../store";
import type { ListKind, Tab, UrlCheck } from "../../types";
import Icon from "../Icon.vue";

interface Draft {
  key: number;
  id: string;
  name: string;
  url: string;
  kind: ListKind;
  notifyNewItems: boolean;
  notifyNewComments: boolean;
  notifyChanges: boolean;
  check: UrlCheck | null;
  checking: boolean;
  confirmDelete: boolean;
}

const MAX_NAME = 40;
const { state } = useAppState();
const drafts = reactive<Draft[]>([]);
const saving = ref(false);
const saveError = ref<string | null>(null);
const savedNote = ref(false);
const dirty = ref(false);
let nextKey = 1;
const timers = new Map<number, ReturnType<typeof setTimeout>>();

function toDraft(t: Tab): Draft {
  return { key: nextKey++, ...t, check: null, checking: false, confirmDelete: false };
}

function load(tabs: readonly Tab[]) {
  drafts.splice(0, drafts.length, ...tabs.map((t) => toDraft({ ...t })));
  drafts.forEach((d) => void validate(d));
  dirty.value = false;
}

async function validate(d: Draft) {
  const url = d.url.trim();
  if (!url) {
    d.check = null;
    return;
  }
  d.checking = true;
  try {
    const res = await api.validateUrl(url);
    if (d.url.trim() === url) {
      d.check = res;
      if (res.valid && res.kind) d.kind = res.kind;
    }
  } catch (e) {
    d.check = { valid: false, kind: null, scopeLabel: null, filtersSummary: [], ignoredParams: [], error: (e as { message?: string })?.message ?? String(e) };
  } finally {
    d.checking = false;
  }
}

function onUrlInput(d: Draft) {
  touch();
  d.check = null;
  clearTimeout(timers.get(d.key));
  timers.set(d.key, setTimeout(() => void validate(d), 300));
}

function touch() {
  dirty.value = true;
  savedNote.value = false;
  saveError.value = null;
}

function hint(c: UrlCheck): string {
  if (!c.valid) return `✗ ${c.error ?? "Not a supported GitLab list URL"}`;
  const parts = ["✓ valid", c.kind === "workItems" ? "Work items" : "Merge requests"];
  if (c.scopeLabel) parts.push(c.scopeLabel);
  const n = c.filtersSummary.length;
  let filters = n === 1 ? "1 filter" : `${n} filters`;
  if (c.ignoredParams.length) filters += ` (ignored: ${c.ignoredParams.join(", ")})`;
  parts.push(filters);
  return parts.join(" · ");
}

function add() {
  touch();
  const d = toDraft({ id: "", name: "", url: "", kind: "mergeRequests", notifyNewItems: true, notifyNewComments: true, notifyChanges: true });
  drafts.push(d);
}

function move(i: number, delta: number) {
  const j = i + delta;
  if (j < 0 || j >= drafts.length) return;
  touch();
  const [d] = drafts.splice(i, 1);
  drafts.splice(j, 0, d);
}

function remove(i: number) {
  touch();
  drafts.splice(i, 1);
}

const problems = computed(() => {
  const out: string[] = [];
  drafts.forEach((d, i) => {
    const label = d.name.trim() || `Tab ${i + 1}`;
    if (!d.name.trim()) out.push(`${label}: name is empty`);
    if (!d.url.trim()) out.push(`${label}: URL is empty`);
    else if (d.checking || !d.check) out.push(`${label}: checking URL…`);
    else if (!d.check.valid) out.push(`${label}: URL is not valid`);
  });
  return out;
});

async function save() {
  saving.value = true;
  saveError.value = null;
  try {
    const tabs: Tab[] = drafts.map((d) => ({
      id: d.id,
      name: d.name.trim().slice(0, MAX_NAME),
      url: d.url.trim(),
      kind: d.check?.kind ?? d.kind,
      notifyNewItems: d.notifyNewItems,
      notifyNewComments: d.notifyNewComments,
      notifyChanges: d.notifyChanges,
    }));
    load(await api.saveTabs(tabs));
    savedNote.value = true;
  } catch (e) {
    saveError.value = (e as { message?: string })?.message ?? String(e);
  } finally {
    saving.value = false;
  }
}

const confirmReset = ref(false);
async function reset() {
  confirmReset.value = false;
  saveError.value = null;
  try {
    load(await api.resetDefaultTabs());
    savedNote.value = true;
  } catch (e) {
    saveError.value = (e as { message?: string })?.message ?? String(e);
  }
}

onMounted(() => load((state.view?.tabs ?? []).map((t) => t.tab)));
onBeforeUnmount(() => timers.forEach((t) => clearTimeout(t)));
</script>

<template>
  <section class="card">
    <h2>Tabs</h2>
    <p class="muted small">Filter a list in GitLab, copy the URL from the address bar and paste it here.</p>
    <ol class="tabs">
      <li v-for="(d, i) in drafts" :key="d.key" class="tab">
        <div class="line">
          <label class="sr-only" :for="`name-${d.key}`">Tab name</label>
          <input :id="`name-${d.key}`" v-model="d.name" type="text" class="name" :maxlength="MAX_NAME" placeholder="Tab name" @input="touch" />
          <button class="icon-btn" title="Move up" aria-label="Move up" :disabled="i === 0" @click="move(i, -1)"><Icon name="arrow-up" /></button>
          <button class="icon-btn" title="Move down" aria-label="Move down" :disabled="i === drafts.length - 1" @click="move(i, 1)"><Icon name="arrow-down" /></button>
          <template v-if="d.confirmDelete">
            <button class="danger small-btn" @click="remove(i)">Delete</button>
            <button class="small-btn" @click="d.confirmDelete = false">Keep</button>
          </template>
          <button v-else class="icon-btn" title="Delete tab" aria-label="Delete tab" @click="d.confirmDelete = true"><Icon name="trash" /></button>
        </div>
        <label class="sr-only" :for="`url-${d.key}`">GitLab list URL</label>
        <input :id="`url-${d.key}`" v-model="d.url" type="url" class="url" spellcheck="false" placeholder="https://gitlab.com/…/-/merge_requests?…" @input="onUrlInput(d)" />
        <div class="hint" :class="{ ok: d.check?.valid, bad: d.check && !d.check.valid }" aria-live="polite">
          <template v-if="d.checking">Checking…</template>
          <template v-else-if="d.check">{{ hint(d.check) }}</template>
        </div>
        <div class="toggles">
          <label class="toggle"><input v-model="d.notifyNewItems" type="checkbox" @change="touch" /> New items</label>
          <label class="toggle"><input v-model="d.notifyNewComments" type="checkbox" @change="touch" /> Comments</label>
          <label class="toggle"><input v-model="d.notifyChanges" type="checkbox" @change="touch" /> Changes</label>
        </div>
      </li>
    </ol>
    <div class="actions">
      <button @click="add"><Icon name="plus" :size="12" /> Add tab</button>
      <template v-if="confirmReset">
        <span class="small">Replace all tabs with the 4 defaults?</span>
        <button class="danger" @click="reset">Reset</button>
        <button @click="confirmReset = false">Cancel</button>
      </template>
      <button v-else class="link" @click="confirmReset = true">Reset to defaults</button>
      <span class="spacer" />
      <span v-if="savedNote && !dirty" class="saved">Saved</span>
      <button class="primary" :disabled="saving || !dirty || problems.length > 0" :title="problems.join('\n')" @click="save">
        {{ saving ? "Saving…" : "Save" }}
      </button>
    </div>
    <p v-if="dirty && problems.length" class="muted small">{{ problems[0] }}</p>
    <p v-if="saveError" class="err" role="alert">{{ saveError }}</p>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.small {
  font-size: var(--fs-sm);
}
.tabs {
  list-style: none;
  margin: 0 0 8px;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.tab {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px;
  background: var(--bg-subtle);
  display: flex;
  flex-direction: column;
  gap: 5px;
}
.line {
  display: flex;
  gap: 2px;
  align-items: center;
}
.name {
  flex: 1;
  font-weight: 600;
  margin-right: 4px;
}
.url {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
}
.hint {
  font-size: var(--fs-xs);
  min-height: 15px;
  color: var(--text-muted);
  overflow-wrap: anywhere;
}
.hint.ok {
  color: var(--green);
}
.hint.bad {
  color: var(--red);
}
.toggles {
  display: flex;
  gap: 14px;
  font-size: var(--fs-sm);
}
.small-btn {
  padding: 1px 8px;
  font-size: var(--fs-xs);
}
.actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.actions button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.spacer {
  flex: 1;
}
.saved {
  color: var(--green);
  font-size: var(--fs-sm);
}
</style>
