<script setup lang="ts">
// App shell: Tabs view or Settings view (no router). nav://settings is handled in the store.
import { onMounted } from "vue";
import { useAppState } from "./store";
import TabBar from "./components/TabBar.vue";
import TabList from "./components/TabList.vue";
import EnvList from "./components/EnvList.vue";
import Footer from "./components/Footer.vue";
import IssueBar from "./components/IssueBar.vue";
import SettingsView from "./components/settings/SettingsView.vue";

const { state, start } = useAppState();
onMounted(() => void start());
</script>

<template>
  <div class="app">
    <div v-if="!state.view" class="boot">Loading…</div>
    <SettingsView v-else-if="state.screen === 'settings'" />
    <template v-else>
      <TabBar />
      <IssueBar v-if="state.view.globalError" :issue="state.view.globalError" :warning="state.view.globalError.kind === 'rateLimited'" />
      <EnvList v-if="state.envsOpen" />
      <TabList v-else />
      <Footer />
    </template>
  </div>
</template>

<style scoped>
.app {
  height: 100%;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  background: var(--bg);
}
.boot {
  margin: auto;
  color: var(--text-muted);
}
</style>
