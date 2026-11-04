//! Dev check against gitlab.com: `GITLAB_TOKEN=... cargo run --example live_check`.
//! Fetches the default tabs, mentions, a notes batch and a merged check, printing a summary.
//! Read-only (`read_api`); prints no secrets.

use gitlab_tracker_lib::gitlab::lists::fetch_tab;
use gitlab_tracker_lib::gitlab::mentions::fetch_mentions;
use gitlab_tracker_lib::gitlab::merged::check_merged;
use gitlab_tracker_lib::gitlab::notes::{NoteTarget, fetch_notes};
use gitlab_tracker_lib::gitlab::user::fetch_current_user;
use gitlab_tracker_lib::gitlab::{GitLabClient, http_client};
use gitlab_tracker_lib::model::{ListKind, default_tabs};
use gitlab_tracker_lib::url_map::parse_tab_url;

#[tokio::main]
async fn main() {
    let token = std::env::var("GITLAB_TOKEN").expect("set GITLAB_TOKEN");
    let client = GitLabClient::new(http_client(), token);
    let me = fetch_current_user(&client).await.expect("current user");
    println!("signed in as @{}", me.username);

    let mut targets = Vec::new();
    let mut mrs = Vec::new();
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
}
