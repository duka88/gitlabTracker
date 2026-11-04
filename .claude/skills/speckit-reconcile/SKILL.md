---
name: "speckit-reconcile"
description: "Detect and (on approval) fix drift between a spec-kit feature's artifacts — spec.md/plan.md/tasks.md plus design docs (data-model.md, contracts/, …) — and what actually shipped, read from the code and git history (plus PR/MR threads and any decision log the team happens to keep). Closes the code-vs-artifact gap that speckit-analyze cannot see, so a subsequent analyze pass is meaningful. Run after implementation/review rounds, before speckit-analyze."
argument-hint: "Optional feature dir or focus areas (defaults to the current feature)"
compatibility: "Requires spec-kit project structure with .specify/ directory and post-plan implementation commits. Works from code + git history alone; a decision log is an optional convenience, not a dependency."
metadata:
  author: "logineko"
  source: "authored in-repo (companion to speckit-analyze)"
user-invocable: true
disable-model-invocation: false
---

## User Input

```text
$ARGUMENTS
```

You **MUST** consider the user input before proceeding (if not empty). It may name the feature directory or narrow the focus (e.g. "just the overlay FRs").

## Why this exists

The `speckit-analyze` skill cross-checks `spec.md` ↔ `plan.md` ↔ `tasks.md` against each other and the project constitution — and reads nothing else. Code, commit history, PR/MR review threads are all invisible to it. So the drift that accumulates *after* the plan freezes — behavior added during review, decisions that supersede an FR, WONTFIX calls, capabilities no FR describes — is exactly what analyze cannot see, and running it on drifted artifacts yields a clean-looking report that is quietly wrong.

Reconciliation is the missing prerequisite: fold implementation reality and recorded decisions back into the artifacts, so analyze validates the current design instead of a frozen one.

Pipeline: **implement / review → `speckit-reconcile` (this) → `speckit-analyze` → merge.** The family also gates analyze *before* implementation (tasks → analyze → implement); this adds a second, post-implementation pass and does not replace that gate.

## Goal

Report every gap between the feature's artifacts and reality, classified and severity-ranked. Then, on explicit approval, apply the smallest edits that make the artifacts reflect what actually shipped and was decided — inventing no new scope.

**The rule: a code change that put an artifact into drift gets propagated back into that artifact — whichever artifact that is.**

"Artifacts" throughout means the feature's spec-kit files — the `spec.md`/`plan.md`/`tasks.md` triad plus whichever design docs the feature keeps (`data-model.md`, `contracts/`, `research.md`, `quickstart.md`) — never build or CI artifacts.

## Operating Constraints

- **Read-only until approved.** Deliver the report first; touch the artifacts only after the user explicitly approves the edits.
- **Reflect reality, never invent scope.** Record only what shipped and what was decided — no aspirational requirements, no design improvements.
- **Smallest faithful edit.** Amend the existing FR / clarification / claim in place; add a new `FR-###` only for a genuinely new capability. Record shipped work as new completed `T###` tasks continuing the numbering — don't rewrite what existing tasks say. Preserve each artifact's voice, numbering and formatting.
- **Decisions are first-class.** A WONTFIX / not-a-bug / deferral is a reconciliation outcome — record it against the FR or claim it bears on.
- **Decision sources are read-only.** Commit messages, PR/MR threads and any decision-log file are inputs, never edited. A decision log (a findings/learnings/followups file, if the team keeps one) is an optional shortcut — never required, its shape never assumed; commits and threads are the always-available source. This holds everywhere below without restating.
- **Standalone by design.** This command deliberately lives outside the vendored spec-kit installment: no `.specify/extensions.yml` hooks exist for it and none should be added. Commit approved edits yourself.

## Execution Steps

### 1. Locate the feature

Parse `$ARGUMENTS` first. A token selects the target feature when it resolves to an existing directory — as given, as `specs/<token>`, or (in any position) when it starts with `specs/` and exists. A `specs/…` token that does not exist is a probable typo: abort and say so. Every other token is focus text; never abort over one. When a path is found, export both overrides before running the script:

```bash
export SPECIFY_FEATURE_DIRECTORY="<path>"        # directory resolution (priority-1 override)
export SPECIFY_FEATURE="$(basename "<path>")"    # passes the script's separate branch-name gate
```

Without the second export, an explicit path dies with "Not on a feature branch" when run from `main` — the post-merge case explicit paths exist for.

With no path token, the script resolves the feature as: `SPECIFY_FEATURE_DIRECTORY` → `.specify/feature.json` (persisted by the last `speckit-specify` run, possibly stale) → branch-name prefix. Cross-check the result against the current branch; on mismatch, say which source won and confirm with the user — a stale `feature.json` silently targets the wrong feature.

Then, from repo root:

