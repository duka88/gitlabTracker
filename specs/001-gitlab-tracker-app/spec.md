# Feature Specification: GitLab Tracker Menu Bar App

**Feature Branch**: `001-gitlab-tracker-app`

**Created**: 2026-11-03

**Status**: Draft

**Input**: User description: "A Mac menu bar app. You click the icon and a small window with tabs drops down. Each tab shows a filtered GitLab list that looks like GitLab's own list pages, with all labels and metadata. The app checks GitLab every 2 minutes and sends Mac notifications for new items and new comments. It talks to GitLab directly, so there's no server." (full description: see Input appendix at the end of this file)

## Clarifications

### Session 2026-11-03

- Q: What should happen when one check finds several new comments from other people? → A: One macOS notification per comment (no grouping). In addition, the window has a bell icon at the top right with an unread-count number. Clicking it opens a popup that lists every notification, and clicking an entry jumps to that item's row in its tab.
- Q: If the window is open and showing a tab when a check finds a new item in that tab, does it count as seen? → A: Yes. The row gets the NEW tag and orange border, but it isn't added to the menu bar count, no macOS notification is sent and no bell entry is created. The same applies to new comments on MRs in the visible tab.
- Q: How many items should each tab load and watch? → A: All items matching the tab (the lists are small); load every page, with a 500-item safety limit and a warning.
- Q: Should the menu bar number be the same as the bell's unread number? → A: No, they stay separate. The menu bar shows unseen new things and clears per tab when that tab is opened; the bell shows unread notifications and clears when its popup is opened. Each tab also shows its own new-count badge, and every row with something new (new item or new comments) is marked.
- Q: Should the app also notify about changes to items already in a tab? → A: Yes, for all tabs. MRs: pipeline failed, approved, changes requested, merged. Work items: status changed, labels changed.
- Q: Should new comments on tickets (work items) also notify? → A: Yes. Every tab, MR or work item, has a "notify on new comments" switch, on by default for all 4 default tabs.
- Q: Should the app notify about mentions, assignments or review requests on items outside the tabs? → A: Notify when someone @mentions the user anywhere in GitLab. No extra tab (review requests are already covered by the Reviewing tab).
- Q: What should happen when the user clicks a macOS notification popup? → A: Open the window and jump to that item's row in its tab (or open the item in the browser if it isn't in a tab). If the app can't tell which notification was clicked, open the window with the bell popup showing.
- Q: How should the app handle the GitLab token expiring? → A: Nothing special. When GitLab rejects the token, the red "Token rejected" bar is shown (FR-031) and the user updates it in Settings.

### Session 2026-11-04

