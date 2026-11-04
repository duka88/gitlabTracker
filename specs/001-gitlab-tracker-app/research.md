# Research: GitLab Tracker Menu Bar App

**Feature**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md) | **Date**: 2026-11-03

> **Network note**: GitLab, crates.io and npm couldn't be reached from the planning machine. Everything below comes from documented API behaviour as known at planning time. Items marked **VERIFY** must be checked in GitLab's GraphQL explorer (`https://gitlab.com/-/graphql-explorer`) or against the crate docs at the start of implementation. Each one has a fallback, so none of them block the design.

---

## R1. Menu bar shell (Tauri 2)

- **Decision**:
  - Tray icon via `tauri::tray::TrayIconBuilder` (`tray-icon` feature) with a template (monochrome) icon.
  - Badge number via `TrayIcon::set_title(Some("3"))`; macOS draws the title text next to the icon. `None` when the count is 0.
  - `show_menu_on_left_click(false)`, so a left click toggles the window and a right click shows the menu (Open / Refresh now / Quit).
  - Window placed under the icon with `tauri-plugin-positioner` (`Position::TrayBottomCenter`), after feeding it tray events with `on_tray_event`.
  - Window settings: `decorations: false`, `visible: false`, `alwaysOnTop: true`, `skipTaskbar: true`, `resizable: false`, size 480×640.
  - ~~Hide on outside click by handling `WindowEvent::Focused(false)` and calling `hide()`.~~ Dropped 2026-11-04 (FR-002d): the window stays open as a normal window; focus changes only drive the "seen" state.
  - No Dock icon: `app.set_activation_policy(tauri::ActivationPolicy::Accessory)` in `setup`, plus `LSUIElement = true` in `Info.plist` so the bundled `.app` never shows a Dock icon.
- **Rationale**: These are first-party Tauri 2 APIs and the official positioner plugin, with no custom Objective-C.
- **Alternatives considered**:
  - `NSPopover` through objc bindings: native look, but a lot of unsafe code. Rejected.
  - `tauri-nspanel`: lets the window float over full-screen apps, but it's a third-party crate. Kept as a later option.
- **VERIFY**: A left click must not also open the menu, and `Focused(false)` must fire when you click the menu bar or the desktop. Fallback for focus: also hide when the app is deactivated (another app becomes active).

## R2. GitLab API: GraphQL for lists

- **Decision**: One GraphQL query per tab per page (`first: 50`, following `pageInfo.endCursor` until done or 500 items). The query root depends on the URL scope (see [contracts/url-mapping.md](./contracts/url-mapping.md)):

| URL scope | MR root | Work-item root |
|---|---|---|
| Dashboard `/dashboard/merge_requests` | `user(username:){ authoredMergeRequests / reviewRequestedMergeRequests / assignedMergeRequests }` chosen by which of `author_username` / `reviewer_username` / `assignee_username` is set | n/a: dashboard work items are mapped to `/dashboard/issues` → not supported in v1, shown as invalid with a hint to use a group URL |
| Group `/groups/<path>/-/merge_requests`, `/<group>/-/work_items` | `group(fullPath:){ mergeRequests(includeSubgroups: true, …) }` | `group(fullPath:){ workItems(includeDescendants: true, …) }` |
| Project `/<ns>/<proj>/-/merge_requests`, `/-/work_items`, `/-/issues` | `project(fullPath:){ mergeRequests(…) }` | `project(fullPath:){ workItems(…) }` |

- **Page size**: 50, not 100, to stay under gitlab.com's query complexity limit (about 250 for an authenticated user) with the nested labels/assignees/reviewers connections. Nested connections are capped (`labels(first: 20)`, `assignees(first: 10)`, `reviewers(first: 10)`).
- **Rationale**: The REST MR list lacks pipeline status and approvals (spec FR-016), and GraphQL work items carry status, iteration, health and hierarchy in widgets. One API style for both tab types.
- **Alternatives considered**: REST lists plus a REST call per MR for approvals/pipelines. That's N+1 requests every 2 minutes. Rejected.
- **VERIFY**:
  1. `group.workItems` with `includeDescendants` exists on gitlab.com (introduced in the 17.x line). Fallback: the URL's project path, or `group.issues` (legacy) for counts.
  2. MR `sort` enum values (`CREATED_DESC`, `UPDATED_DESC`, `MERGED_AT_DESC`, …) for each URL `sort=` value. Fallback: `UPDATED_DESC`.
  3. The complexity score at `first: 50`. If it's too high, drop to `first: 25`.

