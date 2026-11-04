// useAppState(): one reactive store fed by Rust events (research R13). No Pinia, no router.
import { computed, reactive, readonly } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, events } from "./ipc";
import type { AppView, NotificationEntry, RowTarget, TabView } from "./types";

export type Screen = "tabs" | "settings";

interface State {
  view: AppView | null;
  screen: Screen;
  /** True from `poll://started` until the next `state://updated` with `polling: false`. */
  refreshing: boolean;
  bellOpen: boolean;
  bellEntries: NotificationEntry[];
  /** Row to scroll to and highlight for 2 s. */
  highlight: (RowTarget & { until: number }) | null;
  visible: boolean;
}

const state = reactive<State>({
  view: null,
  screen: "tabs",
  refreshing: false,
  bellOpen: false,
  bellEntries: [],
  highlight: null,
  visible: true,
});

let started = false;

function applyView(v: AppView) {
  const wasSignedIn = state.view?.signedIn ?? true;
  state.view = v;
  if (!wasSignedIn && v.signedIn) state.screen = "tabs";
  state.refreshing = v.polling;
  if (!v.signedIn) state.screen = "settings";
}

function activeTabId(): string | null {
  const v = state.view;
  if (!v || v.tabs.length === 0) return null;
  if (v.activeTabId && v.tabs.some((t) => t.tab.id === v.activeTabId)) return v.activeTabId;
  return v.tabs[0].tab.id;
}

async function selectTab(tabId: string) {
  if (state.view) state.view.activeTabId = tabId;
  await api.selectTab(tabId);
  void events.emitVisibility(state.visible, tabId);
}

async function navigateToRow(target: RowTarget) {
  state.screen = "tabs";
  state.bellOpen = false;
  await selectTab(target.tabId);
  state.highlight = { ...target, until: Date.now() + 2000 };
  setTimeout(() => {
    if (state.highlight && state.highlight.until <= Date.now()) state.highlight = null;
  }, 2100);
}

async function openBell() {
  state.bellEntries = await api.openBell();
  state.bellOpen = true;
}

async function openEntry(entry: NotificationEntry) {
  const target = await api.openEntry(entry.id);
  if (target.type === "row") {
    await navigateToRow({ tabId: target.tabId, gid: target.gid });
  } else if (target.type === "tab") {
    state.bellOpen = false;
    state.screen = "tabs";
    await selectTab(target.tabId);
  } else {
    state.bellOpen = false;
  }
}

async function refresh() {
  state.refreshing = true;
  await api.refreshNow();
}

async function start() {
  if (started) return;
  started = true;
  await events.onStateUpdated(applyView);
  await events.onPollStarted(() => {
    state.refreshing = true;
  });
  await events.onNavRow((t) => void navigateToRow(t));
  await events.onNavBell(() => {
    state.screen = "tabs";
    void openBell();
  });
  await events.onNavSettings(() => {
    state.screen = "settings";
  });
  try {
    await getCurrentWindow().onFocusChanged(({ payload: focused }) => {
      state.visible = focused;
      if (!focused) state.bellOpen = false;
      void events.emitVisibility(focused, activeTabId());
    });
  } catch {
    // Not running inside Tauri (e.g. tests).
  }
  applyView(await api.getView());
}

export function useAppState() {
  const activeTab = computed<TabView | null>(() => {
    const id = activeTabId();
    return state.view?.tabs.find((t) => t.tab.id === id) ?? null;
  });
  return {
    state: readonly(state),
    activeTab,
    activeTabId: computed(activeTabId),
    start,
    selectTab,
    refresh,
    openBell,
    closeBell: () => {
      state.bellOpen = false;
    },
    openEntry,
    navigateToRow,
    openSettings: () => {
      state.screen = "settings";
      state.bellOpen = false;
    },
    closeSettings: () => {
      if (state.view?.signedIn) state.screen = "tabs";
    },
    openUrl: (url: string) => api.openUrl(url),
    hide: () => {
      state.bellOpen = false;
      return api.hideWindow();
    },
    quit: () => api.quit(),
  };
}
