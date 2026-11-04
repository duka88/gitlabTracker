# Contract: GitLab GraphQL operations used

Endpoint: `POST https://gitlab.com/api/graphql` (`GITLAB_HOST`, research R17), `Authorization: Bearer <token>`, scope `read_api`. Queries live as `.graphql` files in `src-tauri/src/gitlab/queries/` and are embedded with `include_str!`. Fields are listed in research R5. Fields marked **VERIFY** in research must be checked once in the explorer, and the matching recorded responses saved in `src-tauri/tests/fixtures/`.

| Operation | Used for | Variables | Notes |
|---|---|---|---|
| `CurrentUser` | Token test (FR-006), the "me" filter | — | `currentUser { id username name avatarUrl }`. `null` → TokenRejected. |
| `DashboardMRs` | Dashboard MR tabs | `username`, relation via `@include` flags or 3 query variants, filters, `after` | `user(username){ authoredMergeRequests / reviewRequestedMergeRequests / assignedMergeRequests(first:50, after, state, labels, not, milestoneTitle, draft, sort) }` |
| `GroupMRs` / `ProjectMRs` | Group/project MR tabs | `fullPath`, filters, `after` | `group{ mergeRequests(includeSubgroups:true …) }` / `project{ mergeRequests }` |

The shared `MrFields` fragment (all three MR queries) also selects, since 2026-11-04: `approvedBy(first:20){nodes{username}}`, `resolvableDiscussionsCount`, `resolvedDiscussionsCount` (FR-025a/b), `sourceBranch`, `diffHeadSha` (FR-034) and `workItemRelations(first:10){ nodes{ linkType workItem{ iid reference title webUrl state widgets{ __typename …Status …Labels } } } }` (FR-036, research R19). Verified under GitLab's complexity limit at 50 per page.
| `GroupWorkItems` / `ProjectWorkItems` | Work-item tabs | `fullPath`, filters, `status?`, `after` | Widgets per R5. Retry without `status` on a schema/argument error (R3). |
| `WorkItemCommentCounts` | Fallback (b) in R5 | `fullPath`, `iids` | Only if `WorkItem` has no notes count. |
| `RecentNotesBatch` | New comments (FR-021) | built dynamically with aliases `n0…n9` | MR: `project(fullPath){ mergeRequest(iid){ notes(last:20){…} } }`. Work item: `workItem(id){ widgets{ …on WorkItemWidgetNotes{ discussions(last:20, filter: ONLY_COMMENTS){…} } } }` |
| `MergedCheckBatch` | MR removed from a tab (FR-022b) | aliases | `project(fullPath){ mergeRequest(iid){ state mergedAt mergeUser{username} } }` |
| `TicketsByIid` | Tickets missing from an MR's relations (FR-036b) | `fullPath`, `iids` (never empty) | `project{ workItems(iids, first:100){ nodes{ iid reference title webUrl state widgets{ __typename …Status …Labels } } } }`. An absent `iids` returns the latest 100 work items, so it's only sent with ≥1 iid (research R19). |
| `Mentions` | FR-022e | `after?` | `currentUser{ todos(action:[mentioned, directly_addressed], state:[pending, done], first:50){ nodes{ id action body createdAt author{…} target{…} note{id} } } }` |

## REST calls (test branch, FR-034/FR-037, research R18)

GraphQL has no commit-ancestry query, so `gitlab/branch_check.rs` uses `GitLabClient::rest_get` (`GET https://gitlab.com/api/v4/…`, same Bearer token, `read_api`). Path parts are percent-encoded. 404 means "absent" (branch or commit missing), not an error.

| Call | Used for |
|---|---|
| `projects/:path/repository/branches/:branch` | Test-branch head commit per project. |
| `projects/:path/repository/merge_base?refs[]=:a&refs[]=:b` | Ancestry: `a` is in `b` when the result is `a`. |
| `projects/:path/repository/commits?ref_name=:branch&first_parent=true&per_page=100` | Merge commits on the test branch, to find the last merge of a source branch. |

Results are cached per (project, MR head, test-branch head). REST status mapping: 401 → `TokenRejected`, 403 → `NoPermission`, 404 → absent, 429 → `RateLimited`, other → `Query`.

## Error mapping

| Response | AppError kind | UI text |
|---|---|---|
| HTTP 401, or `currentUser: null` | `TokenRejected` | "Token rejected — update it in Settings" (red, global) |
| HTTP 403 | `NoPermission` | "No permission to read this list" (red, per tab) |
| `project`/`group`/`user` root `null` | `NotFound` | "Project not found" / "Group not found" / "User not found" (red, per tab) |
| HTTP 429 | `RateLimited` | "GitLab rate limit — retrying next check" (orange) |
| Network/timeout | `Unreachable` | "Can't reach GitLab — showing last data" (red, global; items stay) |
| `errors[]` mentioning `status` on a work-item query | (retry) | "Status filter not supported — showing all statuses" (orange, per tab) |
| Other `errors[]` | `Query` | First error message (red, per tab) |
