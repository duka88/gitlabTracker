# Data Model: GitLab Tracker Menu Bar App

**Feature**: [spec.md](./spec.md) | **Research**: [research.md](./research.md)

All types live in Rust (`src-tauri/src/model/`) and derive `Serialize`/`Deserialize`. The ones sent to the UI are mirrored in TypeScript (`src/types.ts`). Field names are `snake_case` in Rust, and `#[serde(rename_all = "camelCase")]` makes them `camelCase` in JSON.

---

## Settings (persisted: `settings.json`)

| Field | Type | Rules |
|---|---|---|
| `interval_minutes` | `u8` | One of 1, 2, 5, 10. Default 2 (FR-008). |
| `notify_mentions` | `bool` | Default `true` (FR-022e). |
| `user` | `Option<UserInfo>` | Set after a successful token test. Cleared when the token is removed. |
| `tabs` | `Vec<Tab>` | Ordered. On first start, the 6 defaults (FR-015, research R14). |
| `schema_version` | `u32` | For migrations. Starts at 1. |

The host is fixed to `https://gitlab.com` in v1 (constant `GITLAB_HOST`, research R17). The token is **not** stored here. It's in the Keychain, under service `com.duka88.gitlab-tracker` and account `gitlab-token:gitlab.com`.

Added 2026-11-04: `test_branch: Option<String>`, the branch flagged on every MR (FR-037). Trimmed; blank or with whitespace → `None` (`normalize_branch`).

### UserInfo

`id: String` (GraphQL gid), `username`, `name`, `avatar_url`.

### Tab

| Field | Type | Rules |
|---|---|---|
| `id` | `String` | UUID v4, stable across renames and reorders. |
| `name` | `String` | 1–40 characters, trimmed. Doesn't have to be unique. |
| `url` | `String` | Must parse into a `TabQuery` (see contracts/url-mapping.md). |
| `kind` | `ListKind` | `MergeRequests` or `WorkItems`. Derived from the URL and kept so the UI can show it. |
| `notify_new_items` | `bool` | |
| `notify_new_comments` | `bool` | Allowed on every tab type (clarification, round 2). |
| `notify_changes` | `bool` | |
| `test_branch` | `Option<String>` | MR tabs only: show just the MRs merged into this branch (FR-034). Trimmed; no whitespace; dropped on work-item tabs. |
| `group_by_week` | `bool` (default `false`) | Work-item tabs only: the UI groups rows by month and week with counts (FR-038). Dropped on MR tabs. Not part of the fingerprint, so toggling it doesn't reset the baseline. |
| `sort` | `TabSort` enum: `gitlab` \| `label` \| `updated` \| `created` \| `title` \| `priority` \| `stage` (default `gitlab`) | The tab's default row order, applied in the UI (FR-039). Doesn't reset the baseline. |

Changing `url` or `test_branch` resets that tab's `TabState` (no notifications on the next load, FR-019). The `TabState` fingerprint is taken over `url` plus `\ntest_branch=<b>` when a test branch is set (`Tab::fingerprint_source`).

## TabQuery (derived, not persisted)

`kind`, `scope: Scope` (`Dashboard{ relation: Author|Reviewer|Assignee, username }` / `Group{ path }` / `Project{ path }`), `filters: Filters`, `ignored_params: Vec<String>`.

`Filters`: `state`, `author`, `assignees[]`, `reviewer`, `labels[]`, `not_labels[]`, `milestone`, `not_milestone`, `status`, `types[]`, `draft: Option<bool>`, `search`, `sort`.

## ListItem (sent to UI; part of `TabState.items`)

Common fields:
- `gid` (global id, the de-duplication key across tabs)
- `kind`, `iid`, `reference` (`!123` / `#45`)
- `title`, `web_url`, `project_path`
- `author: Person`, `created_at`, `updated_at`
- `closed_at: Option<DateTime<Utc>>` (work items: GitLab `closedAt`; `None` for MRs; `#[serde(default)]` so older `state.json` loads). Groups rows in FR-038 tabs, falling back to `updated_at`.
- `labels: Vec<Label>`, `assignees: Vec<Person>`, `milestone: Option<String>`
- `comment_count: Option<u32>`, `upvotes`, `downvotes`
- `description_preview: String` (≤300 characters of plain text)

