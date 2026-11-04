<script setup lang="ts">
// Polling interval: 1, 2, 5 or 10 minutes (FR-008).
import { ref } from "vue";
import { api } from "../../ipc";
import { useAppState } from "../../store";

const OPTIONS = [1, 2, 5, 10] as const;
const { state } = useAppState();
const error = ref<string | null>(null);

async function pick(m: (typeof OPTIONS)[number]) {
  error.value = null;
  try {
    await api.setInterval(m);
  } catch (e) {
    error.value = (e as { message?: string })?.message ?? String(e);
  }
}
</script>

<template>
  <div class="interval">
    <span id="interval-label">Check every</span>
    <div class="seg" role="radiogroup" aria-labelledby="interval-label">
      <button
        v-for="m in OPTIONS"
        :key="m"
        role="radio"
        :aria-checked="state.view?.intervalMinutes === m"
        :class="{ on: state.view?.intervalMinutes === m }"
        @click="pick(m)"
      >
        {{ m }} min
      </button>
    </div>
    <span v-if="error" class="err">{{ error }}</span>
  </div>
</template>

<style scoped>
.interval {
  display: flex;
  align-items: center;
  gap: 8px;
}
.seg {
  display: inline-flex;
}
.seg button {
  border-radius: 0;
  margin-left: -1px;
  padding: 2px 9px;
}
.seg button:first-child {
  border-radius: var(--radius) 0 0 var(--radius);
}
.seg button:last-child {
  border-radius: 0 var(--radius) var(--radius) 0;
}
.seg button.on {
  background: var(--blue-bg);
  border-color: var(--blue);
  color: var(--blue);
  position: relative;
  z-index: 1;
}
.err {
  color: var(--red);
  font-size: var(--fs-sm);
}
</style>
