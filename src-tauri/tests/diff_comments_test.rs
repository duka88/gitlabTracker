//! T071: comment candidates and note filtering.
#![allow(clippy::needless_pass_by_value)]
mod common;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use common::*;
use gitlab_tracker_lib::engine::poll::PollRun;
use gitlab_tracker_lib::gitlab::notes::Note;
use gitlab_tracker_lib::model::{ListItem, ListKind, PersistedState, Tab};
use pretty_assertions::assert_eq;

fn note(id: &str, author: &str, at: DateTime<Utc>, system: bool) -> Note {
    Note {
        id: id.into(),
        body: format!("comment {id}"),
        system,
        author_username: author.into(),
        author_name: format!("Name {author}"),
        created_at: at,
    }
}

/// Baseline at t0, then a poll at +2 with `items`; returns (notifications, state).
fn run(
    tabs: &[Tab],
    visible: Option<&str>,
    first: Vec<(&str, Vec<ListItem>)>,
    second: Vec<(&str, Vec<ListItem>)>,
    notes: HashMap<String, Vec<Note>>,
    st: &mut PersistedState,
) -> Vec<String> {
    poll_tabs(st, tabs, visible, t0(), first);
    let mut r = PollRun::new(tabs, Some("me"), visible, later(2));
    let fetched = second
        .into_iter()
        .map(|(id, items)| (id.to_string(), ok(items)))
        .collect();
    let (_, follow) = r.apply_tabs(st, fetched);
    r.apply_comments(st, &follow.comments, &notes)
        .into_iter()
        .map(|n| format!("{} | {}", n.text, n.detail))
        .collect()
}

fn mr_with_comments(n: u32, c: u32) -> ListItem {
    let mut i = mr(n);
    i.comment_count = Some(c);
    i
}

#[test]
fn count_up_queues_and_notifies_per_note() {
    let tabs = vec![tab("a", "Reviewing", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    let gid = "gid://gitlab/MergeRequest/1".to_string();
    let notes = HashMap::from([(
        gid.clone(),
        vec![
            note("1", "old", later(-10), false), // before baseline
            note("2", "bot", later(1), true),    // system
            note("3", "me", later(1), false),    // mine
            note("4", "marko", later(1), false), // new
            note("5", "ana", later(1), false),   // new
        ],
    )]);
    let n = run(
        &tabs,
        None,
        vec![("a", vec![mr_with_comments(1, 3)])],
        vec![("a", vec![mr_with_comments(1, 7)])],
        notes,
        &mut st,
    );
    assert_eq!(
        n,
        vec![
            "💬 Name marko on !1 | comment 4",
            "💬 Name ana on !1 | comment 5"
        ]
    );
    assert_eq!(st.tabs["a"].new_comments[&gid], 2);
    for id in ["1", "2", "3", "4", "5"] {
        assert!(st.global.seen_comment_ids.contains(id), "{id} marked seen");
    }
}

#[test]
fn unchanged_count_not_queued() {
    let tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    poll_tabs(
        &mut st,
        &tabs,
        None,
        t0(),
        vec![("a", vec![mr_with_comments(1, 3)])],
    );
    let mut r = PollRun::new(&tabs, Some("me"), None, later(2));
    let (_, follow) = r.apply_tabs(
        &mut st,
        [("a".to_string(), ok(vec![mr_with_comments(1, 3)]))].into(),
    );
    assert_eq!(
        follow.comments,
        [] as [gitlab_tracker_lib::engine::poll::CommentCandidate; 0]
    );
}

#[test]
fn work_item_update_queues_even_without_count_change() {
    let tabs = vec![tab("a", "A", ListKind::WorkItems)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![wi(1)])]);
    let mut updated = wi(1);
    updated.updated_at = later(1);
    let mut r = PollRun::new(&tabs, Some("me"), None, later(2));
    let (_, follow) = r.apply_tabs(&mut st, [("a".to_string(), ok(vec![updated]))].into());
    assert_eq!(
        follow.comments.len(),
        1,
        "reply inside an existing thread keeps the thread count"
    );
}

