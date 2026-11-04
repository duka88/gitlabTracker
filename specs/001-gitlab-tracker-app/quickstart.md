# Quickstart & Validation: GitLab Tracker

**Feature**: [spec.md](./spec.md) · Contracts: [tauri-ipc](./contracts/tauri-ipc.md), [url-mapping](./contracts/url-mapping.md), [gitlab-graphql](./contracts/gitlab-graphql.md)

## Prerequisites

1. Internet access to gitlab.com, crates.io and the npm registry.
2. Xcode Command Line Tools: `xcode-select --install`
3. Rust stable: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`, then `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
4. Node 20+ and pnpm (already installed: Node 24, pnpm).
5. A GitLab Personal Access Token with `read_api` (gitlab.com → Preferences → Access tokens).
6. **Before coding**: open `https://gitlab.com/-/graphql-explorer` and confirm the **VERIFY** items in [research.md](./research.md): R2 (`group.workItems`), R3 (status filter shape), R4 (`not` field names), R5 (work-item comment count), R7 (`todos` action enum, `note` field) and R14 (the exact "Ready to Merge" label name). Save one response per query to `src-tauri/tests/fixtures/`.

## Run

```bash
pnpm install
pnpm tauri dev          # UI + Rust with hot reload (notifications may not show)
```

## Automated checks

```bash
cd src-tauri && cargo test && cargo clippy -- -D warnings
pnpm vue-tsc --noEmit && pnpm vitest run
```

Expected: all green. `cargo test` covers URL mapping (every row in url-mapping "Test examples"), the diff engine (baseline, new, comments, changes, visible-tab rule, cross-tab de-duplication), merge-status derivation, label scope splitting and notification text.

## Build

```bash
pnpm tauri build --bundles app,dmg --target universal-apple-darwin
open src-tauri/target/universal-apple-darwin/release/bundle/dmg/
```

On first launch, right-click → Open (the app is ad-hoc signed). Allow notifications in System Settings → Notifications → GitLab Tracker.

## Manual validation (maps to spec "Done when" / success criteria)

| # | Scenario | Steps | Expected |
|---|---|---|---|
| V1 | First run | Launch with no token | No Dock icon. The window opens in Settings. |
| V2 | Token | Paste a bad token, then a good one | Bad: red error, not saved. Good: "signed in as @DusanJ88", saved to the Keychain (check in Keychain Access). |
| V3 | Window | Click the tray icon, click another app, then click the tray icon twice | Opens under the icon in < 0.5 s; stays open behind the other app; the first icon click brings it forward, the second hides it. |
| V4 | Tray menu | Right-click the icon | Open / Refresh now / Quit all work. |
| V5 | Default tabs | Open each of the 4 tabs | Same items and order as the matching GitLab page (SC-002). MR rows show pipeline, approvals, reviewers ✓, labels (scoped as two-part), merge tag. Ticket rows show type, status dot, iteration, due date, weight, health, child progress (SC-008). |
| V6 | Hover / click | Hover a row, then click it | Full title and description preview; then it opens in the browser. |
| V7 | New item | Window hidden; create an MR (or assign yourself a To do ticket) | Within 2 min: one macOS notification, a bell entry, tab badge +1, menu bar +1 (SC-003). Open the tab → badge and menu count drop (SC-006). |
| V8 | Visible-tab rule | Keep "My MRs" open; create an MR | NEW marker on the row, but no notification, no bell entry, menu count unchanged. Hide the window: the menu count is still unchanged. |
| V9 | Comments | A colleague comments on your MR and on one of your tickets; then you comment | Two notifications ("💬 Name on !123", "💬 Name on #45"), and none for your own comment. The row shows "💬 N (+1)" in orange (SC-004). |
| V10 | Mention | A colleague @mentions you on an item in no tab | "📣 Name mentioned you on #78". The bell entry opens the browser (SC-004a). |
| V11 | Changes | Make a pipeline fail on your MR; move a ticket To do → In progress | One notification each, and a "changed" tag on the row. |
| V12 | Merged | Merge one of your MRs as a colleague (or have one merged) | "🔀 !123 merged". The row leaves "My MRs". |
| V13 | Bell | Click the bell; click an entry | The list is newest first and the number clears. The entry jumps to its row and highlights it. |
| V14 | Notification click | Click a macOS notification | The window opens on the row (or with the bell popup as fallback). |
| V15 | Restart | Quit and relaunch | No repeat notifications (SC-005). Bell history is kept. |
| V16 | New tab | Filter a list in GitLab, copy the URL, add it in Settings | Live "valid · Merge requests · Group login5", then the tab loads the same items (SC-007). |
| V17 | Errors | Use a URL for a non-existent project; revoke the token; turn Wi-Fi off | Per-tab "Project not found"; global "Token rejected"; "Can't reach GitLab" with last data kept. |
| V18 | Status fallback | If the status filter is unsupported | Orange warning bar, and the list is shown without the status filter. |
| V19 | Scrolling | Scroll a long list and the Settings body all the way down, then back up | Only the content scrolls; header and footer stay put; the top is reachable again. |
| V20 | Move & resize | Drag the empty part of the tab bar / Settings header; drag a window edge; close and reopen from the icon | The window moves and resizes (not below 380 × 420); buttons, tabs and the bell still click; on reopen it keeps the new position and size. |
| V21 | First sign-in | With no token, save a valid `read_api` token | "Test & save" succeeds and the app switches to the tabs view with the bell visible. |
| V22 | Close button | Click ✕ in the tab bar, reopen from the icon, open Settings and click ✕ there | The window hides each time; the menu bar icon stays and the app keeps polling. |
| V23 | Stays open | Open the window, click another app; click the menu bar icon; click it again | The window stays open behind the other app; the icon brings it to the front; the second click hides it. A new item in the open tab while another app has focus still notifies. |
| V24 | MR readiness | Look at an MR nobody approved (in a project with no approval rules), one with an open review thread, and one approved with all threads resolved | First shows "Needs approval"; second shows "Unresolved threads" and an orange "N to resolve"; only the third shows "Ready to merge". |
