---
description: "Task list for GitLab Tracker menu bar app"
---

# Tasks: GitLab Tracker Menu Bar App

**Input**: Design documents from `/specs/001-gitlab-tracker-app/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Definition of done**: see Notes. Every task must leave `cargo test`, `cargo clippy`, `vue-tsc` and Vitest green.

**Tests**: Constitution principle V requires unit tests for pure core logic (URL mapping, diff engine, merge-status derivation, notification text, persistence, GraphQL parsing) and for TS formatters. Those test tasks are included. UI and OS integration are verified manually with quickstart.md V1–V18.

**Organization**: Tasks are grouped by user story from spec.md. Story labels match the spec: US1–US7.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: The user story the task belongs to
- Paths: `src/` = Vue webview, `src-tauri/` = Rust core (plan.md → Project Structure)

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Toolchain, scaffold and project configuration

- [X] T001 Verify prerequisites from quickstart.md: network access, `xcode-select -p`, install Rust via rustup and add the `aarch64-apple-darwin` and `x86_64-apple-darwin` targets, and confirm `cargo --version` and `pnpm --version`
- [X] T002 Check every **VERIFY** item in specs/001-gitlab-tracker-app/research.md (R2, R3, R4, R5 work-item comment count, R7 todos enum and `note` field, R14 label name) in https://gitlab.com/-/graphql-explorer. Record the outcomes in a "Verification results" section at the end of research.md, and save one raw JSON response per query (current user, dashboard MRs, group work items, MR notes, work-item notes, todos, merged check) to src-tauri/tests/fixtures/
- [X] T003 Scaffold the Tauri 2 + Vue + TypeScript app in the repo root (`pnpm create tauri-app` with the vue-ts template, then merge into the existing repo without touching .specify/, .claude/ or specs/). Set the app name "GitLab Tracker" and identifier `com.duka88.gitlab-tracker` in src-tauri/tauri.conf.json
- [X] T004 Add Rust dependencies to src-tauri/Cargo.toml: tauri (features `tray-icon`, `image-png`), tauri-plugin-positioner (feature `tray-icon`), tauri-plugin-notification, tauri-plugin-opener, reqwest (default-features off; `rustls-tls`, `json`), tokio (`full`), serde, serde_json, keyring 3 (`apple-native`), chrono (`serde`), url, uuid (`v4`, `serde`), thiserror, futures. Dev dependency: pretty_assertions
- [X] T005 [P] Add frontend dev dependencies and scripts to package.json: vitest, vue-tsc, and the scripts `typecheck` (`vue-tsc --noEmit`), `test` (`vitest run`) and `tauri`
- [X] T006 [P] Configure the window in src-tauri/tauri.conf.json: a single window `main` (480×640, `visible:false`, `decorations:false`, `resizable:false`, `alwaysOnTop:true`, `skipTaskbar:true`, `transparent:false`), CSP `default-src 'self'; img-src 'self' avatar: data:; style-src 'self' 'unsafe-inline'` (no remote loads; research R13/R16), and bundle targets `app` and `dmg` with `macOS.signingIdentity: "-"`
- [X] T007 [P] Create src-tauri/Info.plist with `LSUIElement = true` and reference it from the tauri.conf.json bundle config (no Dock icon, FR-001)
- [X] T008 [P] Create src-tauri/capabilities/default.json allowing core events, `notification:default`, `opener:allow-open-url` (scoped to `https://gitlab.com/*`) and `positioner:default`
- [X] T009 [P] Add a tray template icon (monochrome 16/32 px PNG, `iconAsTemplate`) at src-tauri/icons/tray.png and generate app icons with `pnpm tauri icon` into src-tauri/icons/
- [X] T010 [P] Configure Rust lints (`[lints.clippy] pedantic = "warn"` with sensible allows) in src-tauri/Cargo.toml, and add rustfmt.toml at src-tauri/rustfmt.toml
- [X] T011 [P] Add .gitignore entries for node_modules/, dist/, src-tauri/target/ and src-tauri/gen/ in .gitignore

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types, GitLab client, persistence, poller skeleton, IPC, tray and UI shell that every story builds on

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

### Models and persistence

- [X] T012 [P] Define `Settings`, `UserInfo`, `Tab`, `ListKind` with serde camelCase, plus `Settings::default()` including the 4 default tabs from research R14 (all notify switches on), in src-tauri/src/model/settings.rs and src-tauri/src/model/tab.rs
- [X] T013 [P] Define `ListItem` (common + `MrFields` + `WorkItemFields`), `Person`, `Label` (with `scope()`/`value()` split on the last `::`), `PipelineStatus`, `MergeStatusTag`, `Reviewer`, `Health` and `Children` in src-tauri/src/model/item.rs
- [X] T014 [P] Define `TabState`, `Tracked`, `TrackedState` (Mr/WorkItem variants), `ChangeKind`, `TabIssue`, `BoundedSet<T>` (FIFO cap) and `GlobalState` (seen sets, baselines) in src-tauri/src/model/state.rs
- [X] T015 [P] Define `NotificationEntry` and `NotificationKind` (`NewItem | NewItems | Comment | Change | Mention`) and `NotificationLog` (newest 100, `unread_count()`, `mark_all_read()`) in src-tauri/src/model/notification.rs
- [X] T016 Create the src-tauri/src/model/mod.rs re-exports, plus `AppView` and `TabView` view-model structs (data-model.md "UI view model") in src-tauri/src/model/view.rs
- [X] T017 Implement atomic JSON persistence (`load_or_default<T>`, `save<T>` via temp file + rename, corrupt file → default + warning) for settings.json, state.json and notifications.json in the app data dir, in src-tauri/src/store.rs
- [X] T018 [P] Unit tests for persistence (round-trip, corrupt file fallback, BoundedSet cap) in src-tauri/tests/store_test.rs

### Secrets and GitLab client

