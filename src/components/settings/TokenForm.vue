<script setup lang="ts">
// Token form (FR-005..FR-007). The token goes to Rust once and never comes back (constitution II).
import { ref } from "vue";
import { api } from "../../ipc";
import { useAppState } from "../../store";
import type { AppError } from "../../types";

const { state } = useAppState();
const token = ref("");
const busy = ref(false);
const error = ref<string | null>(null);
const confirmRemove = ref(false);

function message(e: unknown): string {
  if (e && typeof e === "object" && "message" in e) return String((e as AppError).message);
  return String(e);
}

async function save() {
  if (!token.value.trim()) return;
  busy.value = true;
  error.value = null;
  try {
    await api.saveToken(token.value.trim());
    token.value = "";
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}

async function remove() {
  busy.value = true;
  error.value = null;
  try {
    await api.removeToken();
    confirmRemove.value = false;
  } catch (e) {
    error.value = message(e);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <section class="card">
    <h2>GitLab token</h2>
    <p v-if="state.view?.user" class="signed">
      Signed in as <strong>@{{ state.view.user.username }}</strong>
      <span class="muted">({{ state.view.user.name }})</span>
    </p>
    <p v-else class="muted">
      Paste a Personal Access Token with the <code>read_api</code> scope (gitlab.com → Preferences → Access tokens).
    </p>
    <form class="row" @submit.prevent="save">
      <label class="sr-only" for="token">Personal Access Token</label>
      <input
        id="token"
        v-model="token"
        type="password"
        autocomplete="off"
        spellcheck="false"
        :placeholder="state.view?.user ? 'Paste a new token to replace it' : 'glpat-…'"
        :disabled="busy"
      />
      <button class="primary" type="submit" :disabled="busy || !token.trim()">{{ busy ? "Testing…" : "Test & save" }}</button>
    </form>
    <p v-if="error" class="err" role="alert">{{ error }}</p>
    <div v-if="state.view?.signedIn" class="remove">
      <template v-if="confirmRemove">
        <span>Remove the token from the Keychain?</span>
        <button class="danger" :disabled="busy" @click="remove">Remove</button>
        <button :disabled="busy" @click="confirmRemove = false">Cancel</button>
      </template>
      <button v-else class="link" @click="confirmRemove = true">Remove token</button>
    </div>
  </section>
</template>

<style scoped src="./settings.css"></style>
<style scoped>
.signed {
  margin: 0 0 8px;
}
.remove {
  margin-top: 8px;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: var(--fs-sm);
}
code {
  font-family: var(--font-mono);
  font-size: var(--fs-xs);
  background: var(--bg-strong);
  padding: 0 3px;
  border-radius: 3px;
}
</style>
