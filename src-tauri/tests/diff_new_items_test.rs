//! T057: new-item diff, cross-tab de-duplication, the visible-tab rule, URL reset.
mod common;

use common::*;
use gitlab_tracker_lib::engine::poll::{recompute_counts, tab_new_count};
use gitlab_tracker_lib::model::{ListKind, NotificationKind, PersistedState};
use pretty_assertions::assert_eq;

#[test]
fn first_load_is_baseline_only() {
    let tabs = vec![tab("a", "Reviewing", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    let n = poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1), mr(2)])]);
    assert_eq!(n, [] as [std::string::String; 0]);
    let ts = &st.tabs["a"];
    assert!(ts.baseline_done);
    assert_eq!(ts.baseline_at, Some(t0()));
    assert!(ts.new_items.is_empty());
    assert_eq!(ts.items.len(), 2);
}

#[test]
fn one_added_item_one_notification() {
    let tabs = vec![tab("a", "Reviewing", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    let n = poll_tabs(
        &mut st,
        &tabs,
        None,
        later(2),
        vec![("a", vec![mr(2), mr(1)])],
    );
    assert_eq!(n, vec!["New in Reviewing: !2 MR 2"]);
    assert!(
        st.tabs["a"]
            .new_items
            .contains("gid://gitlab/MergeRequest/2")
    );
    assert_eq!(recompute_counts(&mut st, &tabs), 1);
}

#[test]
fn several_added_items_one_summary() {
    let tabs = vec![tab("a", "My tickets", ListKind::WorkItems)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![wi(1)])]);
    let mut run = gitlab_tracker_lib::engine::poll::PollRun::new(&tabs, Some("me"), None, later(2));
    let fetched = [("a".to_string(), ok(vec![wi(1), wi(2), wi(3), wi(4)]))]
        .into_iter()
        .collect();
    let (n, _) = run.apply_tabs(&mut st, fetched);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].kind, NotificationKind::NewItems);
    assert_eq!(n[0].text, "3 new in My tickets");
    assert_eq!(n[0].tab_id.as_deref(), Some("a"));
    assert_eq!(n[0].item_gid, None);
    assert_eq!(st.tabs["a"].new_items.len(), 3);
}

#[test]
fn switch_off_markers_but_no_notification() {
    let mut t = tab("a", "Quiet", ListKind::MergeRequests);
    t.notify_new_items = false;
    let tabs = vec![t];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![])]);
    let n = poll_tabs(&mut st, &tabs, None, later(2), vec![("a", vec![mr(5)])]);
    assert_eq!(n, [] as [std::string::String; 0]);
    assert_eq!(st.tabs["a"].new_items.len(), 1);
    assert_eq!(recompute_counts(&mut st, &tabs), 1);
}

#[test]
fn same_gid_in_two_tabs_notifies_once() {
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
        vec![("a", vec![]), ("b", vec![])],
    );
    let n = poll_tabs(
        &mut st,
        &tabs,
        None,
        later(2),
        vec![("a", vec![mr(7)]), ("b", vec![mr(7)])],
    );
    assert_eq!(n, vec!["New in A: !7 MR 7"]);
    assert_eq!(tab_new_count(&st, "a"), 1);
    assert_eq!(tab_new_count(&st, "b"), 1);
    assert_eq!(
        recompute_counts(&mut st, &tabs),
        1,
        "menu count is distinct gids"
    );
}

#[test]
fn item_leaves_and_returns_notifies_again() {
    let tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    assert_eq!(
        poll_tabs(&mut st, &tabs, None, later(2), vec![("a", vec![])]),
        [] as [std::string::String; 0]
    );
    let n = poll_tabs(&mut st, &tabs, None, later(4), vec![("a", vec![mr(1)])]);
    assert_eq!(n, vec!["New in A: !1 MR 1"]);
}

