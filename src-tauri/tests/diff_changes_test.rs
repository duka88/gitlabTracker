//! T077: change detection.
mod common;

use common::*;
use gitlab_tracker_lib::engine::diff::{diff_changes, tracked};
use gitlab_tracker_lib::engine::poll::PollRun;
use gitlab_tracker_lib::gitlab::merged::MergedInfo;
use gitlab_tracker_lib::model::{
    ChangeKind, Label, ListItem, ListKind, PersistedState, PipelineStatus, WorkStatus,
};
use pretty_assertions::assert_eq;

fn failed(mut i: ListItem) -> ListItem {
    i.mr.as_mut().unwrap().pipeline = Some(PipelineStatus::Failed);
    i
}
fn approved(mut i: ListItem) -> ListItem {
    i.mr.as_mut().unwrap().approvals.approved = true;
    i
}
fn status(mut i: ListItem, s: &str) -> ListItem {
    i.work_item.as_mut().unwrap().status = Some(WorkStatus {
        name: s.into(),
        color: None,
    });
    i
}

#[test]
fn each_change_kind() {
    let base = mr(1);
    let prev = tracked(&base);
    assert_eq!(
        diff_changes(&prev, &failed(mr(1)), Some("me")),
        vec![ChangeKind::PipelineFailed]
    );
    assert_eq!(
        diff_changes(&prev, &approved(mr(1)), Some("me")),
        vec![ChangeKind::Approved]
    );
    assert_eq!(
        diff_changes(
            &prev,
            &with_reviewer(mr(1), "ana", "REQUESTED_CHANGES"),
            Some("me")
        ),
        vec![ChangeKind::ChangesRequested {
            by_username: "ana".into(),
            by_name: "Name ana".into()
        }]
    );
    let mut merged = mr(1);
    merged.mr.as_mut().unwrap().state = "merged".into();
    assert_eq!(
        diff_changes(&prev, &merged, Some("me")),
        vec![ChangeKind::Merged]
    );
    assert_eq!(
        diff_changes(&prev, &mr(1), Some("me")),
        [] as [gitlab_tracker_lib::model::ChangeKind; 0]
    );

    let wprev = tracked(&wi(1));
    assert_eq!(
        diff_changes(&wprev, &status(wi(1), "In progress"), None),
        vec![ChangeKind::StatusChanged {
            from: Some("To do".into()),
            to: Some("In progress".into())
        }]
    );
    let mut relabeled = wi(1);
    relabeled.labels = vec![Label::new("state::Ready to Merge", "#000", "#fff")];
    assert_eq!(
        diff_changes(&wprev, &relabeled, None),
        vec![ChangeKind::LabelsChanged {
            added: vec!["state::Ready to Merge".into()],
            removed: vec!["state::In Testing".into()]
        }]
    );
}

#[test]
fn own_changes_requested_skipped() {
    let prev = tracked(&mr(1));
    assert_eq!(
        diff_changes(
            &prev,
            &with_reviewer(mr(1), "me", "REQUESTED_CHANGES"),
            Some("me")
        ),
        [] as [gitlab_tracker_lib::model::ChangeKind; 0]
    );
}

#[test]
fn new_and_changed_same_check_only_new() {
    let tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![])]);
    let n = poll_tabs(
        &mut st,
        &tabs,
        None,
        later(2),
        vec![("a", vec![failed(mr(1))])],
    );
    assert_eq!(n, vec!["New in A: !1 MR 1"]);
    assert!(st.tabs["a"].changes.is_empty());
}

#[test]
fn unchanged_state_no_repeat_and_fail_twice_notifies_twice() {
    let tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    assert_eq!(
        poll_tabs(
            &mut st,
            &tabs,
            None,
            later(2),
            vec![("a", vec![failed(mr(1))])]
        ),
        vec!["❌ Pipeline failed on !1"]
    );
    assert_eq!(
        poll_tabs(
            &mut st,
            &tabs,
            None,
            later(4),
            vec![("a", vec![failed(mr(1))])]
        ),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        poll_tabs(&mut st, &tabs, None, later(6), vec![("a", vec![mr(1)])]),
        [] as [std::string::String; 0]
    );
    assert_eq!(
        poll_tabs(
            &mut st,
            &tabs,
            None,
            later(8),
            vec![("a", vec![failed(mr(1))])]
        ),
        vec!["❌ Pipeline failed on !1"]
    );
    assert_eq!(
        st.tabs["a"].changes["gid://gitlab/MergeRequest/1"],
        vec![ChangeKind::PipelineFailed]
    );
}

