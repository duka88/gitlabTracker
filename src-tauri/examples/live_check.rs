//! Dev check against gitlab.com: `GITLAB_TOKEN=... cargo run --example live_check`.
//! Fetches the default tabs, mentions, a notes batch, a merged check and the tracked envs
//! (plus `EXTRA_ENVS=qa-5,stage` in the default env project), printing a summary.
//! Read-only (`read_api`); prints no secrets.

use gitlab_tracker_lib::gitlab::envs::{fetch_env, flag_envs};
use gitlab_tracker_lib::gitlab::lists::fetch_tab;
use gitlab_tracker_lib::gitlab::mentions::fetch_mentions;
use gitlab_tracker_lib::gitlab::merged::check_merged;
use gitlab_tracker_lib::gitlab::notes::{NoteTarget, fetch_notes};
use gitlab_tracker_lib::gitlab::user::fetch_current_user;
use gitlab_tracker_lib::gitlab::{GitLabClient, http_client};
use gitlab_tracker_lib::model::{
    DEFAULT_ENV_PROJECT, ListItem, ListKind, TrackedEnv, default_tabs, default_tracked_envs,
};
use gitlab_tracker_lib::url_map::parse_tab_url;

#[tokio::main]
async fn main() {
    let token = std::env::var("GITLAB_TOKEN").expect("set GITLAB_TOKEN");
    let client = GitLabClient::new(http_client(), token);
    let me = fetch_current_user(&client).await.expect("current user");
    println!("signed in as @{}", me.username);

    let mut targets = Vec::new();
    let mut mrs = Vec::new();
    let mut all: Vec<ListItem> = Vec::new();
    for tab in default_tabs() {
        let q = parse_tab_url(&tab.url).expect("default URL parses");
        match fetch_tab(&client, &q).await {
            Ok(r) => {
                println!(
                    "{:<16} {:>3} items  warning={:?}  truncated={:?}",
                    tab.name,
                    r.items.len(),
                    r.warning.map(|w| w.message),
                    r.truncated_total
                );
                for i in r.items.iter().take(2) {
                    println!(
                        "    {} comments={:?} labels={}",
                        i.reference,
                        i.comment_count,
                        i.labels.len()
                    );
                }
                all.extend(r.items.iter().cloned());
                if let Some(i) = r.items.first() {
                    targets.push(match tab.kind {
                        ListKind::MergeRequests => NoteTarget::Mr {
                            project_path: i.project_path.clone(),
                            iid: i.iid.clone(),
                        },
                        ListKind::WorkItems => NoteTarget::WorkItem { gid: i.gid.clone() },
                    });
                    if tab.kind == ListKind::MergeRequests {
                        mrs.push((i.project_path.clone(), i.iid.clone()));
                    }
                }
            }
            Err(e) => println!("{:<16} ERROR {e}", tab.name),
        }
    }
    let notes = fetch_notes(&client, &targets).await.expect("notes");
    println!(
        "notes per target: {:?}",
        notes.iter().map(Vec::len).collect::<Vec<_>>()
    );
    let merged = check_merged(&client, &mrs).await.expect("merged");
    println!(
        "merged check: {:?}",
        merged
            .iter()
            .map(|m| m.as_ref().map(|m| m.state.clone()))
            .collect::<Vec<_>>()
    );
    let mentions = fetch_mentions(&client).await.expect("mentions");
    println!(
        "mentions: {} (first target {:?})",
        mentions.len(),
        mentions.first().map(|m| &m.target_gid)
    );

    // Tracked envs (FR-040).
    let mut envs = default_tracked_envs();
    if let Ok(extra) = std::env::var("EXTRA_ENVS") {
        envs.extend(
            extra
                .split(',')
                .map(|n| TrackedEnv::new(DEFAULT_ENV_PROJECT, n.trim())),
        );
    }
    let mut statuses = Vec::new();
    for e in &envs {
        let s = fetch_env(&client, e, chrono::Utc::now()).await;
        println!(
            "env {:<12} state={:?} ref={:?} sha={:?} error={:?}",
            e.name,
            s.state,
            s.deployment.as_ref().map(|d| &d.git_ref),
            s.deployment.as_ref().map(|d| &d.sha[..8]),
            s.error
        );
        statuses.push(s);
    }
    let mut rows: Vec<&mut ListItem> = all.iter_mut().collect();
    flag_envs(&client, &mut rows, &statuses)
        .await
        .expect("env flags");
    for i in all.iter().filter(|i| !i.envs.is_empty()) {
        println!("    {} {:?}", i.reference, i.envs);
    }
}
