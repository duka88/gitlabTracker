//! Env chips (FR-040) for the cases that need no network: an MR whose branch is the deployed
//! ref, its ticket, and a ticket whose branch is deployed.

mod common;

use common::{mr, wi};
use gitlab_tracker_lib::gitlab::envs::flag_envs;
use gitlab_tracker_lib::gitlab::{GitLabClient, http_client};
use gitlab_tracker_lib::model::{
    EnvBadge, EnvDeployment, EnvStatus, LinkedTicket, ListItem, TrackedEnv,
};

fn deployed(name: &str, git_ref: &str, sha: &str) -> EnvStatus {
    let mut s = EnvStatus::empty(TrackedEnv::new("g/p", name));
    s.deployment = Some(EnvDeployment {
        id: 1,
        git_ref: git_ref.into(),
        sha: sha.into(),
        finished_at: None,
        user: None,
        job_url: None,
    });
    s
}

fn on_branch(mut item: ListItem, branch: &str, head: &str, ticket: Option<u32>) -> ListItem {
    let m = item.mr.as_mut().unwrap();
    m.source_branch = branch.into();
    m.head_sha = head.into();
    m.ticket = ticket.map(|n| LinkedTicket {
        reference: format!("#{n}"),
        title: String::new(),
        web_url: format!("https://gitlab.com/g/p/-/work_items/{n}"),
        open: true,
        state_label: None,
        status: None,
    });
    item
}

fn badge(env: &str, up_to_date: bool) -> EnvBadge {
    EnvBadge {
        env: env.into(),
        up_to_date,
    }
}

// The client is never called in these tests: every MR is on the env's deployed branch or in
// another project, so no ancestry check is needed.
fn offline() -> GitLabClient {
    GitLabClient::new(http_client(), "unused")
}

#[tokio::test]
async fn deployed_branch_flags_its_mr_and_ticket() {
    let mut current = on_branch(mr(1), "10-feature", "aaa", Some(10));
    let mut other_project = on_branch(mr(3), "30-x", "ccc", None);
    other_project.project_path = "g/elsewhere".into();
    let mut ticket10 = wi(10);
    let mut unrelated = wi(99);
    let statuses = vec![deployed("qa-1", "10-feature", "aaa")];
    let mut rows = vec![
        &mut current,
        &mut other_project,
        &mut ticket10,
        &mut unrelated,
    ];
    flag_envs(&offline(), &mut rows, &statuses).await.unwrap();

    assert_eq!(current.envs, vec![badge("qa-1", true)]);
    assert_eq!(other_project.envs, vec![]);
    assert_eq!(ticket10.envs, vec![badge("qa-1", true)]);
    assert_eq!(unrelated.envs, vec![]);
}

#[tokio::test]
async fn newer_commits_mark_outdated() {
    let mut stale = on_branch(mr(2), "20-other", "old", Some(20));
    let mut ticket20 = wi(20);
    let statuses = vec![deployed("qa-2", "20-other", "new")];
    let mut rows = vec![&mut stale, &mut ticket20];
    flag_envs(&offline(), &mut rows, &statuses).await.unwrap();
    assert_eq!(stale.envs, vec![badge("qa-2", false)]);
    // The ticket's branch is what's deployed, so the ticket itself is on qa-2.
    assert_eq!(ticket20.envs, vec![badge("qa-2", true)]);
}

#[tokio::test]
async fn badges_follow_settings_order() {
    let mut ticket = wi(7);
    let statuses = vec![
        deployed("qa-2", "7-other", "x"),
        deployed("qa-1", "7-a", "s1"),
    ];
    let mut rows = vec![&mut ticket];
    flag_envs(&offline(), &mut rows, &statuses).await.unwrap();
    assert_eq!(ticket.envs, vec![badge("qa-2", true), badge("qa-1", true)]);
}
