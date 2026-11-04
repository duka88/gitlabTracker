# GitLab Tracker Constitution

GitLab Tracker is a single-user macOS menu bar app (Tauri 2 + Vue 3) that
shows filtered GitLab lists and notifies about new items and comments. These
principles are plan gates for `/speckit-plan` and `/speckit-analyze`.

## Core Principles

### I. Serverless, Direct-to-GitLab

The app talks only to the user's GitLab instance. No backend, no telemetry,
no third-party services. All network calls originate in the Rust side.

### II. Secrets Stay in the Keychain

The Personal Access Token is stored only in the macOS Keychain. It is never
written to config files, logs, the frontend's storage, or error messages, and
never sent to the webview after it has been saved (the UI sees only
"signed in as @user").

### III. Rust Owns Behaviour, Vue Owns Screens

GitLab requests, URL→query translation, polling, diffing, notification
decisions, badge count and persistence live in Rust. The Vue layer renders
state it receives over Tauri commands/events and sends user intents back. No
GitLab calls from the webview.

### IV. Read-Only and Quiet by Default

The app only reads from GitLab (`read_api`). The first load of any tab only
records a baseline and never notifies. A given item or comment notifies at
most once, across all tabs and restarts.

### V. Testable Core Logic

URL parsing/translation, new-item diffing, comment de-duplication and
merge-status derivation are pure functions with unit tests (Rust `cargo test`;
TS logic with Vitest). UI and OS integration are verified manually against
the spec's "Done when" list.

### VI. Simplicity

One window, one process, local JSON for settings and seen-state. Add a
dependency only when it removes real code. Graceful degradation over hard
failure (e.g. retry a work-item query without an unsupported filter and warn).

## Constraints

- Target: macOS (Apple Silicon and Intel), distributed as `.app` + `.dmg`.
- Stack: Tauri 2, Rust stable, Vue 3 + TypeScript + Vite.
- GitLab API: GraphQL for list tabs; REST allowed where GraphQL lacks data.
- Polling interval: 1, 2, 5 or 10 minutes; a poll must never overlap another.

## Development Workflow

- Specs, plans and tasks live under `specs/` and are kept in sync with code.
- `cargo test`, `cargo clippy`, `vue-tsc --noEmit` and Vitest must pass
  before a task is marked done.
- Commits are authored by the user only; no tool attribution in commits.

## Governance

This constitution supersedes ad-hoc practice for this repo. Amendments update
the version and date below and note the reason in the commit message.

**Version**: 1.0.0 | **Ratified**: 2026-11-03 | **Last Amended**: 2026-11-03