- [X] T019 [P] Implement a Keychain wrapper (`get_token()`, `set_token(token)`, `delete_token()`) with service `com.duka88.gitlab-tracker` and account `gitlab-token:gitlab.com`, in src-tauri/src/secrets.rs
- [X] T020 Implement `GitLabClient { token, http }` (base URL from `GITLAB_HOST`) with `graphql<T>(query, vars)`: a 20 s timeout and the error mapping from contracts/gitlab-graphql.md into `AppError` (`TokenRejected`, `NoPermission`, `NotFound`, `RateLimited`, `Unreachable`, `Query`), in src-tauri/src/gitlab/client.rs and src-tauri/src/error.rs
- [X] T021 [P] Add a `CurrentUser` query file at src-tauri/src/gitlab/queries/current_user.graphql, and `fetch_current_user()` in src-tauri/src/gitlab/user.rs
- [X] T022 [P] Implement the `avatar://` async URI scheme (register with `register_asynchronous_uri_scheme_protocol`; decode the percent-encoded URL; allow only `https://gitlab.com` (Gravatar and any other host → 404, no fetch, constitution I); fetch with reqwest; LRU memory cache of 300 entries + disk cache `app_cache_dir()/avatars/<sha256>` kept 7 days; 404 on failure so the UI falls back to initials), per research R16 and contracts/tauri-ipc.md "Custom URI scheme", in src-tauri/src/avatars.rs, and add `sha2` to src-tauri/Cargo.toml
- [X] T023 [P] Add the `GITLAB_HOST` constant (`https://gitlab.com`, research R17) and the helpers `absolute_url(path)` and `avatar_proxy_url(url)`, in src-tauri/src/config.rs

### App core, poller, IPC, tray, UI shell
claude
- [X] T024 Implement `Core` (settings, global state, tab states, notification log, `active_tab_id`, `window_visible`) behind `Arc<tokio::sync::Mutex<_>>`, with `build_view()` and `emit_view(app)` (event `state://updated`), in src-tauri/src/app_state.rs
- [X] T025 Implement the poller skeleton: a tokio task with `interval(settings.interval)` and `MissedTickBehavior::Delay`, a `Notify` for refresh, an `AtomicBool` in-flight guard (FR-018), emitting `poll://started`, calling a `run_poll(core)` stub, and restarting the interval on `set_interval`, in src-tauri/src/engine/poller.rs
- [X] T026 Add a wake-from-sleep watchdog: every 30 s compare `SystemTime::now()` with the previous reading, and on a jump of more than 60 s trigger one refresh through the poller's `Notify` (the in-flight guard prevents bursts), per research R11, in src-tauri/src/engine/poller.rs
- [X] T027 Implement tray setup: `TrayIconBuilder` with the template icon, `show_menu_on_left_click(false)`, a right-click menu (Open / Refresh now / Quit), left-click toggle of window `main` positioned with `Position::TrayBottomCenter`, and `set_badge(count)` via `set_title`, in src-tauri/src/tray.rs
- [X] T028 Wire everything in src-tauri/src/lib.rs: declare the domain modules as `pub mod` (`model`, `url_map`, `gitlab`, `engine`, `store`, `config`) so `src-tauri/tests/*.rs` can import them; register the `avatar` URI scheme and the plugins (positioner, notification, opener); `set_activation_policy(Accessory)`, load the store and token, spawn the poller, hide the window on `WindowEvent::Focused(false)`, track `window_visible` on show/hide, register commands, and open Settings via `nav://settings` when there's no token (FR-007)
- [X] T029 Implement the command handlers `get_view`, `refresh_now`, `select_tab` (clears markers for the tab), `open_url` (host-checked, via opener) and `quit` per contracts/tauri-ipc.md, in src-tauri/src/commands.rs
- [X] T030 [P] Create TS types mirroring the Rust view types (`AppView`, `TabView`, `Tab`, `ListItem`, `Label`, `Person`, `NotificationEntry`, `UrlCheck`, `AppError`) in src/types.ts
- [X] T031 [P] Create typed `invoke`/`listen`/`emit` wrappers for every command and event in contracts/tauri-ipc.md, in src/ipc.ts
- [X] T032 Create the `useAppState()` composable (reactive `AppView`, subscribes to `state://updated`, `poll://started`, `nav://*`, exposes actions, and emits `ui://visibility` on window show/hide and tab change) in src/store.ts
- [X] T033 [P] Create GitLab Pajamas-like CSS tokens (font stack, grays, blue/orange/red/green, label chip radii, light and dark via `prefers-color-scheme`) and base styles in src/styles/tokens.css and src/styles/base.css
- [X] T034 [P] Create the inline SVG icon set component (pipeline passed/failed/running/pending/canceled, issue/task/bug/epic/incident, lock, comment, thumbs up/down, bell, gear, refresh, clock, milestone, iteration, weight, branch) in src/components/Icon.vue
- [X] T035 Create the App shell: view switch between Tabs and Settings, a `nav://settings` handler, and mount with styles, in src/App.vue and src/main.ts

**Checkpoint**: The app launches as a tray-only app, the window toggles under the icon and hides when it loses focus, the right-click menu works, and the view model reaches the UI

---

## Phase 3: User Story 1 - See my GitLab lists from the menu bar (Priority: P1) 🎯 MVP

**Goal**: Sign in with a token, then see the 4 default tabs filled with GitLab-style MR and work-item rows. Clicking a row opens it in the browser.

**Independent Test**: quickstart.md V1–V6: save a valid token, open each default tab, compare it with GitLab, hover and click rows.

### Tests for User Story 1

- [X] T036 [P] [US1] Table-driven tests for every row in contracts/url-mapping.md "Test examples", plus array and comma label forms, `ignored_params`, dashboard work-item rejection and the unmappable-sort fallback, in src-tauri/tests/url_map_test.rs
- [X] T037 [P] [US1] Merge-status derivation tests covering every row of the research R5 table, in src-tauri/tests/merge_status_test.rs
- [X] T038 [P] [US1] Parsing tests turning the T002 fixtures into `ListItem`s (MR and work item, including avatar URLs turned into `avatar://` proxy URLs and the description preview cut to at most 300 plain-text characters at a word boundary with "…"), in src-tauri/tests/parse_test.rs
- [X] T039 [P] [US1] Vitest tests for `relativeTime` ("3h ago"), `splitScopedLabel`, `labelTextColor` fallback, `isOverdue`, in src/util/__tests__/util.test.ts

### Implementation for User Story 1