`Mr` only:
- `draft`, `target_branch`, `state`, `pipeline: Option<PipelineStatus>`
- `approvals: { approved: bool, required: u32, given: u32 }`: `given` counts `approvedBy`; `approved` also needs `given >= max(required, 1)` (FR-025a)
- `unresolved_threads: u32` (resolvable − resolved threads, FR-025b)
- `source_branch`, `head_sha` (`diffHeadSha`)
- `on_branch: Option<{ branch, up_to_date: bool, merged_at: Option<DateTime> }>`: set in tabs with a test branch, or on every MR when the global test branch is set (FR-034, FR-037, research R18)
- `ticket: Option<LinkedTicket>`: `{ reference, title, web_url, open, state_label: Option<Label>, status: Option<WorkStatus> }` (FR-036, research R19)
- `reviewers: Vec<Reviewer>` (`Person` + `approved: bool` + `review_state`)
- `merge_status: Option<MergeStatusTag>`

`WorkItem` only:
- `type_name`, `type_icon`, `confidential`, `iteration: Option<String>`
- `due_date: Option<NaiveDate>`, `weight: Option<u32>`
- `status: Option<{name, color}>`, `health: Option<OnTrack|NeedsAttention|AtRisk>`
- `children: Option<{done: u32, total: u32}>`

`Label`: `title`, `color`, `text_color`, and derived `scope: Option<String>` plus `value` (split on the **last** `::`, as GitLab does).
`Person`: `username`, `name`, `avatar_url`. This is always an `avatar://localhost/<percent-encoded absolute GitLab URL>` (relative GitLab paths are made absolute first), so the webview never contacts GitLab (research R16).
`MergeStatusTag`: `ReadyToMerge | Approved | NeedsApproval | CiFailed | Conflicts | ChangesRequested | UnresolvedThreads` (derivation table in research R5).

## TabState (persisted: `state.json`, keyed by tab id)

| Field | Type | Meaning |
|---|---|---|
| `url_fingerprint` | `String` | Hash of the tab URL. A mismatch means a reset. |
| `baseline_done` | `bool` | False until the first successful load (FR-019). |
| `baseline_at` | `DateTime` | Notes and to-dos older than this never notify. |
| `items` | `Vec<ListItem>` | Last result, for display after a restart and while offline. |
| `tracked` | `HashMap<gid, Tracked>` | `comment_count`, `TrackedState` (below). |
| `new_items` | `HashSet<gid>` | Unseen NEW (FR-020). |
| `new_comments` | `HashMap<gid, u32>` | Unseen new-comment count per item, shown as "(+2)" (FR-029a). |
| `changes` | `HashMap<gid, Vec<ChangeKind>>` | Unseen changes for the "changed" tag (FR-022d). |
| `last_updated` | `Option<DateTime>` | Last successful check. |
| `error` / `warning` | `Option<TabIssue>` | `{kind, message}`, for the red/orange bars (FR-031). |
| `truncated_total` | `Option<u32>` | Set when the 500-item limit is hit. |

`TrackedState`:
- MR: `pipeline`, `approved`, `changes_requested_by: BTreeSet<String>`, `state`
- WorkItem: `status: Option<String>`, `labels: BTreeSet<String>`

## Global state (persisted: `state.json`, top level)

| Field | Type | Meaning |
|---|---|---|
| `seen_comment_ids` | `BoundedSet<String>` (cap 5,000, FIFO) | FR-022 |
| `seen_mention_ids` | `BoundedSet<String>` (cap 2,000) | FR-022f |
| `seen_while_visible` | `HashSet<gid>` | Items whose markers were created while their tab was visible in the open window (FR-023a). They're treated as seen everywhere: left out of **every** tab badge and the menu count, even after the window hides. A gid is removed once no tab has a marker for it any more. Persisted with the markers. |
| `mentions_baseline_at` | `Option<DateTime>` | The first mention poll records this and sends nothing. |

Per-poll only (in memory, dropped after each poll): `poll_notified: HashSet<String>` with keys `new:<gid>` and `change:<gid>:<kind>`, so an item in several tabs notifies once per poll. Not persisted, so an item that leaves and comes back later, or a pipeline that fails again later, notifies again (spec edge cases).

## NotificationEntry (persisted: `notifications.json`, newest 100)

`id` (UUID), `kind` (`NewItem | NewItems | Comment | Change | Mention`), `tab_id: Option<String>`, `item_gid: Option<String>`, `web_url`, `text`, `created_at`, `read: bool`.

`NewItems` (the "3 new in My tickets" summary) has `tab_id` but no `item_gid`. Clicking it switches to the tab.

## UI view model (event payload `state://updated`)

