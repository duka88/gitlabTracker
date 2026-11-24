#![allow(dead_code, clippy::unnecessary_wraps)]
//! Builders shared by the engine tests.

use std::collections::HashMap;

use chrono::{DateTime, Duration, TimeZone, Utc};
use gitlab_tracker_lib::engine::poll::PollRun;
use gitlab_tracker_lib::error::AppError;
use gitlab_tracker_lib::gitlab::lists::FetchResult;
use gitlab_tracker_lib::model::{
    Approvals, Label, ListItem, ListKind, MrFields, PersistedState, Person, PipelineStatus,
    Reviewer, Tab, TabSort, WorkItemFields, WorkStatus,
};

pub fn t0() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 3, 10, 0, 0).unwrap()
}

pub fn later(mins: i64) -> DateTime<Utc> {
    t0() + Duration::minutes(mins)
}

pub fn tab(id: &str, name: &str, kind: ListKind) -> Tab {
    let url = match kind {
        ListKind::MergeRequests => {
            format!("https://gitlab.com/groups/g/-/merge_requests?state=opened&x={id}")
        }
        ListKind::WorkItems => {
            format!("https://gitlab.com/groups/g/-/work_items?state=opened&x={id}")
        }
    };
    Tab {
        id: id.into(),
        name: name.into(),
        url,
        kind,
        notify_new_items: true,
        notify_new_comments: true,
        notify_changes: true,
        test_branch: None,
        group_by_week: false,
        sort: TabSort::Gitlab,
    }
}

pub fn person(u: &str) -> Person {
    Person {
        username: u.into(),
        name: format!("Name {u}"),
        avatar_url: None,
    }
}

pub fn mr(n: u32) -> ListItem {
    ListItem {
        gid: format!("gid://gitlab/MergeRequest/{n}"),
        kind: ListKind::MergeRequests,
        iid: n.to_string(),
        reference: format!("!{n}"),
        title: format!("MR {n}"),
        web_url: format!("https://gitlab.com/g/p/-/merge_requests/{n}"),
        project_path: "g/p".into(),
        author: Some(person("author")),
        created_at: t0(),
        updated_at: t0(),
        closed_at: None,
        labels: vec![],
        assignees: vec![],
        milestone: None,
        comment_count: Some(0),
        upvotes: 0,
        downvotes: 0,
        description_preview: String::new(),
        mr: Some(MrFields {
            draft: false,
            target_branch: "main".into(),
            state: "opened".into(),
            pipeline: Some(PipelineStatus::Success),
            approvals: Approvals {
                approved: false,
                required: 1,
                given: 0,
            },
            unresolved_threads: 0,
            source_branch: String::new(),
            head_sha: String::new(),
            on_branch: None,
            ticket: None,
            reviewers: vec![],
            merge_status: None,
        }),
        work_item: None,
        envs: Vec::new(),
    }
}

pub fn wi(n: u32) -> ListItem {
    ListItem {
        gid: format!("gid://gitlab/WorkItem/{n}"),
        kind: ListKind::WorkItems,
        iid: n.to_string(),
        reference: format!("#{n}"),
        title: format!("Ticket {n}"),
        web_url: format!("https://gitlab.com/g/p/-/work_items/{n}"),
        project_path: "g/p".into(),
        author: Some(person("author")),
        created_at: t0(),
        updated_at: t0(),
        closed_at: None,
        labels: vec![Label::new("state::In Testing", "#000", "#fff")],
        assignees: vec![],
        milestone: None,
        comment_count: Some(0),
        upvotes: 0,
        downvotes: 0,
        description_preview: String::new(),
        mr: None,
        work_item: Some(WorkItemFields {
            type_name: "Issue".into(),
            type_icon: None,
            confidential: false,
            iteration: None,
            due_date: None,
            weight: None,
            status: Some(WorkStatus {
                name: "To do".into(),
                color: None,
            }),
            health: None,
            children: None,
        }),
        envs: Vec::new(),
    }
}

pub fn with_reviewer(mut item: ListItem, user: &str, state: &str) -> ListItem {
    item.mr.as_mut().unwrap().reviewers.push(Reviewer {
        person: person(user),
        approved: state == "APPROVED",
        review_state: Some(state.into()),
    });
    item
}

pub fn ok(items: Vec<ListItem>) -> Result<FetchResult, AppError> {
    Ok(FetchResult {
        items,
        truncated_total: None,
        warning: None,
    })
}

/// Runs one poll's tab step (no mentions/comments) and returns the notification texts.
pub fn poll_tabs(
    state: &mut PersistedState,
    tabs: &[Tab],
    visible: Option<&str>,
    now: DateTime<Utc>,
    data: Vec<(&str, Vec<ListItem>)>,
) -> Vec<String> {
    let mut run = PollRun::new(tabs, Some("me"), visible, now);
    let fetched: HashMap<String, _> = data
        .into_iter()
        .map(|(id, items)| (id.to_string(), ok(items)))
        .collect();
    let (n, _) = run.apply_tabs(state, fetched);
    n.into_iter().map(|n| n.text).collect()
}