1. `.specify/scripts/bash/check-prerequisites.sh --json --require-tasks --include-tasks` → `FEATURE_DIR`, `AVAILABLE_DOCS`.
2. The same script with `--paths-only --json` → `REPO_ROOT`, the root the script built `FEATURE_DIR` from (`git rev-parse --show-toplevel` can disagree with it).
3. `REL_FEATURE_DIR="${FEATURE_DIR#"$REPO_ROOT"/}"`, trailing slash stripped. It must be non-empty, relative and shorter than `FEATURE_DIR`; abort otherwise — a git path filter built from a broken value silently matches everything or nothing.
4. Reconciliation targets: the triad plus every design doc present. Note any decision log (`*FINDINGS*.md`, `*LEARNINGS*.md`, `*FOLLOWUP*.md`, `decisions/`, `open-questions.md`, a review log).
5. The script validates `plan.md`/`tasks.md` but never `spec.md` — verify all three triad files yourself; if one is missing, abort and name the skill that creates it (`speckit-specify`, `speckit-plan`, or `speckit-tasks`).

### 2. Reconstruct the rounds from git history

Resolve the comparison base deterministically — never guess. The target branch is, in order: the open MR/PR's target branch (when a git host is reachable), else the remote head, else `main`. Retrieve that MR's review threads too, **with pagination** — they are a primary decision source; if no host or MR is reachable, carry that into the report as a named coverage limitation rather than silently dropping thread decisions.

```bash
target=${target:-$(git symbolic-ref --short refs/remotes/origin/HEAD 2>/dev/null || echo main)}
git rev-parse --verify -q "$target" >/dev/null || target=main
BASE=$(git merge-base "$target" HEAD)
```

History is the cheapest drift detector. Tag every commit in `BASE..HEAD` by what it touched:

```bash
# C = code, S = spec-kit artifact, D = decision log (align the D pattern with what step 1 found)
git log --no-merges --format='%h %s' "$BASE"..HEAD | while IFS=' ' read -r h subj; do
  files=$(git show --name-only --format= "$h")
  s=$(printf '%s\n' "$files" | grep -Fc \
        -e "$REL_FEATURE_DIR/spec.md" -e "$REL_FEATURE_DIR/plan.md" -e "$REL_FEATURE_DIR/tasks.md" \
        -e "$REL_FEATURE_DIR/data-model.md" -e "$REL_FEATURE_DIR/contracts/" \
        -e "$REL_FEATURE_DIR/research.md" -e "$REL_FEATURE_DIR/quickstart.md" || true)
  c=$(printf '%s\n' "$files" | grep -cE '\.(vue|ts|tsx|js|java|kt|kts|py|go|rb|sql|sh|yml|yaml|properties|xml|tf)$' || true)
  d=$(printf '%s\n' "$files" | grep -icE 'findings|learnings|followup|decisions/|open-questions' || true)
  tags=""; [ "$c" -gt 0 ] && tags="${tags}C"; [ "$s" -gt 0 ] && tags="${tags}S"; [ "$d" -gt 0 ] && tags="${tags}D"
  printf '[%s] %s %s\n' "$tags" "$h" "$subj"
done
```

`grep -F` needs no escaping, `|| true` survives `set -e`, and the code-extension list must track what the repo actually ships — a file type it misses is invisible drift.

Read the pattern: `[C]`/`[CD]` commits moved code or decisions without the artifacts — the prime suspects. The drift window is the run of such commits after the last `[S]` commit (`<last-pre-review>`); with no `[S]` commit on the branch (artifacts authored before branching, or history squashed), use `BASE`. Diff the artifacts over that window — `git diff <last-pre-review>..HEAD -- "$REL_FEATURE_DIR"`, ignoring decision-log churn — to see what was already reconciled and must not be re-flagged. Then read the commit messages and MR threads over the window: they carry the *why* and the disposition (fixed / deferred / rejected).

### 3. Build the two inventories

**Artifact side:**
- Each `FR-###` / `SC-###` with its normative claim; acceptance scenarios; edge cases; clarifications.
- Each `plan.md` claim — architecture, data model, file/module layout, technology constraints. List them explicitly; they read as prose, not `FR-###`.
- Each design-doc claim — `data-model.md` entities/columns/relations, `contracts/` endpoint and payload shapes, `research.md` decisions, `quickstart.md` steps.
- Each `T###` task with its phase and completion mark.

**Reality side** — code and git history are the ground truth:
- The actual behavior of the shipped code. Read the files the feature changed (`git diff --name-only "$BASE"..HEAD` names them) plus their immediate collaborators — not whole modules — and record the material behaviors: data shapes, controls, defaults, thresholds, persistence model, interaction affordances. This is what analyze can never see; do not skip it.
- The decisions behind those behaviors and their disposition, reconstructed from commit messages and MR threads (a decision log, when present, is a faster index).

### 4. Cross-map reality → artifacts

For each reality item, find its home: which FR or clarification should describe it, which plan claim does it bear on, which design doc describes it, which task should cover it — and does that artifact match, contradict, or omit it? A shipped jOOQ DAO where the plan said Panache, a column renamed by a migration while `data-model.md` keeps the old name, a moved module boundary: all contradictions.