- [X] T040 [US1] Implement `parse_tab_url(url) -> Result<TabQuery, UrlError>` (host fixed to gitlab.com; dashboard work-item URLs rejected; unmappable sort → `UPDATED_DESC` with `sort_fallback`) per contracts/url-mapping.md (paths, params, dashboard relation, ignored params, errors), in src-tauri/src/url_map.rs
- [X] T041 [P] [US1] Write MR list queries (dashboard: authored/reviewRequested/assigned variants; group; project) with all R5 MR fields, `first:50`, `after`, and nested caps, in src-tauri/src/gitlab/queries/mrs_dashboard.graphql, mrs_group.graphql and mrs_project.graphql
- [X] T042 [P] [US1] Write work-item list queries (group, project) with all R5 widgets, the `status` variable (shape from T002), `not` filters and the comment-count source chosen in T002, in src-tauri/src/gitlab/queries/work_items_group.graphql and work_items_project.graphql
- [X] T043 [US1] Implement `merge_status(&MrRaw) -> Option<MergeStatusTag>` and the GraphQL JSON → `ListItem` mapping (`avatar://` proxy URLs via `config::avatar_proxy_url`, plain-text description preview of at most 300 characters cut at a word boundary, label scope split, approvals, reviewer approved flags, child progress, health), in src-tauri/src/gitlab/parse.rs
- [X] T044 [US1] Implement `fetch_tab(client, &TabQuery) -> Result<FetchResult>`: build variables from the filters, choose the query by scope and kind, follow `pageInfo.endCursor` up to 500 items (set `truncated_total`), and set the tab warning "Sort not supported — sorted by last updated" when `sort_fallback` is set. If T002 chose comment-count fallback (b), also run the extra `WorkItemCommentCounts` query. In src-tauri/src/gitlab/lists.rs
- [X] T045 [US1] Fill `run_poll` (first version): fetch all tabs concurrently (max 4), store `items`, `tracked` and `last_updated` per tab, keep old items and set `error` on failure, persist, and emit the view, in src-tauri/src/engine/poller.rs
- [X] T046 [US1] Implement the token commands `test_token`, `save_token` (Keychain + settings.user, then refresh) and `remove_token` per contracts/tauri-ipc.md, in src-tauri/src/commands.rs
- [X] T047 [P] [US1] Implement formatters (`relativeTime`, `splitScopedLabel`, `labelTextColor`, `isOverdue`, `formatDueDate`) in src/util/time.ts and src/util/labels.ts
- [X] T048 [P] [US1] Build the LabelChip component (plain chip using the GitLab colour and text colour; scoped label as a two-part chip, `scope | value`, outlined right half), in src/components/LabelChip.vue
- [X] T049 [P] [US1] Build the AvatarStack component (`<img src>` uses the `avatar://` URL from the view model, never a GitLab URL; on image error show an initials circle; up to 4 avatars + "+N", tooltip with names, optional green check badge for approved reviewers), in src/components/AvatarStack.vue
- [X] T050 [P] [US1] Build the PipelineIcon, TypeIcon and StatusDot components (status colour dot + name), in src/components/PipelineIcon.vue, src/components/TypeIcon.vue and src/components/StatusDot.vue
- [X] T051 [US1] Build the MrRow component following FR-025 and spec Story 1 scenario 5. Line 1: title + Draft tag. Line 2: `!iid · created X ago by Name · target branch · milestone`. Then labels. Right side: pipeline, approvals "1/2", assignees, reviewers with approved marks, 💬 count, 👍/👎, "updated X ago", merge-status tag. In src/components/MrRow.vue
- [X] T052 [US1] Build the WorkItemRow component following FR-026 and spec Story 1 scenario 6. Line 1: type icon, title, lock if confidential. Line 2: `#iid · created X ago by Name · milestone · iteration · due date (red if overdue) · weight`. Then status dot and labels. Right side: assignees, 💬 count, child progress "2/5", health badge, "updated X ago". In src/components/WorkItemRow.vue
- [X] T053 [US1] Add row behaviour to both row components: click → `open_url(webUrl)`; hover tooltip with full title and description preview (FR-028); title truncation and label wrapping. Put the shared logic in src/components/RowShell.vue
- [X] T054 [US1] Build TabBar (tab name + item count, active state → `select_tab`, Refresh button spinning between `poll://started` and the next `state://updated`, Settings button) and the tab list view rendering MrRow or WorkItemRow by kind with an empty state, in src/components/TabBar.vue and src/components/TabList.vue
- [X] T055 [US1] Build the Footer ("Updated 1m ago · every 2 min", refreshed every 30 s; Quit button) in src/components/Footer.vue
- [X] T056 [US1] Build the Settings token form (paste token, "Test & save" → `save_token`, show "signed in as @username" or an error, remove token) and the Settings view container, in src/components/settings/TokenForm.vue and src/components/settings/SettingsView.vue

**Checkpoint**: The MVP. All 4 default tabs show real data in GitLab-style rows (SC-002, SC-008)

---

## Phase 4: User Story 2 - Get notified about new items (Priority: P1)

**Goal**: Baseline on first load. After that, new items get NEW markers, tab badges, the menu bar count, macOS notifications and a bell entry. Opening a tab clears its markers. The visible-tab rule applies.

**Independent Test**: quickstart.md V7, V8, V13 (bell), V15 (restart).

### Tests for User Story 2

- [X] T057 [P] [US2] Diff engine tests: first load = baseline only; added item → `new_items` + one `NewItem` notification; ≥2 → one `NewItems` "N new in Tab"; notify switch off → markers but no notification; same gid in two tabs → one notification; item leaves and returns in a later poll → notifies again; same new item in the visible tab and a hidden tab in one poll → no notification and counted in neither badge (active tab compared first); visible-tab rule → marker, gid in `seen_while_visible`, no notification, no badge or menu count, and still not counted after the window hides; URL change resets the baseline. In src-tauri/tests/diff_new_items_test.rs
- [X] T058 [P] [US2] Notification text tests ("New in Reviewing: !123 Title", "3 new in My tickets"), in src-tauri/tests/notify_text_test.rs
- [X] T059 [P] [US2] Count tests: tab badge = markers minus the global `seen_while_visible` (also in other tabs holding the same gid); gid pruned from `seen_while_visible` once no tab has a marker; menu count = distinct gids across tab badges; mentions never counted; counts unchanged when the window hides, in src-tauri/tests/menu_count_test.rs

### Implementation for User Story 2

