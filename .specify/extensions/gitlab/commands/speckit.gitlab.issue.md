---
description: "Create a GitLab issue for a new SpecKit feature and persist its ID so the next before_specify hook can derive the branch name from it. Closes the constitution's Phase 1 gap where speckit-git-feature auto-derives a sequential number unrelated to any real issue."
---

# Create GitLab Issue

This command runs as a `before_specify` hook (registered in
`.specify/extensions.yml`) **ahead of** `speckit.git.feature`. It closes the
Phase 1 gap noted in the project constitution: the SpecKit feature-creation
script auto-derives a sequential branch number unrelated to any real issue.
This command creates the real GitLab issue first, then hands its ID to the
next hook so the **branch** reliably inherits the issue ID. It also asks the
orchestrator to point the spec folder at the same ID, but under vanilla
SpecKit that part is **best-effort** (see step 7) — the guaranteed,
script-backed result is the branch.

## User Input

```text
$ARGUMENTS
```

`$ARGUMENTS` is the user's feature description passed by the
`speckit-specify` skill. If empty, ask the user for a one-line feature title
before proceeding.

## When this command is invoked

Triggered automatically by the `speckit-specify` skill via its
`before_specify` chain. The hook is marked **optional** — the user is asked
whether to create a real GitLab issue, and can decline (in which case
`speckit.git.feature` runs with vanilla sequential numbering, the Phase 1
fallback).

## Prerequisites

- Verify that the tools named in `mcp.create_tool` and `mcp.get_tool`
  (resolved from `gitlab-config.local.yml` overlaid on `gitlab-config.yml`)
  are available as registered MCP tools. If they are not registered, do
  **not** silently fall back to sequential numbering — consult the resolved
  `on_mcp_unavailable`:
  - `fail` (the default): abort with a clear error message and instruct the
    user to either fix the MCP setup, set `gitlab-config.local.yml` with the
    correct tool names for their machine, or re-run the `speckit-specify`
    skill and decline this optional hook.
  - `skip`: do not create an issue; let the chain continue so vanilla
    `speckit.git.feature` runs with sequential numbering. Tell the user the
    GitLab hook was skipped and why.
- Per project memory (`feedback_gitlab_mcp_choice`): the team default is
  `mcp__gitlab__*` (NOT `mcp__gitlab-fdl__*`) for the logineko project. If
  a developer has a different GitLab MCP server installed (e.g. the
  official `gitlab-org/mcp`), use `gitlab-config.local.yml` to override the
  tool names rather than changing the team file.

## Configuration

Read configuration with **per-developer override** support:

1. **First**, if `.specify/extensions/gitlab/gitlab-config.local.yml` exists,
   load it. This file is **gitignored** (each developer maintains their own)
   and is the right place for personal-machine overrides — most commonly the
   MCP tool names, since the team default may not match what every developer
   has installed locally (community MCP vs official `gitlab-org/mcp` vs other).
2. **Then** load `.specify/extensions/gitlab/gitlab-config.yml` (the
   committed team default).
3. **Merge**: any key set in `.local.yml` overrides the same key in the
   committed file. Keys absent from `.local.yml` fall through to the team
   value. Deep merge for the `mcp` block (override one tool name without
   needing to copy the other).

This mirrors the standard `settings.json` ↔ `settings.local.json` pattern.

Resolved configuration keys:
- `project_id` — numeric GitLab project ID (default `14154118`,
  logineko-farming).
- `mcp.create_tool` — MCP tool name to call when creating an issue
  (default `mcp__gitlab__create_issue`). Different installed GitLab MCP
  servers (e.g. official `gitlab-org/mcp` vs the community one) expose
  different tool names; this knob lets the team swap servers without
  rewriting the skill.
- `mcp.get_tool` — MCP tool name to call when fetching an existing issue
  by IID (default `mcp__gitlab__get_issue`). Used only on the
  existing-issue path.
- `default_labels` — array of labels to apply to created issues.
- `issue_body_template` — body for issues this extension creates. The default is
  static; the IID-dependent branch/spec path isn't available until after creation,
  so don't rely on resolving it here.
- `on_mcp_unavailable` — `fail` or `skip` if MCP is unreachable.

## Detect an existing issue reference

Before creating a new issue, check whether the user is referencing an
existing one. Try in this order; **first match wins**:

1. **`GITLAB_ISSUE_IID` env var** — explicit, no confirmation needed.
   Format: a positive integer (e.g. `GITLAB_ISSUE_IID=15842`).

