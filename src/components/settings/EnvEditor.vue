<script setup lang="ts">
// Settings → Environments (FR-040): which GitLab deployment environments to track.
import { ref } from "vue";
import { api } from "../../ipc";
import { useAppState } from "../../store";
import Icon from "../Icon.vue";

const DEFAULT_PROJECT = "login5/login-eko/login-eko";

const { state } = useAppState();
interface Draft {
  key: number;
  project: string;
  name: string;
}
let nextKey = 0;
const drafts = ref<Draft[]>((state.view?.trackedEnvs ?? []).map((e) => ({ key: nextKey++, ...e })));
const error = ref<string | null>(null);
const saved = ref(false);

function add() {
  const project = drafts.value[drafts.value.length - 1]?.project ?? DEFAULT_PROJECT;
  drafts.value.push({ key: nextKey++, project, name: "" });
  saved.value = false;
}
function remove(i: number) {
  drafts.value.splice(i, 1);
  saved.value = false;
}
async function save() {
  error.value = null;
  saved.value = false;
  try {
    const out = await api.setTrackedEnvs(drafts.value.map(({ project, name }) => ({ project, name })));
    drafts.value = out.map((e) => ({ key: nextKey++, ...e }));
    saved.value = true;
  } catch (err) {
    error.value = (err as { message?: string })?.message ?? String(err);
  }
}
</script>

<template>
  <section class="card">
    <h2>Environments</h2>
    <p class="muted small">
      Shows what's deployed on each environment (its latest successful GitLab deployment) in the Envs tab, marks MRs and
      tickets that are on it, and notifies on each new deployment.
    </p>
    <form @submit.prevent="save">
      <div v-for="(d, i) in drafts" :key="d.key" class="row env-row">
        <label class="sr-only" :for="`env-project-${d.key}`">Project path</label>
        <input :id="`env-project-${d.key}`" v-model="d.project" class="project" type="text" spellcheck="false" placeholder="group/project" @input="saved = false" />
        <label class="sr-only" :for="`env-name-${d.key}`">Environment name</label>
        <input :id="`env-name-${d.key}`" v-model="d.name" class="name" type="text" spellcheck="false" placeholder="qa-1" @input="saved = false" />
        <button type="button" class="icon-btn" title="Remove" aria-label="Remove environment" @click="remove(i)">
          <Icon name="trash" :size="14" />
        </button>
      </div>
      <div class="row actions">
        <button type="button" @click="add"><Icon name="plus" :size="12" /> Add environment</button>
        <span class="spacer" />
        <button type="submit">Save</button>
      </div>
    </form>
    <p v-if="saved" class="saved small">Saved</p>
    <p v-if="error" class="err" role="alert">{{ error }}</p>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.small {
  font-size: var(--fs-sm);
}
.env-row {
  margin-bottom: 6px;
}
.project {
  flex: 1;
  min-width: 0;
}
.name {
  width: 110px;
}
.actions {
  margin-top: 4px;
}
.spacer {
  flex: 1;
}
.saved {
  color: var(--green);
  margin: 6px 0 0;
}
</style>
