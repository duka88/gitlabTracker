# Contract: Tauri IPC (Vue ⇄ Rust)

The webview never talks to GitLab and never sees the token (constitution II/III). Types refer to [data-model.md](../data-model.md). All commands are `async` and return `Result<T, AppError>`. `AppError` is serialized as `{ kind: string, message: string }`.

## Commands (`invoke`)

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `get_view` | — | `AppView` | Current snapshot, used on mount. |
| `test_token` | `{ token: string }` | `UserInfo` | Calls `currentUser`. Doesn't save anything. Errors: `TokenRejected`, `Unreachable`. |
| `save_token` | `{ token: string }` | `UserInfo` | Tests the token, then writes it to the Keychain, saves `user` (resetting mention baselines when the username changes), and triggers a poll. |
| `remove_token` | — | `()` | Deletes the Keychain entry and clears `user`. The window shows Settings. |
| `set_interval` | `{ minutes: 1\|2\|5\|10 }` | `()` | Saves the value and restarts the timer. |
| `set_notify_mentions` | `{ enabled: boolean }` | `()` | |
| `validate_url` | `{ url: string }` | `UrlCheck` | Pure parse, no network. `UrlCheck = { valid, kind?, scopeLabel?, filtersSummary?: string[], ignoredParams: string[], error?: string }` (FR-013). |
| `save_tabs` | `{ tabs: Tab[] }` | `Tab[]` | Replaces the whole ordered list (add, rename, reorder, delete, toggles). Ids are added to new tabs. Every URL must be valid. Tabs whose URL changed get their state reset. Triggers a poll. |
| `reset_default_tabs` | — | `Tab[]` | Restores the 4 defaults (R14). |
| `refresh_now` | — | `()` | Starts a poll, or does nothing if one is running (FR-018). |
| `select_tab` | `{ tabId: string }` | `()` | Sets `active_tab_id` and clears that tab's markers (FR-023). |
| `open_bell` | — | `NotificationEntry[]` | Returns entries newest first and marks them all read (FR-024c). |
| `open_entry` | `{ entryId: string }` | `OpenTarget` | `{ type: "row", tabId, gid }` when the item is in a tab, otherwise opens `web_url` in the browser and returns `{ type: "browser" }` (FR-024d). |
| `open_url` | `{ url: string }` | `()` | Opens in the default browser. Only `https://gitlab.com/` URLs are allowed. |
| `quit` | — | `()` | Exits the app. |

## Events (Rust → Vue, `listen`)

| Event | Payload | When |
|---|---|---|
| `state://updated` | `AppView` | After every poll, setting change, marker clear or bell change. |
| `poll://started` | `()` | When a poll starts. Starts the Refresh spinner (FR-030). |
| `nav://row` | `{ tabId, gid }` | A notification click or bell entry. The UI switches tab, scrolls to the row and highlights it for 2 s. |
| `nav://bell` | `()` | Notification click fallback: open the bell popup (FR-024). |
| `nav://settings` | `()` | Opened with no token, or from a "Token rejected" bar (FR-007). |

## Events (Vue → Rust, `emit`)

| Event | Payload | Purpose |
|---|---|---|
| `ui://visibility` | `{ visible: boolean, activeTabId: string \| null }` | Feeds the visible-tab rule (FR-023a). Sent on show/hide and tab change. Rust also tracks show/hide on its own. This keeps the active tab in sync. |

## Custom URI scheme

| Scheme | Request | Response |
|---|---|---|
| `avatar://localhost/<percent-encoded URL>` | `<img src>` in the webview | Image bytes with the original content type, from the cache or fetched by Rust. Only `https://gitlab.com` sources are allowed. Anything else (e.g. Gravatar) returns 404, and the UI shows an initials circle. Same on fetch failure (research R16). |

## Tray (native, no IPC)

| Action | Result |
|---|---|
| Left click | Toggle the window, placed at `TrayBottomCenter`. On show, emit the current `AppView` and apply `select_tab` clearing to the active tab. |
| Right click | Menu: **Open**, **Refresh now**, **Quit**. |
| Title | `menu_count` as text, or empty when it's 0 (FR-004). |
| Window loses focus | Hide. |