#[test]
fn same_change_two_tabs_once() {
    let tabs = vec![
        tab("a", "A", ListKind::MergeRequests),
        tab("b", "B", ListKind::MergeRequests),
    ];
    let mut st = PersistedState::default();
    poll_tabs(
        &mut st,
        &tabs,
        None,
        t0(),
        vec![("a", vec![mr(1)]), ("b", vec![mr(1)])],
    );
    let n = poll_tabs(
        &mut st,
        &tabs,
        None,
        later(2),
        vec![("a", vec![approved(mr(1))]), ("b", vec![approved(mr(1))])],
    );
    assert_eq!(n, vec!["✅ !1 approved"]);
    assert!(
        st.tabs["b"]
            .changes
            .contains_key("gid://gitlab/MergeRequest/1")
    );
}

#[test]
fn visible_tab_change_no_notification() {
    let tabs = vec![tab("a", "A", ListKind::WorkItems)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, Some("a"), t0(), vec![("a", vec![wi(1)])]);
    let n = poll_tabs(
        &mut st,
        &tabs,
        Some("a"),
        later(2),
        vec![("a", vec![status(wi(1), "Doing")])],
    );
    assert_eq!(n, [] as [std::string::String; 0]);
    assert!(st.tabs["a"].changes.contains_key("gid://gitlab/WorkItem/1"));
    assert!(
        st.global
            .seen_while_visible
            .contains("gid://gitlab/WorkItem/1")
    );
}

#[test]
fn switch_off_no_change_markers() {
    let mut t = tab("a", "A", ListKind::MergeRequests);
    t.notify_changes = false;
    let tabs = vec![t];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    assert_eq!(
        poll_tabs(
            &mut st,
            &tabs,
            None,
            later(2),
            vec![("a", vec![failed(mr(1))])]
        ),
        [] as [std::string::String; 0]
    );
    assert!(st.tabs["a"].changes.is_empty());
}

#[test]
fn merged_check_notifies_others_merge_only() {
    let tabs = vec![tab("a", "My MRs", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(
        &mut st,
        &tabs,
        None,
        t0(),
        vec![("a", vec![mr(1), mr(2), mr(3)])],
    );
    let mut r = PollRun::new(&tabs, Some("me"), None, later(2));
    let (_, follow) = r.apply_tabs(&mut st, [("a".to_string(), ok(vec![]))].into());
    let mut cands = follow.merged.clone();
    cands.sort_by(|a, b| a.gid.cmp(&b.gid));
    assert_eq!(cands.len(), 3);
    let results = vec![
        Some(MergedInfo {
            state: "merged".into(),
            merge_user: Some("ana".into()),
        }),
        Some(MergedInfo {
            state: "merged".into(),
            merge_user: Some("me".into()),
        }),
        Some(MergedInfo {
            state: "closed".into(),
            merge_user: None,
        }),
    ];
    let n: Vec<String> = r
        .apply_merged(&cands, &results)
        .into_iter()
        .map(|n| n.text)
        .collect();
    assert_eq!(n, vec!["🔀 !1 merged"]);
}

#[test]
fn closed_list_has_no_merged_check() {
    let mut t = tab("a", "Closed", ListKind::MergeRequests);
    t.url = "https://gitlab.com/groups/g/-/merge_requests?state=all".into();
    let tabs = vec![t];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    let mut r = PollRun::new(&tabs, Some("me"), None, later(2));
    let (_, follow) = r.apply_tabs(&mut st, [("a".to_string(), ok(vec![]))].into());
    assert_eq!(
        follow.merged,
        [] as [gitlab_tracker_lib::engine::poll::MergedCandidate; 0]
    );
}
