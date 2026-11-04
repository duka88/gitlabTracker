# Implementation Plan: GitLab Tracker Menu Bar App

**Branch**: `001-gitlab-tracker-app` | **Date**: 2026-11-03 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-gitlab-tracker-app/spec.md`

## Summary

GitLab Tracker is a macOS menu bar app with a dropdown window of tabs. Each tab is a pasted GitLab list URL, turned into a GraphQL query and drawn in a GitLab-style row layout. It notifies about new items, comments from others, @mentions and key changes, through macOS notifications, an in-app bell, tab badges and a menu bar count.

It's built with Tauri 2:
- **Rust** owns the GitLab GraphQL client, URL translation, polling, the diff and notification engine, the Keychain token, persistence and the tray.
- **Vue 3 + TypeScript** only draws the screens, from a view model pushed by Rust over events.

## Technical Context

**Language/Version**: Rust stable (2021 edition, ≥ 1.80); TypeScript 5.x; Node 20+

**Primary Dependencies**:
- Rust: `tauri` 2 (`tray-icon`), `tauri-plugin-positioner` (tray position), `tauri-plugin-notification`, `tauri-plugin-opener` (open URLs), `sha2` (avatar cache keys), `reqwest` (rustls, json), `tokio`, `serde`/`serde_json`, `keyring` 3 (`apple-native`), `chrono`, `url`, `uuid`, `thiserror`, `futures`
- Frontend: `vue` 3, `vite`, `@tauri-apps/api` 2, `vitest`, `vue-tsc`

**Storage**: JSON files in the app data dir (`settings.json`, `state.json`, `notifications.json`), written atomically. Token in the macOS Keychain.

**Testing**: `cargo test` (URL mapping, diff engine, merge-status derivation, notification text, persistence round-trip; GraphQL parsing against recorded fixtures); Vitest (formatters); manual checks in [quickstart.md](./quickstart.md) V1–V27.

**Target Platform**: macOS 13+ (Apple Silicon + Intel, universal binary)

**Project Type**: desktop app (Tauri: Rust core + Vue webview)

**Performance Goals**: Window shows in < 0.5 s. A full poll of 4–8 tabs finishes in < 10 s on a normal connection. The UI handles 500 rows without jank.

**Constraints**:
- < 150 MB memory, no server, read-only `read_api`.
- Polls never overlap.
- The token never reaches the webview.
- Stay under the GitLab GraphQL complexity limit (`first: 50`).

**Scale/Scope**: 1 user, gitlab.com only, 4–8 tabs, ≤ 500 items per tab, ~10 IPC commands, 2 screens (Tabs, Settings) plus the bell popup.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | Gate | Pre-research | Post-design |
|---|---|---|---|
| I. Serverless, direct-to-GitLab | Only gitlab.com, with every request (including images) from Rust | ✅ | ✅ API calls in `gitlab/`; avatars through the `avatar://` scheme in `avatars.rs`; CSP blocks remote loads (research R13/R16) |
| II. Secrets in Keychain | Token only in the Keychain, never in the UI, files or logs | ✅ | ✅ `keyring`; IPC returns `UserInfo` only (contracts/tauri-ipc.md) |
| III. Rust owns behaviour | No GitLab calls or diff logic in Vue | ✅ | ✅ Vue gets the `AppView` and sends intents |
| IV. Read-only, quiet by default | Baseline on first load; each event notifies once | ✅ | ✅ Baselines, seen sets, visible-tab rule (data-model.md) |
| V. Testable core | Pure functions with unit tests | ✅ | ✅ `url_map`, `diff`, `merge_status`, `notify_text` are pure; fixtures for parsing |
| VI. Simplicity | One window and process, JSON files, minimal dependencies | ✅ | ✅ No Pinia or router; `tauri-nspanel` and `mac-notification-sys` deferred |
| Constraint: GraphQL for lists; REST only where GraphQL lacks data | Both tab kinds use GraphQL | ✅ | ✅ Comments, merged check, mentions and ticket lookup use GraphQL. REST only for commit ancestry in test-branch checks (`gitlab/branch_check.rs`, research R18), which GraphQL doesn't offer |

There are no violations, so Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-gitlab-tracker-app/
├── spec.md
├── plan.md              # this file
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/
│   ├── tauri-ipc.md
│   ├── url-mapping.md
│   └── gitlab-graphql.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks (not created here)
```

### Source Code (repository root)

```text
package.json, vite.config.ts, tsconfig.json, index.html
src/                                   # Vue 3 + TS (screens only)
├── main.ts
├── App.vue                            # router-less: Tabs view | Settings view
├── types.ts                           # mirrors Rust view types
├── ipc.ts                             # typed invoke/listen wrappers
├── store.ts                           # useAppState() composable fed by state://updated
├── util/{time.ts, labels.ts, text.ts} # relative time, scoped label split, truncation
├── components/
│   ├── TabBar.vue                     # tabs + count + new badge, refresh spinner, settings, bell
│   ├── BellPopup.vue
│   ├── IssueBar.vue                   # red error / orange warning
│   ├── MrRow.vue
│   ├── WorkItemRow.vue
│   ├── LabelChip.vue                  # scoped two-part chip
│   ├── AvatarStack.vue
│   ├── PipelineIcon.vue, TypeIcon.vue, StatusDot.vue, Icon.vue
│   ├── Footer.vue
│   └── settings/{TokenForm.vue, IntervalPicker.vue, TabEditor.vue}
└── styles/{tokens.css, base.css}      # GitLab Pajamas-like vars, light/dark

