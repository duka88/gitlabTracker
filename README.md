# GitLab Tracker

A macOS menu bar app (Tauri 2 + Vue 3) that shows your GitLab lists in a dropdown window, with one
tab per pasted list URL. It notifies you about new items, comments from others, @mentions and
key changes (pipeline failed, approved, changes requested, merged, status and label changes).

- Talks only to gitlab.com, read-only (`read_api`). There's no server and no telemetry.
- The token lives only in the macOS Keychain. The UI never sees it.
- Design docs: [`specs/001-gitlab-tracker-app/`](specs/001-gitlab-tracker-app/), including the
  [GraphQL verification results](specs/001-gitlab-tracker-app/research.md#verification-results-2026-11-03-t002).

## Prerequisites

- macOS 13+, Xcode Command Line Tools (`xcode-select --install`)
- Rust stable via rustup, plus `rustup target add aarch64-apple-darwin x86_64-apple-darwin`
- Node 20+ and pnpm
- A GitLab Personal Access Token with the `read_api` scope (gitlab.com → Preferences → Access
  tokens)

## Develop

```bash
pnpm install
pnpm tauri dev            # notifications may not show in dev builds
```

Checks (all must pass before a task is done):

```bash
cd src-tauri && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
pnpm typecheck && pnpm test
```

To check the GraphQL queries against the live API without the UI (read-only, prints no
secrets):

```bash
cd src-tauri && GITLAB_TOKEN=glpat-… cargo run --example live_check
```

## Build

```bash
pnpm tauri build --bundles app,dmg --target universal-apple-darwin
open src-tauri/target/universal-apple-darwin/release/bundle/dmg/
```

The app is ad-hoc signed, so on first launch right-click it and choose **Open**, or use **System
Settings → Privacy & Security → Open Anyway**. Then allow notifications in **System Settings →
Notifications → GitLab Tracker**.

## Where data lives

| What | Where |
|---|---|
| Token | Keychain, service `com.duka88.gitlab-tracker`, account `gitlab-token:gitlab.com` |
| Settings, tabs | `~/Library/Application Support/com.duka88.gitlab-tracker/settings.json` |
| Seen items, markers, baselines | `…/state.json` |
| Bell history (newest 100) | `…/notifications.json` |
| Avatar cache (7 days) | `~/Library/Caches/com.duka88.gitlab-tracker/avatars/` |

Delete `state.json` to start fresh without notifications (each tab records a new baseline).

## Layout

- `src-tauri/src/` holds the Rust side, which owns all behaviour: `url_map` (URL → query),
  `gitlab/` (client, queries, parsing), `engine/` (pure diff and notification rules, the poller,
  the notifier), `tray`, `commands` (IPC) and `avatars` (the `avatar://` scheme).
- `src/` holds the Vue side, which only draws screens from the view model Rust pushes over
  `state://updated`.
- `src-tauri/tests/` has unit tests and anonymised recorded GraphQL responses in `fixtures/`.

## Development workflow

- **Review after every task.** When a task in `specs/*/tasks.md` is implemented, review its
  changes (correctness against the spec and contracts, edge cases, token safety, tests) before
  it is marked `[X]`. Fix the findings first, then check it off.
- A task is done only when `cargo test`, `cargo clippy -- -D warnings` (in `src-tauri/`),
  `pnpm typecheck` and `pnpm test` all pass (constitution, Development Workflow).
- Commits are authored by the user only, with no tool attribution.