#[test]
fn visible_and_hidden_tab_same_item_no_notification_no_badge() {
    // Visible tab "b" is listed second but compared first.
    let tabs = vec![
        tab("a", "Hidden", ListKind::MergeRequests),
        tab("b", "Visible", ListKind::MergeRequests),
    ];
    let mut st = PersistedState::default();
    poll_tabs(
        &mut st,
        &tabs,
        Some("b"),
        t0(),
        vec![("a", vec![]), ("b", vec![])],
    );
    let n = poll_tabs(
        &mut st,
        &tabs,
        Some("b"),
        later(2),
        vec![("a", vec![mr(9)]), ("b", vec![mr(9)])],
    );
    assert_eq!(n, [] as [std::string::String; 0]);
    assert!(
        st.tabs["a"]
            .new_items
            .contains("gid://gitlab/MergeRequest/9")
    );
    assert!(
        st.tabs["b"]
            .new_items
            .contains("gid://gitlab/MergeRequest/9")
    );
    assert_eq!(tab_new_count(&st, "a"), 0);
    assert_eq!(tab_new_count(&st, "b"), 0);
    assert_eq!(recompute_counts(&mut st, &tabs), 0);
}

#[test]
fn visible_tab_rule_persists_after_hide() {
    let tabs = vec![tab("a", "My MRs", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, Some("a"), t0(), vec![("a", vec![])]);
    let n = poll_tabs(
        &mut st,
        &tabs,
        Some("a"),
        later(2),
        vec![("a", vec![mr(3)])],
    );
    assert!(n.is_empty(), "no notification for the visible tab");
    assert!(
        st.tabs["a"]
            .new_items
            .contains("gid://gitlab/MergeRequest/3"),
        "marker still set"
    );
    assert!(
        st.global
            .seen_while_visible
            .contains("gid://gitlab/MergeRequest/3")
    );
    assert_eq!(recompute_counts(&mut st, &tabs), 0);
    // Window hides; next poll with the same item: still not counted, no notification.
    let n = poll_tabs(&mut st, &tabs, None, later(4), vec![("a", vec![mr(3)])]);
    assert_eq!(n, [] as [std::string::String; 0]);
    assert_eq!(recompute_counts(&mut st, &tabs), 0);
    assert_eq!(tab_new_count(&st, "a"), 0);
}

#[test]
fn url_change_resets_baseline() {
    let mut tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    tabs[0].url = "https://gitlab.com/groups/other/-/merge_requests?state=opened".into();
    let n = poll_tabs(
        &mut st,
        &tabs,
        None,
        later(2),
        vec![("a", vec![mr(1), mr(2)])],
    );
    assert!(n.is_empty(), "a changed URL loads a new baseline");
    assert!(st.tabs["a"].new_items.is_empty());
    assert_eq!(st.tabs["a"].baseline_at, Some(later(2)));
}

#[test]
fn error_keeps_last_items() {
    let tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    let mut run = gitlab_tracker_lib::engine::poll::PollRun::new(&tabs, Some("me"), None, later(2));
    let fetched = [(
        "a".to_string(),
        Err(gitlab_tracker_lib::error::AppError::NotFound(
            "Group not found".into(),
        )),
    )]
    .into_iter()
    .collect();
    run.apply_tabs(&mut st, fetched);
    assert_eq!(st.tabs["a"].items.len(), 1);
    assert_eq!(
        st.tabs["a"].error.as_ref().unwrap().message,
        "Group not found"
    );
    // Next success clears it.
    poll_tabs(&mut st, &tabs, None, later(4), vec![("a", vec![mr(1)])]);
    assert!(st.tabs["a"].error.is_none());
}

/// FR-034: a tab with a test branch keeps its baseline between polls (its fingerprint
/// includes the branch), so an MR newly merged into the test branch is NEW and notifies.
#[test]
fn test_branch_tab_keeps_baseline_and_notifies() {
    let mut t = tab("a", "On test", ListKind::MergeRequests);
    t.test_branch = Some("resource-test".into());
    let tabs = vec![t];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![mr(1)])]);
    assert!(st.tabs["a"].baseline_done);
    let n = poll_tabs(
        &mut st,
        &tabs,
        None,
        later(2),
        vec![("a", vec![mr(2), mr(1)])],
    );
    assert_eq!(n, vec!["New in On test: !2 MR 2"]);
}