- Q: Can the user move and resize the window? → A: Yes. It can be dragged by its header bar (tab bar or Settings header) and resized from its edges, down to a minimum of 380 × 420. On first show it opens under the menu bar icon; after that it reopens where the user left it, at the size they chose, until the app restarts.
- Q: When is an MR "Ready to merge"? → A: Only when someone has actually approved it (enough people to meet the rules, and at least one even if the project requires none), nothing else blocks it, and it has no unresolved threads. GitLab reports `approved: true` for MRs nobody approved when no approval rules exist; that alone isn't enough.
- Q: How do I see review comments I still need to address on an MR? → A: The row shows the number of unresolved threads ("N to resolve", orange), and the merge-status tag is "Unresolved threads" while any are open.
- Q: How do I see which of my branches are on the shared test branch (`resource-test`)? → A: An MR tab can name a test branch ("Only MRs merged into branch"). It then lists only the MRs whose source branch is merged into it, each tagged "on resource-test" (green) or "outdated on resource-test" (orange, commits pushed after the last merge into it), with the source branch name. MRs already merged to main drop out because the tab lists open MRs; MRs not on the test branch are hidden. A tab "On resource-test" (My MRs URL + `resource-test`) was added.
- Q: An MR shows "Ready to merge" although its ticket is still being tested. What should decide readiness? → A: The linked ticket's workflow state. An MR that is approved and mergeable is "Ready to merge" only when its ticket has `state::Ready to Merge`; otherwise it shows "Approved". Each MR row shows the ticket number and its state (e.g. "#19368 · Ready for Testing"), clickable to open the ticket, and says explicitly "approved" or "not approved". Example: !19717 (approved by one reviewer, 1/1) resolves #19368 in "Ready for Testing" → "Approved".
- Q: Which ticket does an MR resolve? → A: Its "closes" link if it has one; otherwise the linked ticket whose number starts the branch name (`19368-email-…` → #19368). GitLab often records the link only as "mentioned".
- Q: Some MRs show no ticket state. Can we get it? → A: Yes. When GitLab's link list doesn't contain the ticket (e.g. !19667 and !16339 link nothing; !18059 links 11 items and #14796 is the 11th), the app looks the ticket up by the branch number. Tickets without a `state::` label show GitLab's status instead.
- Q: Can every MR tab flag whether the MR is on `resource-test`, not just the filtered tab? → A: Yes. A global "Test branch" setting (set to `resource-test`) adds the "on / outdated on resource-test" tag to MRs in every MR tab without hiding any.
- Q: Should the window hide when the user clicks another app? → A: No (replaces the popover behaviour in FR-002). It stays open as a normal window that goes behind other apps, not always on top. The menu bar icon brings it to the front, or hides it when it's already in front.
- Q: How does the user close the window? → A: A ✕ button at the top right of the tab bar and of the Settings header hides the window. The app keeps running in the menu bar; Quit stays a separate button.
- Q: What happens after the first token is saved? → A: The app leaves Settings and shows the tabs. Replacing a token while already signed in stays in Settings.
- Q: What does a token need? → A: A classic Personal Access Token with `read_api` is enough. A fine-grained token needs at least **User: Read**, plus read access to merge requests, work items/issues, to-dos, projects and groups for the tracked paths. Without User: Read, GitLab answers the sign-in check with an empty user, and the app reports "Token rejected".

### Session 2026-11-05

- Q: How do we see how many tickets we did each week? → A: A work-item tab can turn on "Group by month and week". Its rows are grouped by month (newest first), and inside each month by week, each heading showing its ticket count (e.g. "November 2026 · 12", "Nov 3 – Nov 9 · 5").
- Q: Whose tickets count? → A: Only mine, the tickets assigned to me; the rest of the team isn't tracked. The tab's URL sets this: `https://gitlab.com/groups/login5/-/work_items?state=closed&assignee_username[]=DusanJ88&sort=closed_at_desc`.
- Q: When does a ticket count as "done" for a week? → A: When it was closed (GitLab's closed date). A row that isn't closed is placed by its last update.
- Q: Where does a week that spans two months go? → A: It's split at the month end. Each part sits under its own month, so month totals are exact.
- Q: How far back does a grouped tab go? → A: No extra setting: the newest 500 rows of the URL (sorted by closed date), with the usual "Showing 500 of N" warning when there are more, in which case the oldest month may be incomplete.
- Q: Are the ticket rows shown under each week? → A: Yes. Month and week headings can be clicked to collapse them, leaving just the counts.
- Q: Should the Done tab be a default tab? → A: Yes. A fifth default tab "Done" (my closed tickets, grouped by month and week, notifications off) is added to FR-015. Existing installs get it through "Reset to defaults" or by adding it by hand; saved tabs aren't changed.
- Q: Should "On resource-test" be a default tab too? → A: Yes, second, after My MRs: my open MRs filtered to the test branch `resource-test` (FR-034), all notifications on. That makes 6 default tabs.
- Q: Should all months and weeks be expanded? → A: No. Only the current month and the current week (today's) start expanded; every other month and week starts collapsed, showing just its count. Clicking a heading expands or collapses it.
- Q: How many months does a grouped tab show? → A: The last 12 months (this month and the 11 before it). When older tickets were loaded, a "Show more" button under the last month adds 12 more months per click. This only limits what's shown; the tab still loads up to 500 rows.
- Q: Should a grouped tab notify about newly closed tickets? → A: Not by default. Turning grouping on in the tab editor also turns off the tab's three notify switches; the user can turn them back on.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - See my GitLab lists from the menu bar (Priority: P1)

As a developer, I click a menu bar icon and a small window drops down under it with tabs (My MRs, Reviewing, My tickets, Ready to merge). Each tab shows the matching GitLab list with the same information GitLab's own list pages show, so I can see what needs my attention without opening the browser. Clicking a row opens it in the browser.

**Why this priority**: This is the core value. Without lists, notifications and tab editing have nothing to work on.

**Independent Test**: Save a valid token, click the icon, and confirm each of the 6 default tabs shows the same items as the matching GitLab page, and that clicking a row opens the right page.

**Acceptance Scenarios**:

1. **Given** no token is saved, **When** the app starts, **Then** the window opens straight into Settings.
2. **Given** a token is entered in Settings, **When** the user saves it, **Then** the app tests it first and shows "signed in as @username", or an error and does not save it.
3. **Given** a valid token, **When** the user clicks the menu bar icon, **Then** the window opens directly under the icon, and clicking anywhere outside it hides it.
4. **Given** the window is open, **When** a tab is selected, **Then** its rows match the GitLab list for that tab's URL (same items, same order).
5. **Given** a merge request row, **Then** it shows the title, a Draft tag if it's a draft, !IID, created time and author, target branch, milestone, all labels (scoped labels as two-part chips), pipeline status, approvals (e.g. 1/2, or just the number of approvals when the project requires none), assignee avatars, reviewer avatars marking who approved, comment count, the number of unresolved threads (e.g. "2 to resolve") when there are any, thumbs up/down counts, "updated X ago", "approved" or "not approved" (FR-036a), the work item (ticket) it resolves with that ticket's workflow state as a label chip (FR-036), the "on / outdated on <test branch>" tag when a test branch applies, with the source branch in tabs filtered by one (FR-034, FR-037), and a merge-status tag.
6. **Given** a work-item row (task, issue, bug…), **Then** it shows a type icon, the title, a lock icon if confidential, #IID, created time and author, milestone, iteration, due date (red when overdue), weight, status as a coloured dot plus its name, all labels, assignee avatars, comment count, child-task progress (e.g. 2/5), health status and "updated X ago".
7. **Given** any row, **When** the user hovers over it, **Then** the full title and a description preview are shown; **When** the user clicks it, **Then** it opens in the default browser.
8. **Given** the menu bar icon, **When** the user right-clicks it, **Then** a menu offers Open, Refresh now and Quit.
9. **Given** the app is running, **Then** there is no Dock icon.

---

### User Story 2 - Get notified about new items (Priority: P1)

As a developer, I get a macOS notification when a new merge request or ticket shows up in one of my tabs, and the menu bar shows how many new things I haven't looked at yet.

**Why this priority**: Background awareness is the main reason to run a menu bar app and not just keep a browser tab open.

**Independent Test**: With the app running, create (or get assigned) an MR or work item that matches a tab, and confirm a notification arrives within one polling interval, the tab shows the item as NEW, the menu bar count goes up, and a bell entry appears.

**Acceptance Scenarios**:

1. **Given** a tab is loaded for the first time, **Then** its current items are remembered and no notifications are sent.
2. **Given** a tab with a baseline, **When** a check finds exactly one new item and "notify on new items" is on, **Then** one notification says "New in <Tab>: !123 Title" (or "#45 Title" for work items).
3. **Given** a check finds several new items in one tab, **Then** one notification says "<N> new in <Tab>".
4. **Given** new items exist, **Then** they show an orange left border and a NEW tag, the tab shows an orange badge with the number of new things in it, and the menu bar count includes them.
5. **Given** a tab has new markers, **When** the user opens that tab, **Then** its markers clear and the menu bar count drops by that amount.
6. **Given** the window is open and showing a tab, **When** a check finds a new item in that tab, **Then** the row shows NEW and an orange border, but the menu bar count doesn't go up, no macOS notification is sent and no bell entry is added.
7. **Given** "notify on new items" is off for a tab, **Then** new items still get NEW markers and count, but no notification is sent.
8. **Given** the polling interval is set to 1, 2, 5 or 10 minutes, **Then** checks happen at that interval, and "Refresh" (in the window or the right-click menu) checks immediately.
9. **Given** notifications have been sent (new items or comments), **Then** a bell at the top right of the window shows the number of unread notifications. **When** the user clicks the bell, **Then** a popup lists each notification (newest first), showing its text, tab and time. **When** the user clicks an entry, **Then** the app switches to that tab and scrolls to and highlights the item's row.

---

### User Story 3 - Get notified about new comments from others (Priority: P2)

As a developer, I get a notification when someone else comments on an MR I authored or am reviewing, or on a ticket in my tabs, but not for my own comments or GitLab's automatic system notes.

**Why this priority**: Review back-and-forth is time-sensitive, but it builds on the lists and polling from Stories 1–2.

**Independent Test**: Have a colleague comment on an MR in "My MRs" and confirm a notification "💬 Name on !123" with a snippet arrives within one interval. Repeat on a ticket in "My tickets" ("💬 Name on #45"). Then comment yourself and confirm nothing arrives.

**Acceptance Scenarios**:

1. **Given** an MR or work item in a tab with "notify on new comments" on, **When** its comment count goes up, **Then** the app fetches the latest comments and notifies once per new comment written by someone else, e.g. "💬 Marko on !123" plus a short snippet.
2. **Given** the new comment is by the signed-in user or is a system note, **Then** no notification is sent.
3. **Given** the same item appears in two comment-enabled tabs, **Then** each comment notifies only once.
4. **Given** an MR or work item has new comments from others, **Then** its comment count is shown in orange until the user opens that tab.
5. **Given** the app is restarted, **Then** comments that already notified do not notify again.
6. **Given** an MR or work item is seen for the first time, **Then** its existing comments are remembered as the baseline and do not notify.
7. **Given** a check finds several new comments from others, **Then** each comment gets its own macOS notification (no grouping).

---

### User Story 4 - Get notified about changes to my items (Priority: P2)

As a developer, I'm told when something important changes on an item already in my tabs: my MR's pipeline fails, it gets approved, changes are requested, it's merged, or a ticket's status or labels change.

**Why this priority**: These are the moments that need action, but they build on polling and the per-item state from Stories 1–3.

**Independent Test**: With "notify on changes" on, make a pipeline fail on an MR in "My MRs" and move a ticket in "My tickets" from To do to In progress. Confirm one notification each within one interval, plus a bell entry and a "changed" marker on the row.

**Acceptance Scenarios**:

1. **Given** an MR in a tab with "notify on changes" on, **When** its pipeline turns failed, it becomes approved, changes are requested or it's merged, **Then** one notification is sent for that change.
2. **Given** a work item in a tab with "notify on changes" on, **When** its status or labels change, **Then** one notification shows the old → new status or the labels added and removed.
3. **Given** an MR leaves an open-MR tab because it was merged, **Then** a "merged" notification is sent. If it was closed, no notification is sent.
4. **Given** an item has an unseen change, **Then** its row shows an orange border and a "changed" tag until the tab is opened, and the change counts in the tab badge and menu bar count.

---

### User Story 5 - Get notified when someone mentions me (Priority: P2)

As a developer, I get a notification when someone @mentions me anywhere in GitLab, even on an MR or ticket that isn't in any of my tabs.

**Why this priority**: A mention is a direct request for my attention, and my tab filters can easily miss it (e.g. a colleague's ticket or a Backlog item).

**Independent Test**: Have a colleague write "@DusanJ88 can you check?" on a ticket that's in none of the tabs. Confirm a "📣 Marko mentioned you on #78" notification and a bell entry arrive within one interval, and that clicking the entry opens #78 in the browser.

**Acceptance Scenarios**:

1. **Given** "notify when I'm mentioned" is on, **When** someone else @mentions the user in a comment or description, **Then** one notification shows who mentioned them, the item reference and a short snippet.
2. **Given** the mention is on an item that isn't in any tab, **Then** the bell entry opens the item in the browser when clicked.
3. **Given** the mention is in a new comment on an item in a comment-enabled tab, **Then** only one notification is sent (the mention), not a second comment notification.
4. **Given** the app starts for the first time, **Then** existing mentions are remembered and don't notify.

---

### User Story 6 - Manage tabs and settings (Priority: P2)

As a developer, I can create a new tab by filtering a list in GitLab, copying the URL and pasting it into the app. I can also rename, reorder, delete and toggle notifications for tabs, and change the checking interval.

**Why this priority**: The 6 default tabs cover the main need. Editing makes the app adaptable.

**Independent Test**: In Settings, add a tab with a pasted GitLab MR or work-item list URL, see the live "valid" hint, save, and confirm the new tab loads the same items as GitLab.

**Acceptance Scenarios**:

1. **Given** the tab editor, **When** the user pastes a URL, **Then** a live hint shows "valid" (with the detected type: merge requests or work items) or "invalid" with a reason.
2. **Given** a valid tab is saved, **Then** it appears in the tab bar and loads data on the next check (or straight away).
3. **Given** the editor, **Then** the user can rename, reorder, delete, and toggle "notify on new items", "notify on new comments" and "notify on changes" for any tab.
4. **Given** the app starts with no saved tabs, **Then** it creates the 6 default tabs: My MRs, On resource-test, Reviewing, My tickets, Ready to merge, Done.
5. **Given** the interval setting, **Then** the user can pick 1, 2, 5 or 10 minutes (default 2), and the footer shows "Updated Xm ago · every N min".
6. **Given** a work-item tab with "Group by month and week" on and a URL listing closed tickets, **Then** the tab shows a heading per month with its ticket count, and under it a heading per week (Monday–Sunday, newest first) with its count, followed by that week's rows; only the current month and week start expanded, the others show just their count until clicked; only the last 12 months are shown, with "Show more" adding 12 older months per click (FR-038).

---

### User Story 7 - Clear errors and graceful degradation (Priority: P3)

As a developer, when something goes wrong I see a clear message, and I still get partial results wherever possible.

**Why this priority**: It improves trust and makes problems easier to debug, but nothing depends on it.

**Independent Test**: Revoke the token, use a URL for a non-existent project, and use a status filter the server doesn't support; confirm the red/orange bars show the expected messages.

**Acceptance Scenarios**:

1. **Given** the token is rejected, **Then** a red bar says "Token rejected" and offers a way into Settings.
2. **Given** a tab's project or group doesn't exist or isn't accessible, **Then** that tab shows a red bar such as "Project not found" or "No permission", and the other tabs keep working.
3. **Given** GitLab rejects a work-item status filter, **Then** the app retries without that filter, shows the results, and shows an orange warning bar that the status filter was ignored.
4. **Given** GitLab is unreachable, **Then** the last loaded data stays visible with an error bar, and the next check tries again.

### Edge Cases

- A pasted URL with unsupported filters: supported filters are applied, and the unsupported ones are listed in the editor's hint as ignored.
- A pasted URL that's not a GitLab list page (e.g. a single MR, or a different host): marked invalid.
- A pasted dashboard work-item/issue URL (`/dashboard/issues`, `/dashboard/work_items`): marked invalid with the hint "use a group or project URL".
- A pasted URL with a sort order the app can't reproduce: the list uses "updated, newest first" and the tab shows an orange warning "Sort not supported — sorted by last updated".
- An item leaves a tab and comes back later: it counts as new again only if it wasn't in the previous check's result.
- More items than one page: the app loads every page, so all matching items are shown and tracked. As a safety limit it stops at 500 items per tab and shows a warning ("showing 500 of N").
- A tab's URL is edited: its baseline resets, so the first load after the edit doesn't notify.
- Checks overlap (slow network plus a short interval or a manual refresh): a new check never starts while one is still running.
- The machine sleeps: within 1 minute of waking, exactly one check runs (even if the interval hasn't passed), and there's no burst of queued checks.
- Notifications are denied in System Settings: the app still works, and markers and the count still update.
- A comment is edited or deleted: no notification.
- Known v1 limitation: new comments are detected when an item's comment count goes up. If one comment is deleted and another added between two checks, the count stays the same and that new comment isn't notified (it still shows in GitLab). Mentions of the user in that comment are still caught by FR-022e.
- An item is new and also changed in the same check: only the "new" notification is sent.
- A pipeline fails, gets retried and fails again between checks: one "failed" notification per check where the status was not failed before.
- Several changes on one item in one check (e.g. status and labels): one notification per change type.
- Very long titles or many labels: rows wrap labels and truncate the title, and hover shows the full title.
- A fine-grained token without **User: Read**: GitLab returns HTTP 200 with `currentUser: null` (REST returns 403 `insufficient_granular_scope`). The app treats it as "Token rejected" and doesn't save it. The Settings hint names `read_api` (classic token) as the simple option.
- A test-branch tab where branches were merged into the test branch more than 100 merges ago, or merged with a non-standard commit title, and pushed since: shown only once the latest commit is on the test branch (the outdated check looks at the newest 100 first-parent commits and git's "Merge branch '<name>'" titles). A squash or cherry-pick onto the test branch isn't detected.
- A grouped tab (FR-038) with more than 500 closed tickets: only the newest 500 are grouped, the "Showing 500 of N" warning shows, and the oldest month shown may be incomplete. A ticket reopened and closed again counts in the week of its latest close.
- A list taller than the window: the list scrolls inside the window; the page itself never scrolls, so the header can't be scrolled out of reach.

## Requirements *(mandatory)*

### Functional Requirements

**Menu bar & window**

- **FR-001**: The app MUST run as a menu bar (status bar) item with no Dock icon.
- **FR-002**: Left-clicking the icon MUST open a small window positioned directly under the icon (FR-002a covers later positions; FR-002d covers how it stays open).
- **FR-002a**: The window MUST be movable by dragging empty space in its header (tab bar or Settings header); buttons, tabs and the bell popup keep their click behaviour. It MUST be resizable from its edges, with a minimum size of 380 × 420 (default 1100 × 640). When placed under the icon it MUST be shifted sideways to stay on its screen. Once it has been placed under the icon, later opens MUST keep the user's position and size until the app restarts.
- **FR-002b**: Only the content area MUST scroll (the tab's list, or the Settings body). The header, error bars and footer stay fixed, and the user can always scroll back to the top.
- **FR-002c**: The tab bar and the Settings header MUST each show a close (✕) button at the top right that hides the window. It MUST NOT quit the app.
- **FR-002d**: The window MUST behave like a normal window: it stays open behind other apps and is not always on top. Clicking the menu bar icon MUST hide the window when it's in front, and otherwise bring it to the front. The active tab counts as seen (FR-023, FR-023a) only while the window has focus; returning focus to the window MUST clear the active tab's markers, like reopening it.
- **FR-003**: Right-clicking the icon MUST show a menu with Open, Refresh now and Quit.
- **FR-004**: The menu bar item MUST show the number of unseen new things (new items, items with unseen new comments and items with unseen changes) next to the icon, and nothing when it's zero. It is the total of the tab badges (FR-030), counting an item that appears in several tabs once. It is separate from the bell's unread number (FR-024b): opening a tab lowers the menu bar number, and opening the bell popup clears only the bell.

**Authentication & settings**

- **FR-005**: The app MUST authenticate with a GitLab Personal Access Token (read_api scope), store it only in the macOS Keychain, and never show it back in full after saving.
- **FR-006**: Before saving a token, the app MUST test it against GitLab and show "signed in as @username", or the failure reason.
- **FR-007**: With no token saved, the app MUST open into Settings. When a token is saved and the user goes from signed out to signed in, the app MUST switch to the tabs view.
- **FR-008**: Users MUST be able to set the check interval to 1, 2, 5 or 10 minutes (default 2), and to turn "notify when I'm mentioned" on or off (default on).
- **FR-009**: Settings and tabs MUST persist across restarts.

**Tabs & URL translation**

- **FR-010**: A tab MUST have a name, a GitLab list URL, "notify on new items", "notify on new comments" and "notify on changes" (all tab types).
- **FR-011**: The app MUST accept gitlab.com merge-request list URLs (dashboard, group and project level) and work-item/issue list URLs (group and project level only, because GitLab's API has no cross-group work-item list), and turn them into equivalent API queries.
- **FR-012**: Filters carried over from the URL MUST include: state, author, reviewer, assignee, labels, excluded labels, milestone, excluded milestone, status (work items), type (work items), draft (MRs), search and sort. The URL's page size is ignored, because each tab loads all matching items (FR-016).
- **FR-013**: The tab editor MUST give a live valid/invalid hint while typing or pasting a URL, including the detected list type and any ignored parameters.
- **FR-014**: Users MUST be able to add, rename, reorder and delete tabs, and toggle their notification switches.
- **FR-015**: On first start (no saved tabs), the app MUST create these 6 tabs for gitlab.com / user DusanJ88 / group login5:
  - **My MRs**: open MRs authored by the user. Notify on new items, new comments and changes.
  - **On resource-test**: open MRs authored by the user whose source branch is merged into `resource-test` (test branch, FR-034). Notify on new items, new comments and changes.
  - **Reviewing**: open MRs where the user is a reviewer. Notify on new items, new comments and changes.
  - **My tickets**: open work items assigned to the user with status "To do", excluding milestone "Backlog" and labels "Ready to Merge", "Ready for Testing" and "In Testing". Notify on new items, new comments and changes.
  - **Ready to merge**: open work items assigned to the user with the label "Ready to Merge", excluding milestone "Backlog". Notify on new items, new comments and changes.
  - **Done**: closed work items assigned to the user, newest closed first, grouped by month and week (FR-038). All three notify switches off.
- **FR-016**: Each tab MUST load all items matching its URL on every check, following pages until done (safety limit: 500 items, with a warning). Each page request MUST return the row data, including pipeline status and approvals for MRs. At most one extra request per tab is allowed when GitLab can't return a field in the list query (e.g. work-item comment counts).
- **FR-017**: If a work-item query fails because of the status filter, the app MUST retry without it and show a warning on that tab.

**Polling & change detection**

- **FR-018**: The app MUST check all tabs at the configured interval, and immediately when Refresh is triggered. Checks MUST NOT overlap.
- **FR-019**: The first successful load of a tab (or after its URL changes) MUST only record a baseline, with no notifications and no NEW markers.
- **FR-020**: On later checks, items not in the tab's previous result MUST be marked NEW and counted. If "notify on new items" is on, one notification per tab per check MUST be sent: "New in <Tab>: <ref> <Title>" for one item, "<N> new in <Tab>" for several.
- **FR-021**: For MRs and work items in tabs with "notify on new comments" on, the app MUST detect a higher comment count than last seen, fetch the latest comments, and notify once per new comment that isn't by the signed-in user and isn't a system note. The notification shows the author's name, the item reference (!123 or #45) and a snippet of at most 100 characters of plain text, cut at a word boundary and ending with "…" when shortened.
- **FR-022**: Each comment MUST notify at most once across all tabs and across restarts.
- **FR-022a**: For tabs with "notify on changes" on, the app MUST compare each item's tracked state with the previous check and notify once per change:
  - MRs: pipeline changed to failed ("❌ Pipeline failed on !123"), MR became approved, i.e. all required approvals met ("✅ !123 approved"), a reviewer requested changes ("✋ Changes requested on !123 by Name"), MR was merged ("🔀 !123 merged").
  - Work items: status changed ("#45 To do → In progress") and labels added or removed ("#45 +Ready to Merge −In Testing").
- **FR-022b**: A merged MR usually drops out of a tab that shows open MRs. When an MR disappears from such a tab, the app MUST check its current state once and send the "merged" notification if it was merged. It must not notify for MRs that were only closed or that stopped matching the filter.
- **FR-022c**: Change detection MUST follow the same rules as other notifications: no notifications on the first load or after a URL edit, one notification per change even if the item is in several tabs, a bell entry for each notification, and nothing sent for the tab that's on screen in the visible, focused window (FR-023a). Two of the user's own actions MUST NOT notify: an MR the user merged themselves, and changes the user requested themselves as a reviewer. Other self-made changes (e.g. moving their own work item's status) do notify, because GitLab doesn't report who made them.
- **FR-022d**: A row with an unseen change MUST be marked until its tab is opened: an orange left border and a small "changed" tag naming what changed (e.g. "pipeline failed", "status").
- **FR-022e**: When "notify when I'm mentioned" is on (a global setting, on by default), the app MUST check on every poll for new @mentions of the signed-in user anywhere on the GitLab host (comments and descriptions, including direct "@user" addressing) and send one notification per mention: "📣 <Name> mentioned you on <ref>" plus a short snippet. Group or "@all" mentions are included only if GitLab reports them as mentions of the user.
- **FR-022f**: Mention notifications MUST follow the shared rules: a baseline on first run with no notifications, at most once per mention across restarts, a bell entry for each, and skipped if the item is in the tab currently on screen in the visible, focused window (FR-023a). When a new comment both mentions the user and would trigger a comment notification, only the mention notification is sent.
- **FR-022g**: Mentions don't change the menu bar number or the tab badges unless the item is also in a tab (where its comment marker applies). They always count toward the bell. Clicking a mention's bell entry jumps to the row if the item is in a tab, and otherwise opens it in the browser.
- **FR-023**: Opening a tab MUST clear its NEW markers and new-comment highlights, and lower the menu bar count to match.
- **FR-023a**: When the window is visible **and focused** and a tab is the active tab (FR-002d: a window left open behind other apps doesn't count), new items, new comments and changes found in that tab MUST be shown with their markers (NEW tag, orange comment count, "changed" tag), but MUST be treated as seen everywhere: the item doesn't count toward any tab badge or the menu bar number (also not after the window hides, and also not in other tabs that contain the same item), no macOS notification is sent and no bell entry is recorded. If the same item also appears in a tab that isn't on screen, it doesn't notify from there either. The markers clear the next time each tab is opened.
- **FR-024**: Clicking a macOS notification MUST open the app window and act like clicking its bell entry (FR-024d): jump to and highlight the item's row in its tab, or open the item in the browser if it isn't in any tab. If the app can't tell which notification was clicked, it MUST open the window with the bell popup showing.
- **FR-024a**: Every notification the app sends (new items, new comments, changes and mentions) MUST also be recorded in an in-app notification list. An entry is recorded whenever a tab's switch would trigger a notification, even if macOS notifications are denied in System Settings.
- **FR-024b**: The window MUST show a bell icon at the top right with the number of unread entries in that list, and no number when it's zero.
- **FR-024c**: Clicking the bell MUST open a popup listing the entries, newest first. Each entry shows its text (e.g. "New in Reviewing: !123 Title" or "💬 Marko on !123: snippet"), the tab name and a relative time. Opening the popup marks all entries as read and clears the bell number.
- **FR-024d**: Clicking an entry MUST switch to that entry's tab and scroll to and highlight the item's row. If the item is no longer in the tab, the entry opens the item in the browser.
- **FR-024e**: The notification list MUST persist across restarts and keep the most recent 100 entries.

**Rows & screens**

- **FR-025**: MR rows MUST show the fields listed in User Story 1, scenario 5. The merge-status tag MUST be one of: Ready to merge, Approved (FR-036), Needs approval, CI failed, Conflicts, Changes requested, Unresolved threads (or none, for drafts and closed/merged MRs).
- **FR-025a**: An MR counts as approved only when GitLab reports its approval rules as met **and** at least one person has approved it, and at least as many people as the rules require. With no approval rules and no approvals it MUST show "Needs approval", never "Ready to merge". The same rule decides the "approved" change notification (FR-022a).
- **FR-025b**: An MR with unresolved threads (resolvable threads minus resolved threads) MUST show the count on its row in orange ("N to resolve") and the "Unresolved threads" tag instead of "Needs approval" or "Ready to merge", even when the project doesn't require threads to be resolved before merging. "Ready to merge" therefore means: open, not a draft, no conflicts, CI not failed, no changes requested, no unresolved threads, approved (FR-025a) and GitLab reports it mergeable.
- **FR-026**: Work-item rows MUST show the fields listed in User Story 1, scenario 6, leaving out any field the item doesn't have.
- **FR-027**: Labels MUST be drawn with their GitLab colours. Scoped labels (`scope::value`) MUST be drawn as two-part chips.
- **FR-028**: Hovering a row MUST show the full title and a description preview (at most 300 characters of plain text, cut at a word boundary and ending with "…" when shortened).
- **FR-029**: Clicking a row MUST open the item's web page in the default browser.
- **FR-029a**: Rows with something new MUST be marked until the tab is opened again: a new item gets an orange left border and a NEW tag; an MR or work item with new comments from others gets an orange left border and an orange comment count showing how many are new (e.g. "💬 12 (+2)").
- **FR-030**: The tab bar MUST show each tab's item count and, when it has something new, an orange badge with the number of new things in that tab (new items, items with new comments and items with unseen changes), a Refresh button that spins while a check runs, a Settings button, and the notification bell at the top right (FR-024b).
- **FR-031**: Errors MUST be shown in a red bar (e.g. "Token rejected", "Project not found", "No permission", "GitLab unreachable") and warnings in an orange bar (e.g. "Status filter not supported", "Sort not supported — sorted by last updated", "Showing 500 of N", "Branch '<name>' not found"), scoped to the affected tab where that applies.
- **FR-032**: The footer MUST show "Updated <relative time> · every <N> min" and a Quit button.

**Test-branch tabs**

- **FR-034**: An MR tab MAY name a test branch (trimmed, no spaces; ignored on work-item tabs). The tab MUST then show only the open MRs from its URL whose source branch is merged into that branch, in the MR's own project, excluding the MR whose source branch *is* the test branch. Each row MUST show its source branch and a tag: "on <branch>" when the MR's latest commit is contained in the test branch, or "outdated on <branch>" when an earlier commit of the branch was merged into it (found through the test branch's merge commits) but newer commits weren't. MRs on neither are hidden. When no project of the listed MRs has the branch, the tab MUST show the orange warning "Branch '<branch>' not found".
- **FR-035**: Changing a tab's test branch MUST reset its baseline like a URL edit (FR-019). An MR that becomes "on <branch>" appears in the tab as a new item and notifies by the tab's "new items" rule. Going from "on" to "outdated" doesn't notify.

- **FR-036**: Each MR row MUST show the work item (ticket) the MR resolves: the work item linked with "closes", or else the linked work item whose number is the leading number of the source branch name. The ticket's reference appears after the MR reference, and its workflow state, taken from its `state::<value>` label and drawn as that label (falling back to GitLab's status as a grey tag), appears on the right next to the test-branch and merge-status tags (e.g. `state | Ready for Testing` · on resource-test · Approved). Both open the ticket in the browser when clicked (the rest of the row still opens the MR); a closed ticket is shown struck through / faded. An MR that would be "Ready to merge" (FR-025b) MUST instead show "Approved" when its ticket has a `state::` label other than "Ready to Merge" (case-insensitive). An MR without a linked ticket, or whose ticket has no `state::` label, is not held back.
- **FR-036a**: Each MR row MUST say explicitly whether the MR is "approved" or "not approved" (FR-025a), followed by the approval count when there is one (e.g. "approved 1/1").
- **FR-036b**: When an MR's linked items don't include its work item (ticket) (nothing linked, or the ticket is past the first 10 links), the app MUST look the ticket up by the number at the start of the source branch name, in the MR's project, and apply FR-036 to it. Tickets without a `state::` label show GitLab's status (e.g. "To do").
- **FR-037**: Settings MUST have a global "Test branch" (trimmed, no spaces, empty = off). When set, every MR in every MR tab that is on that branch MUST carry the "on <branch>" / "outdated on <branch>" tag of FR-034, without filtering the tab. A tab's own test branch (FR-034) still filters that tab. Changing the setting MUST trigger a refresh.
- **FR-038**: A work-item tab MAY turn on "Group by month and week" (ignored on MR tabs). Its rows MUST then be grouped by calendar month, newest first, and within each month by week (Monday to Sunday, local time), newest first; a week that spans two months is split at the month end, each part under its own month. Each month and week heading MUST show its number of rows, and a week heading its date range clipped to the month (e.g. "Oct 27 – Oct 31", "Nov 1 – Nov 2"). A row's date is its closed date, or its last-update date when it isn't closed. Rows inside a week keep the order of the tab's URL. Only the current month and the current week (by today's date) MUST start expanded; every other month and week MUST start collapsed, showing its heading and count. Clicking a heading MUST expand or collapse it, and that choice holds until the app restarts. Only months from the current one back 11 months (12 in total) MUST be shown at first; when older rows exist, a "Show more" control below the last month MUST show the number of hidden rows and add 12 more months per click. A row opened from a notification MUST be revealed (its month shown and its month and week expanded). The usual 500-row limit and warning apply (FR-016). Turning the option on in the tab editor MUST also turn off the tab's three notify switches (the user may turn them back on); turning it on or off MUST NOT reset the tab's baseline or notify.
- **FR-039**: Each tab MUST have a default sort, set in the tab editor: GitLab order (the URL's order, the default), Label (by the row's label titles A–Z, compared title by title; rows without labels last), Last updated (newest first), Created (newest first), Title (A–Z), Priority (`priority::` label, P1 first, rows without one last; ties by stage) or Stage (`state::` label in workflow order In Development → Ready for Testing → In Testing → Ready to Merge, other stages after A–Z, rows without one last; MR rows use their linked ticket's state label when they have none; ties by priority) or Approval (MR rows: approved first, then fewest approvals still needed, then most approvals given; rows without approval data last). The list view MUST show a sort picker above the rows that starts at the tab's default; a choice there holds per tab until the app restarts or the tab's default is changed. Ties keep GitLab order. In a grouped tab (FR-038) the sort applies to rows inside each week. Changing the sort MUST NOT reset the tab's baseline or notify.
- **FR-040**: Settings MUST have an "Environments" list of GitLab deployment environments to track, each a project path and an environment name (default: `qa-1` in `login5/login-eko/login-eko`). Each poll MUST read every tracked env's latest successful deployment from GitLab (ref, commit, who, when) and its state. A new "Envs" tab, shown after the list tabs while at least one env is tracked, MUST list each env with its deployed ref, short commit (opening the deploy job), deployer and time, a link to the environment in GitLab, any fetch error, and the rows from the user's tabs that are on it (clicking one jumps to its row). An MR is on an env when its source branch is the deployed ref, or its head commit is in the deployed commit's history; it is "outdated" there when newer commits were pushed (same rule as FR-034). A ticket is on an env when an MR that resolves it is (FR-036), or when the deployed ref is a branch named after it (`14337-…` → #14337). MR and ticket rows MUST show an "<env>" chip ("<env> (outdated)" when outdated), in Settings order. A new deployment on a tracked env MUST send a notification ("🚀 qa-1: resource-test deployed", with the short commit and deployer) that opens the deploy job; the first fetch of an env is a silent baseline. A failed fetch MUST keep the last known deployment and show the error. Changing the list MUST trigger a poll and MUST NOT reset any tab's baseline.
- **FR-041**: The list view toolbar MUST offer filter checkboxes next to the sort picker: "Not approved" (MR tabs only; shows only MRs that aren't approved, FR-025a) and "Not deployed" (while any env is tracked; shows only rows with no up-to-date env chip, so "outdated" counts as not deployed, FR-040). Ticked filters combine (both must match), apply before sorting and grouping, and hold per tab until the app restarts. While filtering, the toolbar MUST show "N of M"; when nothing matches, the list MUST say "Nothing matches the filters". Filters MUST NOT change counts, badges, baselines or notifications.

**Distribution**

- **FR-033**: The app MUST build into a normal macOS .app and a .dmg installer.

### Key Entities

- **Tab**: id, name, source URL, list type (merge requests / work items), notify-new-items, notify-new-comments, notify-changes, test branch (MR tabs, FR-034), group by month and week (work-item tabs, FR-038), default sort (FR-039), order.
- **Tab query**: what the URL translates into — scope (dashboard / group path / project path), list type and filter set; plus ignored parameters and validation errors.
- **List item**: a merge request or work item with every displayed field (ref, title, author, dates, labels, assignees, reviewers + approval state, pipeline status, approvals, comment count, reactions, status, type, milestone, iteration, due date, weight, health, child progress, confidential, closed date, web URL, description preview).
- **Tab state**: last result (item ids, comment counts and each item's tracked state: pipeline status, approval state, review state, MR state, work-item status, labels), baseline-established flag,  set of NEW item ids, set of items with new comments, last error/warning, last updated time.
- **Seen while visible**: the global set of items whose markers appeared while their tab was on screen. They're shown but never counted in any badge or the menu bar (FR-023a).
- **Seen comments**: the global set of comment ids already processed, used to de-duplicate across tabs and restarts.
- **Seen mentions**: the set of mention ids already processed, used for the same de-duplication.
- **Notification entry**: id, kind (new item / new comment / change / mention), tab id, item reference and web URL, text (title or author + snippet), created time, read flag. The newest 100 are kept.
- **Settings**: signed-in username/id, polling interval, notify-when-mentioned. The token itself lives in the Keychain.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Clicking the menu bar icon shows (or brings forward) the window in under 0.5 s, and clicking it again while the window is in front hides it every time.
- **SC-002**: All 6 default tabs show the same items as the matching GitLab web pages (100% match on item set, and on order for every sort the app supports).
- **SC-003**: A newly matching MR or work item produces a notification within one polling interval (≤ 2 minutes at the default setting).
- **SC-004**: A comment by someone else on a tracked MR or work item produces exactly one notification within one interval. The user's own comments and system notes produce zero.
- **SC-004a**: An @mention of the user by someone else, on any item, produces exactly one notification within one interval.
- **SC-005**: Restarting the app causes zero repeat notifications for items or comments already seen.
- **SC-006**: Opening a tab with new markers brings the menu bar count to zero for that tab immediately.
- **SC-007**: A copied GitLab list URL becomes a working tab in under 30 seconds of user effort, with no manual editing of the URL.
- **SC-008**: Every row shows every field GitLab shows for that item type in the list view, as listed in FR-025/FR-026.
- **SC-009**: The app stays responsive and uses under 150 MB of memory with 4–8 tabs of up to 200 items each.

## Assumptions

- Single user, gitlab.com only in v1 (the host isn't configurable); self-hosted or multiple servers are later additions.
- Terminology: "ticket" in screen text and stories means a GitLab **work item** (issue, task, bug, epic…). Requirements use "work item", or "work item (ticket)" where the ticket of an MR is meant.
- Default tabs target gitlab.com, user @DusanJ88 and group `login5`. The user can edit them.
- Work-item "status" (To do, In progress…) requires GitLab Premium/Ultimate. If it isn't available, the status filter is dropped with a warning and the status dot is hidden.
- Tabs are personal lists with modest volume. Every matching item is loaded and tracked, with a 500-item safety limit per tab.
- "Comment count" means user notes (non-system) as GitLab reports them in the list. Thread replies count.
- Notifications work only in the built, signed or ad-hoc-signed app. During development they may not appear, and the user must allow them in System Settings.
- Clicking a macOS notification goes to the item's row when macOS reports which notification was clicked; otherwise the app falls back to showing the bell popup (FR-024).
- Stack is fixed by the user: Tauri 2 (Rust side does the GitLab requests, polling, notifications, badge and Keychain), Vue 3 + TypeScript + Vite for screens. GraphQL is used for both list types; REST only for commit ancestry, which GraphQL doesn't offer (FR-034, research R18).
- Team conventions the MR features rely on (hard-coded in v1, could become settings later):
  - A work item's workflow state is its scoped label `state::<value>` (e.g. "In Development", "Ready for Testing", "In Testing", "Ready to Merge"); the value that allows merging is exactly "Ready to Merge", compared case-insensitively (FR-036).
  - Weeks run Monday to Sunday, in the Mac's local time (FR-038).
  - A branch name starts with the number of the work item (ticket) it resolves, e.g. `19368-email-subjects-…` (FR-036, FR-036b).
  - Branches reach a test branch (e.g. `resource-test`) through ordinary merge commits titled "Merge branch '<name>' …" (FR-034); squashes and cherry-picks aren't detected.
- Out of scope for v1: notifications for changes other than those in FR-022a (e.g. assignee, milestone or due-date changes), start at login, multiple GitLab servers, grouping/sorting inside the app beyond the URL's sort, write actions.

## Input appendix

The user's full description (stack, menu bar behaviour, tabs, URL translation, row layouts, notifications, screens, "done when") was provided in the conversation that started this feature and is summarized above. Its "Done when" list maps to SC-001–SC-006 and SC-008.