#[test]
fn already_seen_note_ignored() {
    let tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    st.global.seen_comment_ids.insert("4");
    let gid = "gid://gitlab/MergeRequest/1".to_string();
    let notes = HashMap::from([(gid, vec![note("4", "marko", later(1), false)])]);
    let n = run(
        &tabs,
        None,
        vec![("a", vec![mr_with_comments(1, 1)])],
        vec![("a", vec![mr_with_comments(1, 2)])],
        notes,
        &mut st,
    );
    assert_eq!(n, [] as [std::string::String; 0]);
}

#[test]
fn same_note_in_two_tabs_once() {
    let tabs = vec![
        tab("a", "A", ListKind::MergeRequests),
        tab("b", "B", ListKind::MergeRequests),
    ];
    let mut st = PersistedState::default();
    let gid = "gid://gitlab/MergeRequest/1".to_string();
    let notes = HashMap::from([(gid.clone(), vec![note("9", "marko", later(1), false)])]);
    let n = run(
        &tabs,
        None,
        vec![
            ("a", vec![mr_with_comments(1, 1)]),
            ("b", vec![mr_with_comments(1, 1)]),
        ],
        vec![
            ("a", vec![mr_with_comments(1, 2)]),
            ("b", vec![mr_with_comments(1, 2)]),
        ],
        notes,
        &mut st,
    );
    assert_eq!(n.len(), 1);
    assert_eq!(st.tabs["a"].new_comments[&gid], 1);
    assert_eq!(st.tabs["b"].new_comments[&gid], 1);
}

#[test]
fn visible_tab_comment_marks_but_no_notification() {
    let tabs = vec![
        tab("a", "A", ListKind::MergeRequests),
        tab("b", "B", ListKind::MergeRequests),
    ];
    let mut st = PersistedState::default();
    let gid = "gid://gitlab/MergeRequest/1".to_string();
    let notes = HashMap::from([(gid.clone(), vec![note("9", "marko", later(1), false)])]);
    let n = run(
        &tabs,
        Some("b"),
        vec![
            ("a", vec![mr_with_comments(1, 1)]),
            ("b", vec![mr_with_comments(1, 1)]),
        ],
        vec![
            ("a", vec![mr_with_comments(1, 2)]),
            ("b", vec![mr_with_comments(1, 2)]),
        ],
        notes,
        &mut st,
    );
    assert_eq!(n, [] as [std::string::String; 0]);
    assert_eq!(st.tabs["b"].new_comments[&gid], 1);
    assert!(st.global.seen_while_visible.contains(&gid));
    assert!(st.global.seen_comment_ids.contains("9"));
}

#[test]
fn first_sight_of_item_no_comment_notifications() {
    let tabs = vec![tab("a", "A", ListKind::MergeRequests)];
    let mut st = PersistedState::default();
    let gid = "gid://gitlab/MergeRequest/2".to_string();
    let notes = HashMap::from([(gid, vec![note("9", "marko", later(1), false)])]);
    let n = run(
        &tabs,
        None,
        vec![("a", vec![])],
        vec![("a", vec![mr_with_comments(2, 5)])],
        notes,
        &mut st,
    );
    assert!(
        n.is_empty(),
        "new item: only the NEW notification, comments not fetched"
    );
}

#[test]
fn switch_off_no_candidates() {
    let mut t = tab("a", "A", ListKind::MergeRequests);
    t.notify_new_comments = false;
    let tabs = vec![t];
    let mut st = PersistedState::default();
    poll_tabs(
        &mut st,
        &tabs,
        None,
        t0(),
        vec![("a", vec![mr_with_comments(1, 1)])],
    );
    let mut r = PollRun::new(&tabs, Some("me"), None, later(2));
    let (_, follow) = r.apply_tabs(
        &mut st,
        [("a".to_string(), ok(vec![mr_with_comments(1, 5)]))].into(),
    );
    assert_eq!(
        follow.comments,
        [] as [gitlab_tracker_lib::engine::poll::CommentCandidate; 0]
    );
}
