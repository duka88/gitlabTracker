//! T059: tab badges, menu count, `seen_while_visible` pruning.
mod common;

use common::*;
use gitlab_tracker_lib::engine::poll::{clear_tab_markers, recompute_counts, tab_new_count};
use gitlab_tracker_lib::model::{ListKind, PersistedState};
use pretty_assertions::assert_eq;

fn setup() -> (Vec<gitlab_tracker_lib::model::Tab>, PersistedState) {
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
    (tabs, st)
}

#[test]
fn badge_excludes_seen_while_visible_in_other_tabs() {
    let (tabs, mut st) = setup();
    // gid 1 appears in visible tab a and hidden tab b; gid 2 only in b.
    poll_tabs(
        &mut st,
        &tabs,
        Some("a"),
        later(2),
        vec![("a", vec![mr(1)]), ("b", vec![mr(1), mr(2)])],
    );
    assert_eq!(tab_new_count(&st, "a"), 0);
    assert_eq!(tab_new_count(&st, "b"), 1, "only gid 2 counts in b");
    assert_eq!(recompute_counts(&mut st, &tabs), 1);
}

#[test]
fn pruned_once_no_tab_has_marker() {
    let (tabs, mut st) = setup();
    poll_tabs(
        &mut st,
        &tabs,
        Some("a"),
        later(2),
        vec![("a", vec![mr(1)]), ("b", vec![mr(1)])],
    );
    assert!(
        st.global
            .seen_while_visible
            .contains("gid://gitlab/MergeRequest/1")
    );
    clear_tab_markers(&mut st, "a");
    assert!(
        st.global
            .seen_while_visible
            .contains("gid://gitlab/MergeRequest/1"),
        "b still has a marker"
    );
    clear_tab_markers(&mut st, "b");
    assert!(st.global.seen_while_visible.is_empty());
    assert_eq!(recompute_counts(&mut st, &tabs), 0);
}

#[test]
fn menu_count_is_distinct_and_unchanged_on_hide() {
    let (tabs, mut st) = setup();
    poll_tabs(
        &mut st,
        &tabs,
        None,
        later(2),
        vec![("a", vec![mr(1), mr(2)]), ("b", vec![mr(2), mr(3)])],
    );
    assert_eq!(recompute_counts(&mut st, &tabs), 3);
    // Window show/hide alone changes nothing; a poll with nothing new keeps the count.
    poll_tabs(
        &mut st,
        &tabs,
        None,
        later(4),
        vec![("a", vec![mr(1), mr(2)]), ("b", vec![mr(2), mr(3)])],
    );
    assert_eq!(recompute_counts(&mut st, &tabs), 3);
    clear_tab_markers(&mut st, "a");
    assert_eq!(
        recompute_counts(&mut st, &tabs),
        2,
        "gid 2 still marked in b, gid 3 in b"
    );
}

#[test]
fn mentions_never_counted() {
    use gitlab_tracker_lib::engine::poll::PollRun;
    use gitlab_tracker_lib::gitlab::mentions::Mention;
    let (tabs, mut st) = setup();
    let m = |id: &str, at| Mention {
        id: id.into(),
        author_username: "x".into(),
        author_name: "X".into(),
        body: "@me hi".into(),
        created_at: at,
        target_ref: "#78".into(),
        target_url: None,
        target_gid: Some("gid://gitlab/WorkItem/78".into()),
        note_id: Some("1".into()),
    };
    let mut run = PollRun::new(&tabs, Some("me"), None, t0());
    run.apply_mentions(&mut st, &[]);
    let mut run = PollRun::new(&tabs, Some("me"), None, later(2));
    let n = run.apply_mentions(&mut st, &[m("t1", later(1))]);
    assert_eq!(n.len(), 1);
    assert_eq!(recompute_counts(&mut st, &tabs), 0);
}
