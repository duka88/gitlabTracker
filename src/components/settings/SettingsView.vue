<script setup lang="ts">
// Settings: token, interval, mentions toggle, tabs (T056, T087, T092).
import { ref } from "vue";
import { api } from "../../ipc";
import { useAppState } from "../../store";
import Icon from "../Icon.vue";
import IssueBar from "../IssueBar.vue";
import TokenForm from "./TokenForm.vue";
import IntervalPicker from "./IntervalPicker.vue";
import TabEditor from "./TabEditor.vue";
import EnvEditor from "./EnvEditor.vue";

const { state, closeSettings, quit, hide } = useAppState();
const mentionError = ref<string | null>(null);

// FR-037: branch flagged on every MR.
const testBranch = ref(state.view?.testBranch ?? "");
const branchError = ref<string | null>(null);
const branchSaved = ref(false);
async function saveTestBranch() {
  branchError.value = null;
  branchSaved.value = false;
  const b = testBranch.value.trim();
  if (/\s/.test(b)) {
    branchError.value = "A branch name can't contain spaces";
    return;
  }
  try {
    await api.setTestBranch(b || null);
    branchSaved.value = true;
  } catch (err) {
    branchError.value = (err as { message?: string })?.message ?? String(err);
  }
}

async function setMentions(e: Event) {
  mentionError.value = null;
  try {
    await api.setNotifyMentions((e.target as HTMLInputElement).checked);
  } catch (err) {
    mentionError.value = (err as { message?: string })?.message ?? String(err);
  }
}
</script>

<template>
  <div class="settings">
    <header class="head" data-tauri-drag-region="deep">
      <button v-if="state.view?.signedIn" class="icon-btn" title="Back to tabs" aria-label="Back to tabs" @click="closeSettings">
        <Icon name="back" :size="15" />
      </button>
      <h1>Settings</h1>
      <span class="spacer" />
      <button class="quit" @click="quit">Quit</button>
      <button class="icon-btn" title="Close window" aria-label="Close window" @click="hide">
        <Icon name="close" :size="15" />
      </button>
    </header>
    <IssueBar v-if="state.view?.globalError && state.view.globalError.kind !== 'noToken'" :issue="state.view.globalError" />
    <div class="body">
      <TokenForm />
      <section class="card">
        <h2>Notifications</h2>
        <IntervalPicker />
        <label class="toggle mention">
          <input type="checkbox" :checked="state.view?.notifyMentions" @change="setMentions" />
          Notify when I'm mentioned
        </label>
        <p v-if="mentionError" class="err" role="alert">{{ mentionError }}</p>
      </section>
      <section class="card">
        <h2>Test branch</h2>
        <p class="muted small">Every MR merged into this branch is flagged "on &lt;branch&gt;" in all MR tabs.</p>
        <form class="row" @submit.prevent="saveTestBranch">
          <label class="sr-only" for="test-branch">Test branch</label>
          <input id="test-branch" v-model="testBranch" type="text" spellcheck="false" placeholder="e.g. resource-test (empty = off)" @input="branchSaved = false" />
          <button type="submit">Save</button>
        </form>
        <p v-if="branchSaved" class="saved small">Saved</p>
        <p v-if="branchError" class="err" role="alert">{{ branchError }}</p>
      </section>
      <EnvEditor />
      <TabEditor />
    </div>
  </div>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.settings {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.head {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border);
  background: var(--bg-subtle);
  min-height: 36px;
}
h1 {
  margin: 0;
  font-size: var(--fs-md);
  font-weight: 600;
}
.spacer {
  flex: 1;
}
.quit {
  font-size: var(--fs-xs);
  padding: 1px 8px;
}
.body {
  flex: 1;
  min-height: 0;
  overscroll-behavior: contain;
  overflow-y: auto;
  user-select: text;
}
.small {
  font-size: var(--fs-sm);
}
.saved {
  color: var(--green);
  margin: 6px 0 0;
}
.mention {
  margin-top: 10px;
}
</style>