src-tauri/
├── Cargo.toml, tauri.conf.json, Info.plist (LSUIElement)
├── capabilities/default.json
├── icons/ (app icons + tray template icon)
├── src/
│   ├── main.rs, lib.rs                # builder, plugins, setup, command registration
│   ├── tray.rs                        # tray icon, menu, title badge, toggle/position window
│   ├── commands.rs                    # IPC handlers (contracts/tauri-ipc.md)
│   ├── app_state.rs                   # Arc<Mutex<Core>>, AppView builder, event emit
│   ├── model/{settings.rs, tab.rs, item.rs, state.rs, notification.rs}
│   ├── url_map.rs                     # parse_tab_url → TabQuery (pure)
│   ├── gitlab/
│   │   ├── client.rs                  # reqwest GraphQL client + rest_get (REST, FR-034), error mapping
│   │   ├── queries/*.graphql          # incl. tickets_by_iid.graphql (FR-036b)
│   │   ├── lists.rs                   # TabQuery → paged fetch → Vec<ListItem>
│   │   ├── notes.rs, mentions.rs, merged.rs
│   │   ├── branch_check.rs            # MRs on a test branch: ancestry via REST, cached (FR-034/037, R18)
│   │   ├── tickets.rs                 # ticket lookup by branch number (FR-036b, R19)
│   │   └── parse.rs                   # GraphQL JSON → ListItem (merge_status, linked ticket here)
│   ├── engine/
│   │   ├── poller.rs                  # interval, refresh Notify, in-flight guard
│   │   ├── diff.rs                    # baseline/new/comment/change detection (pure)
│   │   ├── notify_text.rs             # notification strings (pure)
│   │   └── notifier.rs                # Notifier trait + tauri-plugin-notification impl + click routing
│   ├── config.rs                      # GITLAB_HOST constant (gitlab.com only in v1)
│   ├── avatars.rs                     # avatar:// URI scheme: fetch via Rust, memory + disk cache (R16)
│   ├── secrets.rs                     # keyring wrapper
│   └── store.rs                       # atomic JSON persistence, defaults, migration
└── tests/
    ├── fixtures/*.json                # recorded GraphQL responses
    ├── url_map_test.rs
    ├── diff_*_test.rs, merge_status_test.rs, tabs_test.rs
    ├── branch_check_test.rs           # merge parsing/matching (FR-034)
    └── parse_test.rs
```

**Structure Decision**: The standard single-repo Tauri 2 layout (`src/` for the webview, `src-tauri/` for Rust), scaffolded with `pnpm create tauri-app` (Vue + TS template) and then trimmed. Domain logic sits in plain Rust modules (`url_map`, `engine/diff`, `gitlab/parse`) with no Tauri types, so they can be unit-tested without a running app.

## Key Design Decisions (see research.md for rationale)

1. **GraphQL everywhere**: lists, notes, the merged check and mentions (R2, R6, R7). One page = 50 items; follow the cursor up to 500.
2. **The poll pipeline** is strictly ordered: mentions → tabs → diff → comments → merged → visible-tab filter → count → persist → emit (data-model "Poll algorithm").
3. **De-duplication** uses global gids, note ids, to-do ids and change keys in bounded seen sets that persist across restarts.
4. **Two counters**: the menu bar count is distinct gids with markers outside the on-screen tab of the visible, focused window (FR-023a); the bell count is unread entries. Tab badges are per tab.
5. **Notification clicks** route through the activation heuristic, with the bell popup as fallback (R9). `Notifier` is a trait, so a precise click handler can be swapped in later.
6. **Wake handling**: a wall-clock watchdog triggers one refresh after sleep (R11).
7. **Graceful degradation**: status filter retry (R3), sort fallback warning, the work-item comment-count fallback chain (R5), and the last good data kept on errors.

## Risks

| Risk | Impact | Mitigation |
|---|---|---|
| GraphQL field names differ from research (marked VERIFY) | Queries fail | Check them first (quickstart prerequisite 6). Queries live in separate `.graphql` files, and parsing is tested on fixtures. |
| No user-notes count on WorkItem | No comment count on ticket rows | Fallback chain (R5 a→b→c) |
| Notification click target unknown on desktop | FR-024 precision | Bell popup fallback is part of the spec. `mac-notification-sys` upgrade path. |
| Query complexity over the limit | Tab errors | Page size 25 and nested `first:` caps |
| No network or Rust on the dev machine right now | Can't build | Install the prerequisites before `/speckit-implement` |

## Complexity Tracking

No constitution violations.