Then the reverse pass: for every FR/SC, plan claim and design-doc claim, confirm the code still satisfies it or a later decision superseded it.

### 5. Classify each gap

| Type | Meaning | Typical severity |
|------|---------|------------------|
| **CONTRADICTION** | An artifact asserts something the code no longer does — wrong default, changed rule, superseded architecture, outdated data-model/contract shape | **HIGH**; **CRITICAL** if the shipped behavior conflicts with a constitution `MUST` |
| **MISSING-IN-SPEC** | Shipped capability/decision with no FR/clarification | MEDIUM (HIGH if user-facing contract) |
| **MISSING-IN-TASKS** | Shipped change/decision/round with no task | MEDIUM |
| **STALE** | Artifact describes superseded wording/behavior, short of contradiction | MEDIUM/LOW |
| **DECISION-UNRECORDED** | WONTFIX / not-a-bug / deferral not noted where it bears | LOW/MEDIUM |
| **UNIMPLEMENTED** | FR/SC with no corresponding code (reverse drift) | HIGH if in-scope, else note |
| **REFINEMENT-ONLY** | Tighter implementation of an existing FR; needs at most a one-line note | LOW |

Severities use the `speckit-analyze` ladder (CRITICAL/HIGH/MEDIUM/LOW), so the hand-off needs no remapping.

### 6. Produce the reconciliation report (read-only)

Output Markdown, no file writes:

## Reconciliation Report — {feature}

**Rounds:** {n} commits — {s} artifact-touching, {c} code-touching, {d} decision-log-touching (tags overlap). Drift window: {range}.

| ID | Type | Severity | Location | Artifact says | Reality (evidence) | Proposed edit |
|----|------|----------|----------|---------------|--------------------|---------------|
| G1 | CONTRADICTION | HIGH | FR-022 | … | … (`Component.vue:LNN`) | amend FR-022: … |
| G2 | CONTRADICTION | HIGH | plan.md §Data model | … | … (jOOQ DAO shipped, `FooDaoImpl.java:LNN`) | amend plan §Data model: … |
| G3 | CONTRADICTION | HIGH | data-model.md §entity | … | … (column renamed, `V###__*.sql`) | amend data-model §entity: … |

**Blind spots analyze cannot catch:** the CONTRADICTION / MISSING-IN-SPEC / DECISION-UNRECORDED / UNIMPLEMENTED rows, cited by gap ID — reference the table, don't re-render it. This is the core value of the pass.

**Coverage:** decisions found (across commit messages / PR threads / a decision log if any) = N; reflected in the artifacts = X; with a task = Y. Resolved base + target branch, and whether MR/PR threads were reachable — name any unreachable source as a blind spot.

**Metrics:** contradictions, missing-in-spec, missing-in-tasks, stale, unrecorded-decisions, unimplemented, refinement-only.

### 7. Apply on approval

Ask: *"Apply these edits so the artifacts match what shipped?"* Then:

- First check the proposed edits against `.specify/memory/constitution.md`: never canonicalize behavior that violates a constitution `MUST` — flag it as an unratified decision instead (analyze rates that conflict CRITICAL).
- Amend contradicted/stale FRs, plan claims and design-doc claims in place; a new `FR-###` (new capability only) gets a covering task reference.
- Record decisions (WONTFIX / not-a-bug / deferral) against the FRs or claims they bear on.
- Add a tasks.md phase (e.g. "Phase N: Implementation/Review round") of completed tasks for the shipped work, each referencing the FR/US it covers the way existing tasks do — so the follow-up analyze maps them instead of flagging them unmapped.

### 8. Hand off to analyze

Recommend running the `speckit-analyze` skill next — its consistency and coverage checks now operate on the current design. If reconciliation surfaced an open product question (an unratified decision), name it for the user to settle before merge.

## Operating Principles

- **History points, code proves, decisions explain** — the tags find the window fast, the code is ground truth for behavior, the decisions carry the why and the disposition.
- **Contradictions outrank omissions** — an artifact that lies about the code is worse than one that is merely silent.
- **Name analyze's blind spots explicitly** — surfacing what analyze structurally cannot see is the deliverable's whole point.
- **Reconcile, don't redesign** — make the artifacts true, not better. New scope belongs in a new `speckit-specify` run.
- **Deterministic IDs** — rerunning without changes must produce stable gap IDs and counts.

## Gotchas

- **Squashed/rebased history collapses rounds.** Per-commit tagging under-counts; fall back on the window content diff and the MR threads for round structure.
- **Base misresolution silently changes everything.** A wrong base changes the whole inventory; that is why the report echoes the resolved base and target branch.
- **Drift hides in prose.** Plan and design-doc claims aren't `FR-###`; an FR-only scan misses them. That is what the artifact-side inventory is for.
- **Thread-only decisions can be invisible.** No thread found ≠ no decision made — report unreachable sources as coverage limitations.
- **Re-runs must not double-count.** Already-reconciled claims show up in the window diff; exclude them so IDs and counts stay stable.

## Context

$ARGUMENTS