- [X] T060 [US2] Implement the pure diff for new items, `diff_new_items(prev, current, ctx) -> DiffOutcome { new_items, notifications }`, applying baseline, switch, cross-tab de-duplication within the poll (`poll_notified`, in memory, not persisted) and the visible-tab rule, which compares the active tab first and adds its gids to the global `seen_while_visible` and their keys to `poll_notified` (FR-019, FR-020, FR-023a), in src-tauri/src/engine/diff.rs
- [X] T061 [P] [US2] Implement notification text builders for new items in src-tauri/src/engine/notify_text.rs
- [X] T062 [US2] Implement the `Notifier` trait and its `tauri-plugin-notification` implementation (request permission on first use; send title + body). Add each sent notification to `NotificationLog` (persisted, newest 100), and record `last_sent_at` and the last entry id for click routing, in src-tauri/src/engine/notifier.rs
- [X] T063 [US2] Integrate the diff into `run_poll`: apply the outcome to `TabState.new_items`, send notifications through the Notifier, recompute tab badges and `menu_count` excluding `seen_while_visible` (FR-004, data-model step 9) and update the tray badge, in src-tauri/src/engine/poller.rs
- [X] T064 [US2] Make `select_tab` and window-show clear the active tab's `new_items`/`new_comments`/`changes`, prune the global `seen_while_visible` of gids no tab has a marker for, recompute the count, update the badge and emit the view (FR-023). Handle `ui://visibility` to keep `active_tab_id` and `window_visible` current, in src-tauri/src/commands.rs and src-tauri/src/app_state.rs
- [X] T065 [US2] Implement the `open_bell` and `open_entry` commands per contracts/tauri-ipc.md (mark all read; route to `{row}` or open the browser), in src-tauri/src/commands.rs
- [X] T066 [US2] Implement notification-click routing (FR-024, research R9): on app activation or reopen within 10 s of the last notification, emit `nav://row` when exactly one unread entry was sent since the window was last shown, otherwise `nav://bell`, then show the window. In src-tauri/src/engine/notifier.rs and src-tauri/src/lib.rs
- [X] T067 [P] [US2] Add NEW markers to the row components (orange left border + NEW tag when the gid is in `newItems`) in src/components/RowShell.vue
- [X] T068 [P] [US2] Add an orange new-count badge per tab in TabBar (FR-030) in src/components/TabBar.vue
- [X] T069 [US2] Build the BellPopup (bell icon at top right with an unread number; popup of entries newest first with text, tab name and relative time; opening calls `open_bell`; clicking an entry calls `open_entry` and, for a `row` target, switches tab, scrolls to the row and highlights it for 2 s). Handle `nav://row` and `nav://bell` the same way. In src/components/BellPopup.vue and src/components/TabList.vue
- [X] T070 [US2] Add the interval picker (1/2/5/10 → `set_interval`) to Settings in src/components/settings/IntervalPicker.vue

**Checkpoint**: New items notify within one interval. The counts and the bell behave as described in the spec (SC-003, SC-006)

---

## Phase 5: User Story 3 - Get notified about new comments from others (Priority: P2)

**Goal**: A higher comment count leads to fetching the notes, then one notification per comment from someone else, on MRs and work items, de-duplicated across tabs and restarts.

**Independent Test**: quickstart.md V9 and V15.

### Tests for User Story 3

- [X] T071 [P] [US3] Comment diff tests: count up → item queued; notes filtered (system, own, seen, before `baseline_at`); one notification per note; `new_comments[gid]` incremented; same note in two tabs → once; visible-tab rule; first sight of an item → no notifications. In src-tauri/tests/diff_comments_test.rs
- [X] T072 [P] [US3] Comment text tests ("💬 Marko on !123", "💬 Marko on #45", snippet at most 100 characters cut at a word boundary with "…", Markdown stripped), in src-tauri/tests/notify_text_test.rs

### Implementation for User Story 3

- [X] T073 [US3] Implement `RecentNotesBatch`: build an aliased GraphQL query for up to 10 items (MR notes via `project.mergeRequest.notes(last:20)`, work-item notes via the `WorkItemWidgetNotes` discussions), and parse into `Note { id, body, system, author, created_at }`, in src-tauri/src/gitlab/notes.rs
- [X] T074 [US3] Implement the pure comment diff (`detect_comment_candidates`, `filter_new_notes`) and the comment text builder, in src-tauri/src/engine/diff.rs and src-tauri/src/engine/notify_text.rs
- [X] T075 [US3] Integrate into `run_poll` after the list diff: queue candidates (when `notify_new_comments` is on), fetch notes in batches, update `seen_comment_ids` and `new_comments`, and send notifications, in src-tauri/src/engine/poller.rs
- [X] T076 [P] [US3] Show the new-comment marker on rows: orange left border and an orange "💬 12 (+2)" count from `newComments[gid]` (FR-029a), in src/components/RowShell.vue, src/components/MrRow.vue and src/components/WorkItemRow.vue

**Checkpoint**: Comments from others notify exactly once. Your own comments and system notes never do (SC-004)

---

## Phase 6: User Story 4 - Get notified about changes to my items (Priority: P2)

**Goal**: Pipeline failed, approved, changes requested and merged on MRs; status and label changes on work items. One notification per change, plus a "changed" tag on the row.

**Independent Test**: quickstart.md V11 and V12.

### Tests for User Story 4

- [X] T077 [P] [US4] Change diff tests: each `ChangeKind` detected from `TrackedState` pairs; new and changed in the same check → only "new"; repeated poll with unchanged state → no repeat; fail → pass → fail across polls → notifies twice; same change in two tabs in one poll → once; own "changes requested" skipped; merged check result → notification, closed → none; visible-tab rule. In src-tauri/tests/diff_changes_test.rs

### Implementation for User Story 4

- [X] T078 [US4] Build `TrackedState` from `ListItem` and implement the pure `diff_changes(prev, current, ctx)` with change keys (research R8), in src-tauri/src/engine/diff.rs
- [X] T079 [P] [US4] Implement `MergedCheckBatch` (aliased `project.mergeRequest(iid){state mergedAt mergeUser{username}}`) in src-tauri/src/gitlab/merged.rs
- [X] T080 [P] [US4] Add change notification texts ("❌ Pipeline failed on !123", "✅ !123 approved", "✋ Changes requested on !123 by Name", "🔀 !123 merged", "#45 To do → In progress", "#45 +Ready to Merge −In Testing") in src-tauri/src/engine/notify_text.rs
- [X] T081 [US4] Integrate into `run_poll`: change detection (when `notify_changes` is on), MR ids removed from open-MR tabs → merged check (skip when `mergeUser` is me), update `changes` markers and send notifications, in src-tauri/src/engine/poller.rs
- [X] T082 [P] [US4] Show the "changed" tag on rows naming what changed (e.g. "pipeline failed", "status"), with an orange border (FR-022d), in src/components/RowShell.vue