`AppView { signed_in, user, interval_minutes, notify_mentions, polling: bool, last_poll_at, active_tab_id, tabs: Vec<TabView>, menu_count, bell_unread, global_error }`
`TabView { tab: Tab, items: Vec<ListItem>, item_count, new_count, new_items, new_comments, changes, error, warning, truncated_total }`

---

## State transitions

### Tab lifecycle

```text
          url set/changed
 (none) ───────────────► Unloaded ──first ok load──► Baseline (no notify, no markers)
                            ▲                              │ next ok load
                            │ url changed                  ▼
                            └───────────────────────── Tracking ──error──► Tracking+error (keep last items)
                                                           ▲                     │ ok load
                                                           └─────────────────────┘
```

### Poll algorithm (one pass; the in-flight guard prevents overlap)

1. If there's no token, stop and emit `global_error = NoToken`.
2. **Mentions** (if `notify_mentions`): fetch to-dos. If there's no baseline yet, record ids and the baseline time and go on. Otherwise, for each new to-do, add `note.id` to `seen_comment_ids` (if present) and create a `Mention` notification, unless the item is in the visible tab (FR-022f).
3. **Tabs** (concurrently): fetch all pages and save the result. On error, set `error`, keep the old items and continue.
4. For each tab with `baseline_done == false`: store `items` and `tracked`, set `baseline_done` and `baseline_at`, and skip to the next tab.
5. **Diff** against the previous `tracked`. Order matters for the visible-tab rule: when the window is visible and focused (FR-002d), the active tab is compared **first**, then the other tabs in tab order (the fetches in step 3 run concurrently, the comparison doesn't):
   - `added = current − previous` → `new_items`, plus a `NewItem`/`NewItems` notification (if `notify_new_items` and `new:<gid>` isn't in `poll_notified`).
   - `comment_count` went up (not for added items) → queue the item for comment fetch (if `notify_new_comments`).
   - `TrackedState` differs (not for added items) → `ChangeKind`s (R8) → `changes`, plus a `Change` notification per kind (if `notify_changes`, key `change:<gid>:<kind>` not in `poll_notified`).
   - `removed` MR ids (open-MR tabs only, `notify_changes`) → queue for the merged check.
6. **Comment fetch** (batched): new notes not in `seen_comment_ids`, not `system`, not by the signed-in user, created after `baseline_at` → `new_comments[gid] += 1`, plus one `Comment` notification per note. Add every fetched note id to `seen_comment_ids`.
7. **Merged check** (batched): `state == merged` and `mergeUser != me` → `Change(Merged)` notification.
8. **Visible-tab rule** (FR-023a, applied during steps 5–7): when the window is visible and focused (FR-002d), markers for `active_tab_id` are still set and their gids are added to the global `seen_while_visible`, but no notifications or bell entries are created. Their keys (`new:<gid>`, `change:<gid>:<kind>`) go into `poll_notified` and their note ids into `seen_comment_ids`, so a hidden tab holding the same item can't notify for it in this check (that's why the active tab is compared first). From the next check on, the item is in every tab's previous result, so it can't count as new again.
9. Recompute counts. Tab badge = gids with markers in that tab, minus the global `seen_while_visible`. `menu_count` = distinct gids across all tab badges (FR-004). Mentions never add to either count (FR-022g). Update the tray title, persist, and emit `state://updated`.

### Marker clearing

- `select_tab(id)` (and the window being shown or regaining focus with that tab active, FR-002d) → clear `new_items`, `new_comments` and `changes` for that tab, remove from `seen_while_visible` every gid that no tab has a marker for any more, then recompute `menu_count` (FR-023).
- `open_bell()` → mark all `NotificationEntry.read = true`, so `bell_unread` becomes 0 (FR-024c). Tab markers aren't touched.

## Validation rules (summary)

| Rule | Source |
|---|---|
| The URL host must be `gitlab.com`. Path must match a known list page. Dashboard work-item URLs are invalid. | FR-011, edge cases |
| A sort the app can't map → `UPDATED_DESC` plus a tab warning. | FR-031, edge case |
| Unknown query params go to `ignored_params` (shown in the hint) and aren't an error. | Edge case |
| A dashboard MR URL needs exactly one of author / reviewer / assignee. | R2 |
| The interval must be 1, 2, 5 or 10. | FR-008 |
| The token is saved only after `currentUser` returns non-null. | FR-006 |
| A tab name is 1–40 characters. | FR-014 |