2. **Patterns in `$ARGUMENTS`** (case-insensitive):
   - `#15842` — bare hash followed by digits
   - `gl#15842` — GitLab shorthand
   - `issue 15842` / `issues 15842`
   - `ticket 15842` / `tickets 15842`
   - `gitlab issue 15842`
   - Full URL anywhere in the args: `gitlab.com/<group>/<project>/-/issues/15842`
     (or the `/-/work_items/15842` form GitLab now serves)

   Recommended regex (apply with `i` flag):
   ```
   (?:^|\s)#(\d+)\b
   | \bgl#(\d+)\b
   | \b(?:gitlab\s+)?(?:issues?|tickets?)\s+(\d+)\b
   | gitlab\.com/[^\s]+/-/(?:issues|work_items)/(\d+)\b
   ```

When a reference is detected:

- If it came from `GITLAB_ISSUE_IID`: proceed silently with that IID.
- If it came from `$ARGUMENTS`: confirm with the user (it may have been
  incidental phrasing, e.g. "Fix issue 15842 reporting" — the user does
  mean issue 15842, but sometimes the number is unrelated to the feature
  tracker). Ask the user: "Detected reference to issue #15842 — use that as
  the tracker, or create a new one?"

When confirmed, call the configured fetch tool (`mcp.get_tool` from
`gitlab-config.yml`, default `mcp__gitlab__get_issue`) with `issue_iid` =
the parsed IID. Choose the project to fetch from carefully, because GitLab
IIDs are **project-scoped** — the same IID exists in every project:
- For `#id` / `gl#id` / `issue N` / `ticket N` / `GITLAB_ISSUE_IID`
  references (no project context): use `project_id` from config.
- For a **full URL**: use the `<group>/<project>` path parsed from the URL
  itself (URL-encoded) as the project, not the configured `project_id`. If
  that project differs from the configured `project_id`, warn the user that
  the issue lives in another project and confirm before proceeding —
  otherwise a same-numbered issue in `project_id` would be silently linked.

Capture `iid`, `web_url`, `title` from the response. Then **skip the
create_issue step below** and continue from "Persist" with the
existing-issue data.

Derive the **short name** as in step 1 of Execution — but strip the
matched reference token from `$ARGUMENTS` first, otherwise the slug ends
up with `issue` or `15842` in it. If stripping leaves nothing usable
(e.g. the input was only `#15842` or a bare URL), derive the short name
from the fetched issue `title` instead — never emit a bare `15842-` slug
with an empty tail. If neither yields anything, ask the user for a short
name.

## Execution (create-new-issue path)

Run this section only when **no existing reference was detected** (or the
user declined to use the detected one).

**Retry guard (partial idempotency).** Before creating anything, check
`.specify/feature.json` for an existing `gitlab.issue_iid`. If one is
present, this is a re-run after a later step failed — do **not** create a
second issue; fetch that recorded issue and ask the user: "Found issue #<iid>
from a previous run — reuse it, or create a new one?" Only create a new issue
when no IID is recorded or the user declines reuse.

**Limitation:** this guard only covers retries that fail *before*
the `speckit-specify` skill reaches its own directory step, because that step
rewrites `.specify/feature.json` with only `feature_directory` and drops the
`gitlab` block (see step 7). After that point the recorded IID is gone, so a
later rerun can still create a duplicate. The `optional: true` hook prompt is
the backstop; closing this fully would require changing the upstream
`speckit-specify` skill as noted in step 7.

1. **Derive a short name** (2–4 words, kebab-case) from `$ARGUMENTS` using
   the same rules as `speckit.git.feature`: action-noun where possible,
   preserve technical terms / acronyms.

2. **Derive an issue title** from `$ARGUMENTS`: full first sentence if
   reasonably short (≤ 80 chars), otherwise a concise one-line summary.
   Append `(SpecKit pilot)` to the title **only** if the constitution still
   says we are in pilot phase (check for that wording in
   `.specify/memory/constitution.md`).

3. **Render the issue body** from `issue_body_template`. The default template is
   static text (no IID-dependent variables): the issue IID — and therefore the
   `<iid>-<short-name>` branch and spec-dir names — is not known until
   `create_issue` returns in step 5, so anything referencing them here would only
   render literal placeholders. If a team adds such variables, backfill them with a
   post-create update rather than here.

4. **Call the configured create tool** (`mcp.create_tool` from
   `gitlab-config.yml`, default `mcp__gitlab__create_issue`) with:
   - `project_id`: from config
   - `title`: from step 2
   - `description`: rendered body from step 3
   - `labels`: `default_labels` from config

5. **Capture from the response**:
   - `iid` — the project-scoped issue number (the integer used in branch
     names like `15842-foo`). This is **NOT** the global `id` field; you
     specifically want `iid`.
   - `web_url`
   - `title`