**Checkpoint**: Change notifications work for all tabs with "notify on changes" on

---

## Phase 7: User Story 5 - Get notified when someone mentions me (Priority: P2)

**Goal**: @mentions anywhere on GitLab notify once. Mentions win over comment notifications. The bell entry opens the row or the browser.

**Independent Test**: quickstart.md V10.

### Tests for User Story 5

- [X] T083 [P] [US5] Mention tests: first poll = baseline; new to-do → one Mention notification; `note.id` added to `seen_comment_ids` so the comment doesn't notify again; to-do already seen → none; item in the visible tab → none; `notify_mentions` off → none. In src-tauri/tests/mentions_test.rs

### Implementation for User Story 5

- [X] T084 [US5] Write the Mentions query (shape from T002) and `fetch_mentions()` → `Vec<Mention { id, author, body, created_at, target_ref, target_url, target_gid, note_id }>` in src-tauri/src/gitlab/queries/mentions.graphql and src-tauri/src/gitlab/mentions.rs
- [X] T085 [US5] Implement the pure `diff_mentions` and the "📣 Name mentioned you on #78" text, in src-tauri/src/engine/diff.rs and src-tauri/src/engine/notify_text.rs
- [X] T086 [US5] Integrate mentions as the first poll step (before tabs, so de-duplication favours mentions), store the entry with `item_gid` (for row routing when the item is in a tab) and `web_url`. Mentions add to the bell only, never to the tab badges or the menu count (FR-022g), in src-tauri/src/engine/poller.rs
- [X] T087 [US5] Add the `set_notify_mentions` command and the Settings toggle "Notify when I'm mentioned", in src-tauri/src/commands.rs and src/components/settings/SettingsView.vue

**Checkpoint**: Mentions on items outside the tabs reach the user once (SC-004a)

---

## Phase 8: User Story 6 - Manage tabs and settings (Priority: P2)

**Goal**: Add, rename, reorder and delete tabs from pasted GitLab URLs with a live valid/invalid hint, and toggle the three notify switches per tab.

**Independent Test**: quickstart.md V16.

### Implementation for User Story 6

- [X] T088 [US6] Implement the `validate_url` command (returns `UrlCheck` with kind, scope label, filter summary, ignored params, error) using `parse_tab_url`, in src-tauri/src/commands.rs
- [X] T089 [US6] Implement the `save_tabs` command (validate every URL, assign UUIDs, reset `TabState` where the URL fingerprint changed, drop the state of deleted tabs, persist, trigger a poll) and `reset_default_tabs`, in src-tauri/src/commands.rs and src-tauri/src/app_state.rs
- [X] T090 [P] [US6] Unit tests for `save_tabs` state handling (rename keeps the baseline, URL change resets it, delete removes state, reorder keeps ids), in src-tauri/tests/tabs_test.rs
- [X] T091 [US6] Build the TabEditor (list of tabs with name input, URL input with a debounced live `validate_url` hint "✓ valid · Merge requests · Group login5 · 3 filters (ignored: foo)" or "✗ reason", toggles for new items, comments and changes, up/down reorder, delete with inline confirm, "Add tab", "Reset to defaults", Save), in src/components/settings/TabEditor.vue
- [X] T092 [US6] Mount the TabEditor, IntervalPicker and mentions toggle in SettingsView with a Back button to the tabs, in src/components/settings/SettingsView.vue

**Checkpoint**: A copied GitLab URL becomes a working tab (SC-007)

---

## Phase 9: User Story 7 - Clear errors and graceful degradation (Priority: P3)

**Goal**: Red and orange bars with clear messages, status-filter retry, last data kept when offline.

**Independent Test**: quickstart.md V17 and V18.

### Tests for User Story 7

- [X] T093 [P] [US7] Tests for the error mapping (401/403/null root/429/network → `AppError` kind and message) and the status-filter retry decision (`is_status_filter_error`), in src-tauri/tests/errors_test.rs

### Implementation for User Story 7

- [X] T094 [US7] Implement the work-item status-filter retry: on a schema/argument error mentioning `status`, rerun without status and set the tab warning "Status filter not supported — showing all statuses" (FR-017, research R3), in src-tauri/src/gitlab/lists.rs
- [X] T095 [US7] Set `global_error` for `TokenRejected` and `Unreachable`, and per-tab `error`/`warning` for the others (including "showing 500 of N" truncation). Clear them on the next success, in src-tauri/src/engine/poller.rs
- [X] T096 [US7] Build IssueBar (red for errors, orange for warnings; a "Token rejected" bar with an "Open Settings" action) and show it as a global bar above the tabs and a per-tab bar above the list, in src/components/IssueBar.vue and src/components/TabList.vue

**Checkpoint**: Every error scenario in spec Story 7 shows the right bar, and the other tabs keep working

---

## Phase 9b: Window, Settings, MR status, ticket and test-branch features (2026-11-04, from first manual run)

**Goal**: Fix scrolling, allow dragging and resizing, add a close button, and leave Settings after the first sign-in (FR-002, FR-002a–d, FR-007), and fix MR readiness and show unresolved threads (FR-025a, FR-025b), add test-branch tabs and the global test-branch flag (FR-034, FR-035, FR-037), and show each MR's ticket and approval (FR-036, FR-036a, FR-036b).

**Independent Test**: quickstart.md V19–V27.

