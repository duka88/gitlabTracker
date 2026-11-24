// Typed wrappers for every command and event in contracts/tauri-ipc.md.
import { invoke } from "@tauri-apps/api/core";
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AppView,
  NotificationEntry,
  OpenTarget,
  RowTarget,
  Tab,
  TrackedEnv,
  UrlCheck,
  UserInfo,
} from "./types";

export const api = {
  getView: () => invoke<AppView>("get_view"),
  testToken: (token: string) => invoke<UserInfo>("test_token", { token }),
  saveToken: (token: string) => invoke<UserInfo>("save_token", { token }),
  removeToken: () => invoke<void>("remove_token"),
  setInterval: (minutes: 1 | 2 | 5 | 10) => invoke<void>("set_interval", { minutes }),
  setNotifyMentions: (enabled: boolean) => invoke<void>("set_notify_mentions", { enabled }),
  setTestBranch: (branch: string | null) => invoke<void>("set_test_branch", { branch }),
  setTrackedEnvs: (envs: TrackedEnv[]) => invoke<TrackedEnv[]>("set_tracked_envs", { envs }),
  validateUrl: (url: string) => invoke<UrlCheck>("validate_url", { url }),
  saveTabs: (tabs: Tab[]) => invoke<Tab[]>("save_tabs", { tabs }),
  resetDefaultTabs: () => invoke<Tab[]>("reset_default_tabs"),
  refreshNow: () => invoke<void>("refresh_now"),
  selectTab: (tabId: string) => invoke<void>("select_tab", { tabId }),
  openBell: () => invoke<NotificationEntry[]>("open_bell"),
  openEntry: (entryId: string) => invoke<OpenTarget>("open_entry", { entryId }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  hideWindow: () => invoke<void>("hide_window"),
  quit: () => invoke<void>("quit"),
};

export const events = {
  onStateUpdated: (cb: (v: AppView) => void): Promise<UnlistenFn> =>
    listen<AppView>("state://updated", (e) => cb(e.payload)),
  onPollStarted: (cb: () => void): Promise<UnlistenFn> => listen("poll://started", () => cb()),
  onNavRow: (cb: (t: RowTarget) => void): Promise<UnlistenFn> =>
    listen<RowTarget>("nav://row", (e) => cb(e.payload)),
  onNavBell: (cb: () => void): Promise<UnlistenFn> => listen("nav://bell", () => cb()),
  onNavSettings: (cb: () => void): Promise<UnlistenFn> => listen("nav://settings", () => cb()),
  /** `ui://visibility`: feeds the visible-tab rule (FR-023a). */
  emitVisibility: (visible: boolean, activeTabId: string | null) =>
    emit("ui://visibility", { visible, activeTabId }),
};
