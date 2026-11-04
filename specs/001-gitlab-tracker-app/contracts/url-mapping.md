# Contract: GitLab list URL → TabQuery

Implemented as a pure function, `parse_tab_url(url) -> Result<TabQuery, UrlError>` (host fixed to `gitlab.com`), with table-driven unit tests (one row per example below). Unknown parameters never fail the parse; they're collected in `ignored_params`.

## Accepted paths

| Pattern (after host) | Kind | Scope |
|---|---|---|
| `/dashboard/merge_requests` | MergeRequests | Dashboard (needs exactly one of author / reviewer / assignee username) |
| `/groups/<group/sub>/-/merge_requests` | MergeRequests | Group |
| `/<ns…>/<project>/-/merge_requests` | MergeRequests | Project |
| `/groups/<group/sub>/-/work_items`, `/groups/<g>/-/issues` | WorkItems | Group |
| `/<ns…>/<project>/-/work_items`, `/<ns…>/<project>/-/issues` | WorkItems | Project |

Rejected, with a reason:
- a different host;
- a single item (`/-/merge_requests/123`, `/-/work_items/45`, `/-/issues/45`);
- `/dashboard/issues` and `/dashboard/work_items` ("use a group or project URL; GitLab's API has no cross-group work-item list");
- any other path.

## Query parameters

GitLab writes arrays as `key[]=a&key[]=b` and also accepts `key=a,b` for labels. Both are accepted. Values are URL-decoded and `+` becomes a space.

| URL param | Filter | MR GraphQL arg | Work-item GraphQL arg |
|---|---|---|---|
| `state` (`opened`/`closed`/`merged`/`all`) | state | `state` (`all` → omitted) | `state` (`merged` is invalid) |
| `author_username` | author | `authorUsername` / dashboard relation | `authorUsername` |
| `assignee_username`, `assignee_username[]` | assignees | `assigneeUsername` (first) / dashboard relation | `assigneeUsernames` |
| `reviewer_username` | reviewer | `reviewerUsername` / dashboard relation | ignored |
| `label_name[]`, `label_name` | labels | `labels` | `labelName` |
| `not[label_name][]` | not_labels | `not: {labels}` | `not: {labelName}` |
| `milestone_title` | milestone | `milestoneTitle` | `milestoneTitle: [..]` |
| `not[milestone_title]` | not_milestone | `not: {milestoneTitle}` | `not: {milestoneTitle: [..]}` |
| `status`, `status_name`, `status_id` | status | ignored | `status: {name}` / `{id}` (R3, with fallback) |
| `type[]` (`issue`,`task`,`incident`,…) | types | ignored | `types: [ISSUE, TASK…]` |
| `draft` (`yes`/`no`), `wip` | draft | `draft: true/false` | ignored |
| `search` | search | `search` (group/project only) | `search` |
| `sort` (`created_date`, `created_asc`, `updated_desc`, `updated_asc`, `milestone_due_desc`, `priority`, …) | sort | `sort` enum; an unmappable value → `UPDATED_DESC` + `sort_fallback: true` (the tab warning "Sort not supported — sorted by last updated") | same |
| `per_page`, `page`, `first_page_size`, `last_page_size` | — | ignored (all items are loaded) | ignored |
| anything else | — | → `ignored_params` | → `ignored_params` |

## Dashboard MR relation

| Present param | GraphQL root |
|---|---|
| `author_username=X` | `user(username: X) { authoredMergeRequests(...) }` |
| `reviewer_username=X` | `user(username: X) { reviewRequestedMergeRequests(...) }` |
| `assignee_username=X` | `user(username: X) { assignedMergeRequests(...) }` |
| more than one | Use the first in the order above. The others are applied as filter args where the connection accepts them, otherwise added to `ignored_params`. |
| none | Invalid: "Dashboard URL needs an author, reviewer or assignee filter". |

## Test examples (must pass)

| URL | Expect |
|---|---|
| R14 "My MRs" | MR · Dashboard(Author, DusanJ88) · state opened |
| R14 "Reviewing" | MR · Dashboard(Reviewer, DusanJ88) · state opened |
| R14 "My tickets" | WorkItems · Group(login5) · assignees [DusanJ88] · status "To do" · not_milestone Backlog · not_labels [state::Ready to Merge, state::Ready for Testing, state::In Testing] |
| R14 "Ready to merge" | WorkItems · Group(login5) · labels [state::Ready to Merge] · not_milestone Backlog |
| `https://gitlab.com/login5/login-eko/login-eko/-/merge_requests?label_name[]=a::b&draft=no&sort=created_date` | MR · Project(login5/login-eko/login-eko) · labels [a::b] · draft false · sort CREATED_DESC |
| `https://gitlab.com/groups/login5/-/issues?foo=1` | WorkItems · Group(login5) · ignored [foo] |
| `https://gitlab.com/login5/x/-/merge_requests/12` | Error "single item, not a list" |
| `https://example.com/groups/a/-/merge_requests` | Error "different host" |
| `https://gitlab.com/dashboard/merge_requests` | Error "needs author/reviewer/assignee" |