- [X] T103 [P] Make only the content scroll: `min-height: 0` on `.settings`, `.settings .body` and `.list`; `overflow: hidden` on `html`/`body`/`#app`/`.app` (FR-002b), in src/components/settings/SettingsView.vue, src/components/TabList.vue, src/App.vue and src/styles/base.css
- [X] T104 [P] Make the tab bar and Settings header drag regions (`data-tauri-drag-region="deep"`), exclude the bell popup and its backdrop (`="false"`), and allow `core:window:allow-start-dragging` (FR-002a), in src/components/TabBar.vue, src/components/settings/SettingsView.vue, src/components/BellPopup.vue and src-tauri/capabilities/default.json
- [X] T105 Make the window resizable (min 380 × 420), and snap it under the tray icon only until that has worked once, so later opens keep the user's position and size (FR-002a), in src-tauri/tauri.conf.json and src-tauri/src/tray.rs
- [X] T106 [P] Switch to the tabs view when a state update goes from signed out to signed in (FR-007), in src/store.ts
- [X] T108 [P] Add a close (✕) button to the tab bar and Settings header that hides the window through a new `hide_window` command (FR-002c), in src-tauri/src/commands.rs, src-tauri/src/lib.rs, src/ipc.ts, src/store.ts, src/components/TabBar.vue and src/components/settings/SettingsView.vue
- [X] T109 Keep the window open on focus loss: drop the hide-on-blur handler, set `alwaysOnTop: false`, make the icon hide only a focused window and otherwise bring it forward, and clear the active tab's markers when focus returns via a shared `mark_window_seen` (FR-002, FR-002d), in src-tauri/src/lib.rs, src-tauri/src/tray.rs and src-tauri/tauri.conf.json
- [X] T110 Fix the MR approval and readiness rules and show unresolved threads: add `approvedBy`, `resolvableDiscussionsCount` and `resolvedDiscussionsCount` to the three MR queries and fixtures; count approvals from `approvedBy`; "approved" needs at least `max(required, 1)` approvers; unresolved threads > 0 → "Unresolved threads" tag and an orange "N to resolve" count on the row (FR-025a, FR-025b), in src-tauri/src/gitlab/queries/mrs_*.graphql, src-tauri/tests/fixtures/mrs_*.json, src-tauri/src/gitlab/parse.rs, src-tauri/src/model/item.rs, src-tauri/tests/merge_status_test.rs, src/types.ts, src/components/MrRow.vue and src/components/rowParts.css
- [X] T111 Add test-branch tabs: `Tab.test_branch` (validated in `apply_tab_edits`, part of the baseline fingerprint everywhere incl. the poll loop), `sourceBranch`/`diffHeadSha` in the MR queries and fixtures, `GitLabClient::rest_get`, `gitlab::branch_check::filter_on_branch` with the ancestry checks and cache of research R18, poller hook, "Only MRs merged into branch" field in the tab editor, and the source branch plus "on/outdated on <branch>" tag on MR rows; tests for merge parsing/matching, tab validation and baseline retention (FR-034, FR-035), in src-tauri/src/model/tab.rs, src-tauri/src/model/item.rs, src-tauri/src/app_state.rs, src-tauri/src/engine/poll.rs, src-tauri/src/engine/poller.rs, src-tauri/src/gitlab/client.rs, src-tauri/src/gitlab/branch_check.rs, src-tauri/src/gitlab/parse.rs, src-tauri/src/gitlab/queries/mrs_*.graphql, src-tauri/tests/branch_check_test.rs, src-tauri/tests/tabs_test.rs, src-tauri/tests/diff_new_items_test.rs, src/types.ts, src/components/settings/TabEditor.vue and src/components/MrRow.vue
- [X] T112 Show the MR's linked ticket and hold back "Ready to merge" until the ticket is ready: `workItemRelations(first: 10)` in the three MR queries and the project fixture; `pick_ticket` (CLOSES, else the branch-number match) and `ticket_blocks_merge`; new `MergeStatusTag::Approved`; ticket chip (reference · state, struck through when closed, opens the ticket) and an explicit "approved"/"not approved" on MR rows; tests for ticket picking, the Approved tag and the fixture (FR-036, FR-036a), in src-tauri/src/gitlab/queries/mrs_*.graphql, src-tauri/tests/fixtures/mrs_project_page1.json, src-tauri/src/gitlab/parse.rs, src-tauri/src/model/item.rs, src-tauri/tests/merge_status_test.rs, src-tauri/tests/parse_test.rs, src/types.ts and src/components/MrRow.vue
- [X] T113 Add the global "Test branch" setting that flags every MR in every MR tab: `Settings.test_branch` + `normalize_branch`, `set_test_branch` command, view field, `filter_on_branch(…, keep_all)` called from the poller for MR tabs without their own test branch, and a "Test branch" card in Settings; set to `resource-test` for the user (FR-037), in src-tauri/src/model/settings.rs, src-tauri/src/model/view.rs, src-tauri/src/app_state.rs, src-tauri/src/commands.rs, src-tauri/src/lib.rs, src-tauri/src/engine/poller.rs, src-tauri/src/gitlab/branch_check.rs, src-tauri/tests/tabs_test.rs, src/ipc.ts and src/components/settings/SettingsView.vue
- [X] T114 Look up tickets missing from an MR's relations by the branch number: `branch_ticket_iid`, `ticket_from`, `queries/tickets_by_iid.graphql`, `gitlab::tickets::{missing_tickets, apply_tickets, fill_missing_tickets}` called from the poller for every tab, re-applying the Approved rule; test with a parsed lookup (FR-036b), in src-tauri/src/gitlab/parse.rs, src-tauri/src/gitlab/tickets.rs, src-tauri/src/gitlab/queries/tickets_by_iid.graphql, src-tauri/src/gitlab/mod.rs, src-tauri/src/engine/poller.rs and src-tauri/tests/merge_status_test.rs
- [X] T115 Show the ticket's state as its coloured `state::` label chip on the right of the MR row, next to the test-branch and merge-status tags (grey status tag as fallback, faded when closed, opens the ticket); the meta line keeps only the ticket reference (FR-036), in src/components/MrRow.vue
- [X] T116 Sync the design docs with T103–T115: new IPC commands `hide_window`/`set_test_branch`, `Tab.testBranch`, view fields and the new tray/window behaviour in contracts/tauri-ipc.md; the extended `MrFields` fragment, `TicketsByIid` and the REST calls in contracts/gitlab-graphql.md; new modules and tests in plan.md
- [X] T117 Resolve the /speckit-analyze findings of 2026-11-04: focus-based visible-tab rule in FR-023a and the data model (I1), "Approved" in FR-025 (I2), US1 scenario 5 row fields (I3), REST in the plan's constitution check (C1), team conventions in Assumptions (U1), Phase 9b in the dependency list (F1), plus wording fixes (T1, F2, F3, F4, D1, F5, F6), in spec.md, plan.md, data-model.md and tasks.md
- [X] T118 Resolve the second /speckit-analyze run (2026-11-04): focus-based `ui://visibility` in contracts/tauri-ipc.md (I4), marker clearing on refocus in data-model.md (I5), "visible, focused window" wording in FR-022c, FR-022f and plan.md (I6), "open and focused" in quickstart V8 (I7)
- [ ] T107 Run quickstart V19–V27 on the dev build and on a rebuilt .app