## R3. Work-item status filter ("To do")

- **Decision**: Send `status: { name: "To do" }` (input type `WorkItemWidgetStatusFilterInput`) on `workItems`. The URL parameter is `status=To%20do` (GitLab's newer list UI may use the status id: both `status_name` and `status_id` style values are accepted and passed through as `name` or `id`). If the response has a GraphQL error that mentions `status`, or an argument/schema error, retry once without it and set the tab warning "Status filter not supported — showing all statuses" (FR-017).
- **Rationale**: Status is a Premium widget added in the 18.x line. The retry makes the app work on any tier.
- **VERIFY**: The exact argument name and input shape in the explorer. Candidates in order: `status: {name:}`, `statusWidget: {status: {name:}}`. The translator keeps this in one function, so a change touches a single place.

## R4. Exclude ("not") filters

- **Decision**: Both MR and work-item queries take `not: { labelName: [...], milestoneTitle: [...], authorUsername, assigneeUsernames }`. MR `not` uses `labels` and `milestoneTitle`. URL params `not[label_name][]=X` and `not[milestone_title]=Backlog` map directly.
- **VERIFY**: The MR `not` input field names (`labels` vs `labelName`). The translator has a per-type field map.

## R5. Row fields → GraphQL fields

| Row field | MR field | Work-item field |
|---|---|---|
| ref | `iid` (+ `reference` for `!123`) | `iid`, `reference(full: false)` |
| title / draft | `title`, `draft` | `title`, `confidential` |
| type icon | — | `workItemType { name iconName }` |
| author / created / updated | `author{name username avatarUrl webUrl}`, `createdAt`, `updatedAt` | same |
| target branch | `targetBranch` | — |
| milestone | `milestone{title}` | `widgets ... on WorkItemWidgetMilestone { milestone{title} }` |
| iteration | — | `WorkItemWidgetIteration { iteration{title startDate dueDate} }` |
| labels | `labels{nodes{title color textColor}}` | `WorkItemWidgetLabels { labels{nodes{…}} }` |
| pipeline | `headPipeline{status}` | — |
| approvals | `approved`, `approvalsRequired`, `approvalsLeft`, `approvedBy{nodes{username}}` | — |
| reviewers + approval | `reviewers{nodes{username name avatarUrl mergeRequestInteraction{approved reviewState}}}` | — |
| assignees | `assignees{nodes{…}}` | `WorkItemWidgetAssignees { assignees{nodes{…}} }` |
| merge status tag | `detailedMergeStatus`, `conflicts`, `draft`, `state`, plus approvals and unresolved threads | — |
| unresolved threads | `resolvableDiscussionsCount − resolvedDiscussionsCount` (added 2026-11-04, FR-025b) | — |
| comments | `userNotesCount` | `userNotesCount`? (see **VERIFY**) |
| 👍/👎 | `upvotes`, `downvotes` | `WorkItemWidgetAwardEmoji { upvotes downvotes }` |
| due date | — | `WorkItemWidgetStartAndDueDate { dueDate }` |
| weight | — | `WorkItemWidgetWeight { weight }` |
| status | — | `WorkItemWidgetStatus { status{name color iconName} }` |
| health | — | `WorkItemWidgetHealthStatus { healthStatus }` |
| child progress | — | `WorkItemWidgetHierarchy { rolledUpCountsByType{…} }` or `children{count}` + closed count |
| description preview | `description` (trimmed to 300 chars of plain text in Rust) | `WorkItemWidgetDescription { description }` |
| web link | `webUrl` | `webUrl` |
| project | `project{fullPath}` | `namespace{fullPath}` |

- **VERIFY** (most important): Is there a user-notes count on `WorkItem`? Fallbacks, in order:
  - (a) `WorkItemWidgetNotes { discussions(filter: ONLY_COMMENTS) { count } }` if a count is exposed;
  - (b) a second query `group.issues(iids: [...]) { iid userNotesCount }` per tab (one extra request, same filters by iid);
  - (c) detect comments by an `updatedAt` change and fetch the last notes. (c) shows no count on the row.

**Merge status tag derivation** (pure function, unit-tested):

| Condition (first match wins) | Tag |
|---|---|
| `state != opened` or `draft` | none |
| `conflicts` or `detailedMergeStatus == CONFLICT` | Conflicts |
| `headPipeline.status == FAILED` or `detailedMergeStatus == CI_MUST_PASS` with a failed pipeline | CI failed |
| any reviewer `reviewState == REQUESTED_CHANGES` or `detailedMergeStatus == REQUESTED_CHANGES` | Changes requested |
| `detailedMergeStatus == DISCUSSIONS_NOT_RESOLVED` or unresolved threads > 0 | Unresolved threads |
| not approved for real (`approved == false`, or fewer approvers in `approvedBy` than `max(approvalsRequired, 1)`) / `detailedMergeStatus == NOT_APPROVED` | Needs approval |
| `detailedMergeStatus == MERGEABLE` and the linked ticket has a `state::` label other than "Ready to Merge" (FR-036) | Approved |
| `detailedMergeStatus == MERGEABLE` | Ready to merge |
| otherwise (e.g. `CI_STILL_RUNNING`, `CHECKING`) | none (the pipeline icon shows running) |

*2026-11-04 (FR-025a/b):* the original implementation used `approvalsRequired − approvalsLeft` as the approval count and trusted `approved`. With no approval rules GitLab reports `approved: true` and 0 required, so unapproved MRs showed "Ready to merge" (seen live on gitlab.com: `approved: true`, `approvalsRequired: 0`, `approvedBy` empty). The count now comes from `approvedBy` (falling back to required − left when the list is missing), and `approved` is only trusted together with it. Unresolved threads now block "Ready to merge" even when the project doesn't require resolving them.

## R6. New comments

- **Decision**: The comment count from the list query is the trigger. When an item's count goes above the stored value, fetch its latest notes:
  - MR: `project(fullPath){ mergeRequest(iid){ notes(last: 20){ nodes{ id body system author{username name} createdAt } } } }`.
  - Work item: `workItem(id){ widgets{ ... on WorkItemWidgetNotes { discussions(last: 20, filter: ONLY_COMMENTS){ nodes{ notes{ nodes{ id body system author{…} createdAt } } } } } } }`.
  - Skip notes that are `system`, written by the signed-in user, already in `seen_comment_ids`, or created before the item's baseline time. Batch up to 10 items per GraphQL request using aliases.
- **Rationale**: Comments are fetched only when something changed, which keeps polls cheap. Note ids are globally unique, so they de-duplicate across tabs.
- **Alternatives**: The REST `/notes` endpoints. That needs project ids and gives no benefit over GraphQL, so GraphQL is kept everywhere for consistency (constitution: REST only where GraphQL lacks data).

## R7. Mentions

- **Decision**: GitLab's To-Do list, via `currentUser{ todos(action: [mentioned, directly_addressed], state: [pending, done], first: 50) { nodes{ id action body createdAt author{username name} targetUrl target{ ... on MergeRequest {iid reference webUrl} ... on Issue {iid reference webUrl} ... on WorkItem {iid reference webUrl} } note{ id } } } }`, sorted newest first. A to-do whose `id` isn't in `seen_mention_ids` and whose `createdAt` is after the mention baseline is new.
  - Including `done` means mentions you already handled on the web still notify once.
  - When `note.id` is present, it's added to `seen_comment_ids` before comment processing, so the same comment never notifies twice (FR-022f).
- **Rationale**: To-dos are GitLab's own mention feed and cover items outside all tabs with one cheap query.
- **VERIFY**: The `todos(action:)` enum names (`mentioned`, `directly_addressed`) and the `note` field on `Todo`. Fallback: filter by `action` client-side.
- **Ordering per poll**: Mentions are fetched before tab comment processing, so the mention wins in de-duplication.

## R8. Change detection

- **Decision**: Per item, store a `TrackedState`:
  - MR: `pipeline`, `approved`, `changes_requested_by` (set), `state`.
  - Work item: `status`, `labels` (set).

  Compare with the previous poll (see [data-model.md](./data-model.md#change-detection)). Merged detection: for MR ids that dropped out of a tab, query `project(fullPath){ mergeRequest(iid){ state mergedAt mergeUser{username} } }` (aliased, batched).
  - Self-made changes: GraphQL doesn't expose who changed a status or label cheaply, so these do notify (allowed by FR-022c's "when it doesn't").
  - Exception: "merged" isn't sent when `mergeUser` is the signed-in user, and "changes requested" isn't sent when the reviewer is the signed-in user.
- **De-duplication**: Across polls, a change notifies only when the previous `TrackedState` differs from the new one, so a pipeline that fails, recovers and fails again notifies again. Within one poll, an in-memory set keyed `<item-global-id>:<kind>` makes sure an item in two tabs notifies once. Nothing is stored for this between polls.

## R9. macOS notifications and click handling

- **Decision**: `tauri-plugin-notification` (desktop) to request permission and send.
  - **Click target**: the desktop plugin doesn't report which notification was clicked. Clicking any notification activates the app, so Rust listens for app activation within 10 s of the last sent notification and routes it like this:
    - one unread notification sent since the window was last shown → jump to its row;
    - otherwise → open the window with the bell popup (FR-024 fallback).
- **Alternatives considered**: `mac-notification-sys` with `wait_for_click(true)` on a thread per notification gives the exact clicked id, but uses the deprecated `NSUserNotification` API and blocks a thread per notification. It's kept as an upgrade path behind the same `Notifier` trait.
- **VERIFY**: With `ActivationPolicy::Accessory`, a notification click must raise `RunEvent::Reopen` or a focus event. If neither fires, the click only activates the app, and the bell popup shows the next time the window opens.
- **Dev builds**: Notifications need a bundled, signed or ad-hoc-signed `.app` (`tauri build` produces this). `tauri dev` may not show them.

## R10. Token storage

- **Decision**: The `keyring` crate (v3, `apple-native` feature), with service `com.duka88.gitlab-tracker` and account `gitlab-token:gitlab.com`. Rust reads the token into memory at start and never sends it to the webview. The UI gets `{signedIn, username, name, avatarUrl}` only.
- **Alternatives**: `tauri-plugin-stronghold`: heavier and not the system Keychain. Rejected.

## R11. HTTP and async

- **Decision**: `reqwest` (rustls, json) on Tauri's tokio runtime, with a 20 s timeout. One `GitLabClient { token, http }` (base URL `GITLAB_HOST`). Errors are classified as:
  - `401` → `TokenRejected`
  - `403` → `NoPermission`
  - GraphQL `null` root (`project`/`group` is null) → `NotFound`
  - network → `Unreachable`
  - GraphQL errors → `Query(String)`
- **Polling**: One tokio task with `tokio::time::interval(n)` and `MissedTickBehavior::Delay`. A `Notify` handles manual refresh, and an `AtomicBool` (in-flight) guard means no two polls run at once (FR-018).
- **Wake from sleep**: tokio's monotonic clock doesn't advance while the Mac sleeps, so the interval alone wouldn't poll on wake. A 30 s watchdog compares wall-clock time (`SystemTime`) with its previous reading. A jump of more than 60 s means the Mac was asleep, and it triggers one refresh through the same `Notify` (the in-flight guard prevents a burst). Alternative: `NSWorkspaceDidWakeNotification` via objc. Rejected as unsafe code for little gain.
- **Tabs per poll**: Fetched concurrently (`join_all`, max 4 at once).

## R12. Persistence

- **Decision**: One JSON file per concern in `app_data_dir()` (`~/Library/Application Support/com.duka88.gitlab-tracker/`):
  - `settings.json`: interval, mentions on/off, tabs.
  - `state.json`: per-tab baselines, seen sets, NEW markers.
  - `notifications.json`: last 100 entries.

  Writes are atomic (write to a temp file, then rename). Missing or corrupt files fall back to defaults with a warning.
- **Alternatives**: `tauri-plugin-store`: fine, but our own serde types are simpler to test. SQLite: overkill.

## R13. Frontend

- **Decision**:
  - Vue 3 `<script setup>` + TypeScript + Vite.
  - State lives in a single composable store (`useAppState`) that's fed by Tauri events (`state://updated`) and calls `invoke` commands. No Pinia or router: the views are a tab list and Settings.
  - Images: the webview never loads anything from GitLab (constitution I). Avatars use the `avatar://` scheme (R16), and the CSP is `default-src 'self'; img-src 'self' avatar: data:; style-src 'self' 'unsafe-inline'`.
  - Styling: plain CSS with variables that follow GitLab Pajamas (font stack, label chip, status colours), with light/dark from `prefers-color-scheme`.
  - Icons: a small inline SVG set (pipeline status, type, lock, comment, thumbs, bell, gear, refresh).
  - Tests: Vitest for formatting helpers (relative time, label scope split, contrast fallback).
- **Rationale**: The constitution says Vue only draws screens, and the UI is small.

## R14. Default tab URLs (gitlab.com, @DusanJ88, group `login5`)

| Tab | URL |
|---|---|
| My MRs | `https://gitlab.com/dashboard/merge_requests?author_username=DusanJ88&state=opened` |
| Reviewing | `https://gitlab.com/dashboard/merge_requests?reviewer_username=DusanJ88&state=opened` |
| My tickets | `https://gitlab.com/groups/login5/-/work_items?state=opened&assignee_username[]=DusanJ88&status=To%20do&not[milestone_title]=Backlog&not[label_name][]=state::Ready%20to%20Merge&not[label_name][]=state::Ready%20for%20Testing&not[label_name][]=state::In%20Testing` |
| Ready to merge | `https://gitlab.com/groups/login5/-/work_items?state=opened&assignee_username[]=DusanJ88&label_name[]=state::Ready%20to%20Merge&not[milestone_title]=Backlog` |

- **VERIFIED (2026-11-03)**: the labels are scoped: `state::Ready to Merge`, `state::Ready for Testing`, `state::In Testing`. The URLs above use the scoped names (see Verification results).

## R15. Build and distribution

- **Decision**:
  - `pnpm tauri build --bundles app,dmg` with an ad-hoc signature (`signingIdentity: "-"`), producing a universal binary (`--target universal-apple-darwin`).
  - Notarization is out of scope for a personal tool. Gatekeeper's "Open anyway" is documented in [quickstart.md](./quickstart.md).
- **Prerequisites**: Rust stable (rustup), Xcode Command Line Tools, Node 20+ and pnpm. Rust isn't installed on the dev machine yet, and the network was unavailable during planning.

## R16. Avatars without webview network access

- **Decision**: Register an async custom URI scheme `avatar` with `tauri::Builder::register_asynchronous_uri_scheme_protocol`. `ListItem` people carry `avatar_url = "avatar://localhost/<percent-encoded GitLab avatar URL>"`. The handler checks that the decoded URL is on `https://gitlab.com`, fetches it with the Rust HTTP client, and caches the bytes:
  - in memory (LRU, 300 entries);
  - on disk in `app_cache_dir()/avatars/<sha256>`, for 7 days.

  Any other source, including the Gravatar URLs GitLab returns for users without an uploaded avatar, is not fetched (constitution I: no third-party services); the UI draws an initials circle for those. On failure it also falls back to initials.
- **Rationale**: This keeps constitution I (all network calls in Rust), works for avatars that need authentication, and caches across polls and restarts.
- **Alternatives considered**:
  - Inline `data:` URLs in the view payload: makes every `state://updated` event large. Rejected.
  - Changing the constitution: not needed.

## R17. Host

- **Decision**: v1 supports only `https://gitlab.com` (spec Assumptions). The host is a constant (`GITLAB_HOST`) in `src-tauri/src/config.rs`, not a setting, which keeps the token form, capabilities and the URL check simple. Supporting self-hosted later means turning the constant into a setting.

## R18. Test-branch membership (FR-034, added 2026-11-04)

- **Context**: the user merges feature branches into a shared `resource-test` branch with ordinary merge commits ("Merge branch '<x>' of gitlab.com:… into resource-test", two parents), and wants to see which open MRs are on it.
- **Decision**: per MR, using the REST API (GraphQL has no ancestry query):
  1. `GET projects/:p/repository/branches/:test` → test head (404 → branch absent in that project).
  2. Up to date ⇔ MR `diffHeadSha` is an ancestor of the test head: `GET repository/merge_base?refs[]=<head>&refs[]=<test>` returns `<head>`.
  3. Otherwise, the newest first-parent merge on the test branch whose title contains `'<source>'` or `/<source>'` (`GET repository/commits?ref_name=<test>&first_parent=true&per_page=100`); if its second parent is an ancestor of the MR head → outdated. Else not on test.
  4. Results are cached by (project, MR head, test head), so an unchanged test branch costs one request per project per poll.
- **Rejected**: comparing `merge_base(test, head)` against `main`. A branch that merged `main` after being merged into the test branch has several merge bases with it, and GitLab returns just one, often a `main` commit, so `!18059` read as "not on test" although merged the day before. `merge_base(a, b) == a` is exact because an ancestor is the unique best common ancestor. Also rejected: the compare API (returns full diffs, heavy for a 600-commit branch) and set comparison via `ref_name=a..b` ranges (works, but needs many pages per busy branch).
- **Verified live (2026-11-04)** on `login5/login-eko/login-eko`: 10 of 12 open MRs on `resource-test`, `!18059` outdated (7 commits pushed after the merge), `!19469` and `!19183` not on test.

## R19. Linked ticket of an MR (FR-036, added 2026-11-04)

- **Field**: `MergeRequest.workItemRelations(first: 10) { nodes { linkType workItem { iid reference title webUrl state widgets { __typename …Status …Labels } } } }`. `closingIssuesReferences` doesn't exist on GitLab.com's `MergeRequest`; `linkedWorkItems` is deprecated in 19.2 in favour of `workItemRelations`.
- **Finding**: on `login5`, most MRs link their ticket only as `MENTIONED` (e.g. !19717 → #19368), a few as `CLOSES`, and some mention unrelated tickets too (!19469 mentions #18442, #13876 and #19450). Branch names start with the ticket number.
- **Decision**: `CLOSES` wins; otherwise the related item whose iid is the leading digits of `sourceBranch`. No guess beyond that.
- **Workflow state**: the team's state is the scoped label `state::…` (To do / In Development / Ready for Testing / In Testing / Ready to Merge). GitLab's status widget says "To do" on nearly all of them, so it's only a fallback.
- **Fallback (FR-036b)**: when the relations don't contain the branch-number ticket (none linked: !19667, !16339; or past the first 10: !18059 has 11 and #14796 is last), one `project.workItems(iids: [...])` query per project fetches the missing ones (`queries/tickets_by_iid.graphql`, `gitlab::tickets`). Only sent with a non-empty `iids` list: with the argument missing GitLab returns the project's latest 100 work items instead. A failed lookup leaves the MR without a ticket rather than failing the tab.
- **Cost**: the 50-MR page with these fields stays under GitLab's query complexity limit (verified on all three MR queries, 2026-11-04).

---

## Verification results (2026-11-03, T002)

Checked against gitlab.com with schema introspection and live, read-only queries as @DusanJ88 (token `read_api`). The recorded responses are in `src-tauri/tests/fixtures/`. They're trimmed to a few nodes, and other people's names, item titles, descriptions and comment bodies are replaced with neutral text; field shapes, ids and enum values are real.

| Item | Result | Consequence |
|---|---|---|
| R2.1 `group.workItems(includeDescendants:)` | ✅ exists (Group only; Project has no `includeDescendants`) | Used as planned. |
| R2.2 MR `sort` enum | ✅ `CREATED_DESC/ASC`, `UPDATED_DESC/ASC`, `MERGED_AT_*`, `CLOSED_AT_*`, `TITLE_*`, `POPULARITY_*`, `PRIORITY_*`, `LABEL_PRIORITY_*`, `MILESTONE_DUE_*`. `WorkItemSort` adds `DUE_DATE_*`, `START_DATE_*`, `WEIGHT_*`, `STATUS_*`, `HEALTH_STATUS_*` and has no `MERGED_AT_*`. | Sort map is per kind. |
| R2.3 Complexity at `first: 50` | ✅ MR list 109/250, work-item list 52/250, to-dos 38/250, notes batch (2 items) 39/250 | Keep `first: 50`. Notes batches of 10 stay under the limit. |
| R2 dashboard connections | ✅ `authored`/`reviewRequested`/`assignedMergeRequests` all take `state labels milestoneTitle not draft search sort`. Each lacks the filter for its own relation (`authored` has no `authorUsername`, `reviewRequested` no `reviewerUsername`, `assigned` no `assigneeUsername`). | One `DashboardMRs` query with `@include(if:)` per relation. |
| R3 status filter | ✅ `status: WorkItemWidgetStatusFilterInput { id, name }` on both `group.workItems` and `project.workItems`. `{name: "To do"}` works on login5. | Retry-without-status kept for other tiers. |
| R4 `not` filters | ✅ MR: `MergeRequestsResolverNegatedParams { labels, labelName, milestoneTitle: String, authorUsername, assigneeUsernames, reviewerUsername, … }`. Work item: `NegatedWorkItemFilterInput { labelName: [String!], milestoneTitle: [String!], authorUsername: [String!], assigneeUsernames, types, … }`. | MR uses `labels` + `milestoneTitle` (single string); work items use `labelName` + `milestoneTitle` (list). |
| R5 work-item comment count | ⚠️ No `userNotesCount` on `WorkItem`. It has `userDiscussionsCount: Int!` (threads, not notes), and the notes widget's `NoteConnection` has `count`. | **Decision:** show `userDiscussionsCount` on the row. Trigger a notes fetch when it goes up **or** when `updatedAt` changes (catches replies inside an existing thread); seen note ids keep this exact. No extra query (fallback (b) not needed). |
| R5 health status | ⚠️ The `WorkItemWidgetHealthStatus` widget isn't returned on login5 (tier). | Optional field; row just omits it. |
| R5 other widgets | ✅ Assignees, Labels, Milestone, Iteration, StartAndDueDate, Weight, Status (`name color iconName`), AwardEmoji, Description, Hierarchy (`rolledUpCountsByType { countsByState { all closed } }`). | As in R5. |
| R5 `detailedMergeStatus` | ✅ Enum confirmed; `UNCHECKED` is common (computed lazily) and falls through the derivation table. Reviewer `reviewState` adds `UNAPPROVED` and `REVIEW_STARTED`. | Derivation unchanged. |
| R5 avatars | ⚠️ `avatarUrl` is usually a **relative** path (`/uploads/-/system/user/avatar/…`). | Made absolute with `GITLAB_HOST` before the `avatar://` wrapping (as data-model says). |
| R6 notes | ✅ MR: `mergeRequest.notes(last: 20, filter: ONLY_COMMENTS)`. Work item: `WorkItemWidgetNotes.notes(last: 20, filter: ONLY_COMMENTS)` (simpler than `discussions`, which has no `last`). Nodes come oldest first. | Use `notes`, not `discussions`, for both. |
| R6/R7 note ids | ⚠️ The same note has different gid type prefixes (`Note/`, `DiscussionNote/`, `DiffNote/`), with the same number. | `seen_comment_ids` stores the numeric part only. |
| R7 to-dos | ✅ `TodoActionEnum` has `mentioned`, `directly_addressed`; `TodoStateEnum` `pending`, `done`; `Todo.note { id }`, `targetUrl`, `target` (`Todoable` interface with `webUrl`), `sort: CREATED_DESC`. | As planned. |
| R7 to-do targets | ⚠️ Work-item to-dos come back as `Issue` with gid `gid://gitlab/Issue/N`; the list has `gid://gitlab/WorkItem/N` with the same N. | Normalise `Issue/N` → `WorkItem/N` for row routing and the visible-tab rule. |
| R8 merged check | ✅ `project.mergeRequest(iid) { state mergedAt mergeUser { username } }`. | As planned. |
| R14 label names | ⚠️ Scoped: `state::Ready to Merge`, `state::Ready for Testing`, `state::In Testing`. With the unscoped names the "My tickets" filter let `state::In Testing` items through (64 vs 49 items). | Defaults fixed above. |