6. **Persist** the issue info into `.specify/feature.json`, merging with any
   existing keys:

   ```json
   {
     "feature_directory": "specs/<iid>-<short-name>",
     "gitlab": {
       "issue_iid": <iid>,
       "issue_url": "<web_url>",
       "issue_title": "<title>"
     }
   }
   ```

   (At hook time the spec directory doesn't exist yet — record the expected
   `specs/<iid>-…` path; `speckit-specify` creates the actual directory, which
   matches this only if the step-7 hand-off is honoured.)

7. **Hand off the branch name (guaranteed) and the spec directory
   (best-effort).** Compute `BRANCH_NAME = "<iid>-<short-name>"` and
   `FEATURE_DIR = "specs/<iid>-<short-name>"`, then **explicitly state in your
   reply** to the orchestrator (the agent running `speckit-specify`):

   > **Hand-off**:
   > - When invoking `speckit.git.feature`, set `GIT_BRANCH_NAME=<BRANCH_NAME>`
   >   so the feature script bypasses its auto-numbering and uses the GitLab
   >   issue ID. See `speckit.git.feature` § Environment Variable Override.
   > - For the `speckit-specify` skill's own directory step, set
   >   `SPECIFY_FEATURE_DIRECTORY=<FEATURE_DIR>` so the spec folder picks up the
   >   same issue ID.

   **What is guaranteed vs. best-effort:**
   - **Branch — guaranteed.** `speckit.git.feature` runs `create-new-feature.sh`,
     which honours `GIT_BRANCH_NAME` *deterministically* (a script), so the
     branch is always `<iid>-<short-name>`.
   - **Spec folder — best-effort.** The upstream `speckit-specify` skill only
     honours `SPECIFY_FEATURE_DIRECTORY` when *the user* provided it; it does not
     document consuming a value handed off by a `before_specify` hook. So the
     spec folder inherits the issue ID only if the orchestrator carries this
     hand-off into specify's directory step — otherwise specify auto-generates
     `specs/NNN-…` and **only the branch** carries the issue ID. Fully
     guaranteeing the spec folder would require a paired change to the vendored
     `speckit-specify` skill, which is out of scope per the customization
     contract; treat it as best-effort until then.

   Note also that `speckit-specify` rewrites `.specify/feature.json` with only
   `feature_directory`, so the `gitlab` block persisted in step 6 is
   **best-effort** and is expected to be overwritten; the durable record of the
   issue link is the branch name plus the GitLab issue itself.

8. Print the result as JSON for downstream parsing:

   ```json
   {
     "ISSUE_IID": <iid>,
     "ISSUE_URL": "<web_url>",
     "BRANCH_NAME": "<iid>-<short-name>",
     "FEATURE_DIRECTORY": "specs/<iid>-<short-name>",
     "SHORT_NAME": "<short-name>"
   }
   ```

## On failure

- **MCP unreachable / the configured create or fetch tool errors**: behavior
  depends on the resolved `on_mcp_unavailable`:
  - `fail` (the default): do not fall back to sequential numbering. Print the
    error verbatim and stop the `speckit-specify` flow. The user can re-run
    and decline the optional hook if they want the Phase 1 vanilla path.
  - `skip`: print a short notice and continue without creating an issue, so
    vanilla `speckit.git.feature` runs with sequential numbering.
- **Project ID misconfigured**: same — fail loud.
- **Empty `$ARGUMENTS` and user declines to supply a title**: stop the flow;
  do not create an issue with a placeholder title.

## Idempotency

The hook's idempotency depends on which path is taken:

- **Existing-issue path** (an IID was detected in `$ARGUMENTS` or set via
  `GITLAB_ISSUE_IID`): idempotent. Re-running with the same reference
  fetches the same issue; no duplicate is created. Safe to re-run if a
  later `speckit-specify` step fails and the user wants to retry.
- **Create-new-issue path** (no reference detected): **partially** guarded by
  the **retry guard** in the Execution section. It reuses a recorded
  `gitlab.issue_iid` from `.specify/feature.json` if one is still there — but
  `speckit-specify` overwrites that file (dropping the `gitlab` block) once it
  reaches its directory step, so the guard only protects retries that fail
  before that point. After a full run the IID is gone and a rerun can still
  create a duplicate. The `optional: true` hook prompt in `extensions.yml`
  remains the front-line guard against accidental runs.

## Why this exists

Constitution Workflow Integration table mandates `<issue-id>-<kebab-slug>`
as the source-of-truth shape for branches and spec folders. Without this
hook, `speckit.git.feature` auto-derives a sequential number with no
relationship to any real issue — per the constitution itself, that output
"should be discarded, not merged."

This hook closes that gap **for the branch** deterministically. The matching
**spec folder** is best-effort under vanilla SpecKit (see step 7): fully
guaranteeing it would require a paired change to the upstream
`speckit-specify` skill, deliberately left out of scope per the customization
contract. The branch ID is the reliable anchor; the spec folder follows when
the orchestrator carries the hand-off through.