---

## Phase 9c: Weekly done-ticket counts (2026-11-05)

**Goal**: Group a work-item tab by month and then by week, with counts per month and week, so the team sees how many tickets it closed each week (FR-038).

**Independent Test**: quickstart.md V28.

- [X] T119 Add `closedAt` to the two work-item queries, `ListItem.closed_at` (parsed for work items, `None` for MRs) and `Tab.group_by_week` (serde default false; kept only on work-item tabs in `apply_tab_edits`; not part of the fingerprint); tests for parsing `closedAt` and for the tab rule (FR-038), in src-tauri/src/gitlab/queries/work_items_*.graphql, src-tauri/src/model/item.rs, src-tauri/src/model/tab.rs, src-tauri/src/gitlab/parse.rs, src-tauri/src/app_state.rs, src-tauri/tests/common/mod.rs, src-tauri/tests/parse_test.rs and src-tauri/tests/tabs_test.rs
- [X] T120 [P] Add the pure `groupByMonthWeek(items)` (closed date else updated date, local time, Monday weeks split at month ends, newest first, URL order kept inside a week, labels and counts) with unit tests for month-boundary weeks, Sunday/Monday edges and the updated-date fallback (FR-038), in src/util/groups.ts and src/util/__tests__/groups.test.ts
- [X] T121 Add `closedAt` and `groupByWeek` to the TS types, and render grouped tabs in TabList with month and week headings and counts, clickable to collapse or expand (FR-038), in src/types.ts and src/components/TabList.vue
- [X] T122 Add a "Group by month and week" checkbox to work-item tabs in the tab editor, sent as `groupByWeek`; ticking it also turns off the tab's three notify switches (FR-038), in src/components/settings/TabEditor.vue
- [X] T123 Sync the design docs: `Tab.group_by_week` and `ListItem.closed_at` in data-model.md, `Tab.groupByWeek` in contracts/tauri-ipc.md, `closedAt` in contracts/gitlab-graphql.md, and a new check V28 in quickstart.md (a "Done" tab with my closed-tickets URL, `state=closed&assignee_username[]=DusanJ88&sort=closed_at_desc`)
- [X] T125 Add the fifth default tab "Done" (`https://gitlab.com/groups/login5/-/work_items?state=closed&assignee_username[]=DusanJ88&sort=closed_at_desc`, `group_by_week: true`, all notify switches off) to `default_tabs`, update the "Replace all tabs with the defaults?" text and the tests that count or index default tabs; test that it parses as a closed work-item list sorted by closed date (FR-015, FR-038), in src-tauri/src/model/tab.rs, src/components/settings/TabEditor.vue, src-tauri/tests/store_test.rs and src-tauri/tests/url_map_test.rs
- [X] T126 Add "On resource-test" as the second default tab (My MRs URL, `test_branch: resource-test`, notifications on), update the reset text to 6 and the tests that count or index default tabs (FR-015, FR-034), in src-tauri/src/model/tab.rs, src/components/settings/TabEditor.vue, src-tauri/tests/store_test.rs, src-tauri/tests/url_map_test.rs and src-tauri/tests/tabs_test.rs
- [X] T127 [P] Add `periodKeys(date)` (month and week keys for a date, shared by the grouping) and `monthsBack(monthKey, n)` to the grouping util, with tests for today's keys and the 12-month window across a year boundary (FR-038), in src/util/groups.ts and src/util/__tests__/groups.test.ts
- [X] T128 In TabList, start every month and week collapsed except the current ones (expanded/collapsed toggles kept per tab until restart), show only the last 12 months with a "Show more (N tickets)" button adding 12 months per click, and reveal a highlighted row's month and week (FR-038), in src/components/TabList.vue
- [X] T129 Update quickstart V28 for the collapsed-by-default headings and the 12-month window with "Show more", in quickstart.md
- [ ] T124 Run quickstart V28 on the dev build (and V5 for the Done and On resource-test tabs after "Reset to defaults")

---

## Phase 9d: What's deployed on which env (2026-11-24)

**Goal**: Track GitLab deployment environments (default qa-1) instead of the manual env sheet: an Envs tab, env chips on MR and ticket rows, and a notification per new deployment (FR-040).

**Independent Test**: Settings → Environments lists `login5/login-eko/login-eko qa-1`; the Envs tab shows qa-1's deployed ref, commit, deployer and time, and the MRs on it; those MRs show a "qa-1" chip; adding `qa-5` shows its `14337-…` branch and chips #14337 when it's in a tab.

- [X] T130 Add `TrackedEnv`, `EnvStatus`, `EnvDeployment`, `EnvBadge`, `normalize_envs` and `default_tracked_envs`; `Settings.tracked_envs`, `PersistedState.envs`, `ListItem.envs`, `NotificationKind::Deploy` and the view fields (FR-040), in src-tauri/src/model/env.rs, src-tauri/src/model/settings.rs, src-tauri/src/model/state.rs, src-tauri/src/model/item.rs, src-tauri/src/model/notification.rs, src-tauri/src/model/view.rs and src-tauri/src/app_state.rs
- [X] T131 Fetch each env's state and latest successful deployment over REST, and set env chips on every row: deployed-branch match, else the FR-034 ancestry check against the deployed commit (`check` now reads history from the commit, results cached per label), tickets via their MRs and via `<iid>-…` deployed branches (FR-040), in src-tauri/src/gitlab/envs.rs, src-tauri/src/gitlab/branch_check.rs and src-tauri/tests/envs_test.rs
- [X] T132 Fetch envs alongside the tabs in each poll, flag rows across all tabs, merge into `state.envs` (keep last deployment on error, drop untracked) and notify on a new deployment id after a silent baseline; add `set_tracked_envs` (FR-040), in src-tauri/src/engine/env_watch.rs, src-tauri/src/engine/poller.rs, src-tauri/src/commands.rs and src-tauri/src/lib.rs
- [X] T133 Add the Envs tab (env cards with ref, commit → deploy job, deployer, time, rows on it → jump to row), env chips on MR and work-item rows, and the Settings "Environments" editor (FR-040), in src/types.ts, src/ipc.ts, src/store.ts, src/App.vue, src/components/TabBar.vue, src/components/EnvList.vue, src/components/EnvChips.vue, src/components/MrRow.vue, src/components/WorkItemRow.vue, src/components/settings/EnvEditor.vue and src/components/settings/SettingsView.vue
- [X] T134 Print tracked envs (plus `EXTRA_ENVS`) and the rows on them in the live check; checked against gitlab.com: qa-1 = `resource-test`, 13 of the user's MRs on it (2 outdated) (FR-040), in src-tauri/examples/live_check.rs
- [ ] T135 Run the Independent Test above on the dev build, including a deploy notification
- [X] T136 Add "Not approved" and "Not deployed" filter checkboxes to the list toolbar, per tab until restart, with "N of M" and an empty-filter message; pure `filterItems` with tests (FR-041), in src/util/filter.ts, src/util/__tests__/filter.test.ts and src/components/TabList.vue

---

## Phase 10: Polish & Cross-Cutting Concerns

- [X] T097 [P] Write README.md (what it is, prerequisites, token scope, dev and build commands, allowing notifications, Gatekeeper "Open anyway", where data lives, VERIFY results link)
- [X] T098 [P] Check the 500-row list for smooth rendering and lazy-load avatars (`loading="lazy"`), in src/components/TabList.vue
- [X] T099 Security pass: confirm the token never appears in logs, events, files or error messages (grep `token` in src-tauri/src), and that `open_url` and capabilities only allow the configured host, in src-tauri/src/commands.rs and src-tauri/capabilities/default.json
- [X] T100 Run `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`, `pnpm typecheck` and `pnpm test`, and fix every finding (repo-wide)
- [X] T101 Build the universal .app and .dmg with `pnpm tauri build --bundles app,dmg --target universal-apple-darwin`, and confirm the outputs in src-tauri/target/universal-apple-darwin/release/bundle/
- [ ] T102 Run the manual checks V1–V18, plus a memory check (Activity Monitor, 8 tabs × 200 items, must stay under 150 MB, SC-009), in specs/001-gitlab-tracker-app/quickstart.md on the built app and record pass/fail in specs/001-gitlab-tracker-app/checklists/validation.md

---

## Dependencies & Execution Order

### Phase dependencies

- **Setup (Phase 1)**: T001 → T002 (needs network) → T003 → T004–T011.
- **Foundational (Phase 2)**: Depends on Setup. Blocks all stories.
- **US1 (Phase 3)**: Depends on Foundational. This is the MVP.
- **US2 (Phase 4)**: Depends on US1 (needs real lists to diff). Adds the Notifier and bell used by every later story.
- **US3, US4, US5 (Phases 5–7)**: Each depends on US2 (Notifier, log, visible-tab rule). They're independent of each other and can run in parallel, but all three edit `engine/diff.rs`, `notify_text.rs` and `poller.rs`, so merge them one at a time.
- **US6 (Phase 8)**: Depends on Foundational + US1 (`parse_tab_url`). Can run in parallel with US2–US5.
- **US7 (Phase 9)**: Depends on US1. Can run in parallel with US2+.
- **Phase 9b (T103–T116)**: Fixes and features from the first manual run (2026-11-04). Depends on US1, US2 (visible-tab rule, notifications), US6 (tab editor) and US7 (warning bars). Within it: T110 → T112 → T114 → T115 (MR status → ticket → ticket fallback → layout); T111 → T113 (test-branch tab → global flag); T116 after T103–T115; T107 last.
- **Phase 9c (T119–T129)**: Weekly done-ticket counts (2026-11-05). Depends on US1 (TabList) and US6 (tab editor). T119 → T121 → T122; T120 can run in parallel with T119; T125 after T119; T126 after T125; T127 → T128 → T129; T123 after T119–T122; T124 last.
- **Polish (Phase 10)**: After all the chosen stories.

### Story completion order

```text
Setup → Foundational → US1 ─┬─► US2 ─┬─► US3
                            │        ├─► US4
                            │        └─► US5
                            ├─► US6
                            └─► US7
                                         └──► Polish
```

### Within each story

Tests are written first and must fail, then come query/parse, the pure engine logic, poller integration and the UI.

## Parallel Examples

**Phase 2**: T012, T013, T014, T015 (models), T019 (secrets), T022 (avatar scheme), T023 (config), T030, T031 (TS types/IPC), T033, T034 (styles/icons) all touch different files.

**US1**: T036–T039 (tests) together; then T041 + T042 (queries) and T047–T050 (UI atoms) in parallel; then T043 → T044 → T045, and T051 → T054.

**US2**: T057–T059 (tests) together; T061, T067 and T068 in parallel with T060.

**US3 / US4 / US5**: Different developers or sessions can take one story each after US2. T079 and T080 in US4 can be done in parallel.

## Implementation Strategy

### MVP first

1. Phase 1 (with network and Rust installed, and the VERIFY items checked).
2. Phase 2.
3. Phase 3 (US1). **Stop and check** with quickstart V1–V6. That's a usable read-only menu bar viewer.

### Incremental delivery

4. US2: notifications for new items, counts, bell (V7, V8, V13, V15).
5. US6: own tabs (V16), so it's useful beyond the defaults.
6. US3 → US4 → US5: comments, changes, mentions (V9–V12, V14).
7. US7: error polish (V17, V18).
8. Polish: build the .dmg and do the full V1–V18 run.

## Notes

- **Definition of done (constitution, Development Workflow)**: a task may be checked off only when `cargo test`, `cargo clippy -- -D warnings`, `pnpm typecheck` and `pnpm test` all pass. T100 is the final full run, not the only one.

- [P] = different files and no dependency on unfinished tasks.
- Any task that changes a GraphQL query must update its fixture and parse test.
- Commits are authored by the user only, with no tool attribution (constitution).
