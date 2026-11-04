//! T083: mentions.
mod common;

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use common::*;
use gitlab_tracker_lib::engine::poll::PollRun;
use gitlab_tracker_lib::gitlab::mentions::Mention;
use gitlab_tracker_lib::gitlab::notes::Note;
use gitlab_tracker_lib::model::{ListKind, NotificationKind, PersistedState};
use pretty_assertions::assert_eq;

fn mention(id: &str, at: DateTime<Utc>, gid: &str, note: &str) -> Mention {
    Mention {
        id: id.into(),
        author_username: "marko".into(),
        author_name: "Marko".into(),
        body: "@me please **check**".into(),
        created_at: at,
        target_ref: "#78".into(),
        target_url: Some("https://gitlab.com/g/p/-/work_items/78".into()),
        target_gid: Some(gid.into()),
        note_id: Some(note.into()),
    }
}

#[test]
fn baseline_then_new_mention_once() {
    let tabs = vec![tab("a", "A", ListKind::WorkItems)];
    let mut st = PersistedState::default();
    let old = mention("t1", later(-5), "gid://gitlab/WorkItem/78", "100");
    let n = PollRun::new(&tabs, Some("me"), None, t0())
        .apply_mentions(&mut st, std::slice::from_ref(&old));
    assert!(n.is_empty(), "first poll records a baseline");
    assert_eq!(st.global.mentions_baseline_at, Some(t0()));

    let new = mention("t2", later(1), "gid://gitlab/WorkItem/78", "101");
    let n = PollRun::new(&tabs, Some("me"), None, later(2))
        .apply_mentions(&mut st, &[new.clone(), old.clone()]);
    assert_eq!(n.len(), 1);
    assert_eq!(n[0].kind, NotificationKind::Mention);
    assert_eq!(n[0].text, "📣 Marko mentioned you on #78");
    assert_eq!(n[0].detail, "@me please check");
    assert_eq!(n[0].item_gid.as_deref(), Some("gid://gitlab/WorkItem/78"));
    assert!(st.global.seen_comment_ids.contains("101"));

    let n = PollRun::new(&tabs, Some("me"), None, later(4)).apply_mentions(&mut st, &[new, old]);
    assert!(n.is_empty(), "already seen");
}

#[test]
fn mention_wins_over_comment() {
    let tabs = vec![tab("a", "A", ListKind::WorkItems)];
    let mut st = PersistedState::default();
    let mut item = wi(78);
    item.comment_count = Some(1);
    poll_tabs(&mut st, &tabs, None, t0(), vec![("a", vec![item.clone()])]);
    PollRun::new(&tabs, Some("me"), None, t0()).apply_mentions(&mut st, &[]);

    let mut r = PollRun::new(&tabs, Some("me"), None, later(2));
    let m = r.apply_mentions(
        &mut st,
        &[mention("t2", later(1), "gid://gitlab/WorkItem/78", "555")],
    );
    assert_eq!(m.len(), 1);
    item.comment_count = Some(2);
    let (_, follow) = r.apply_tabs(&mut st, [("a".to_string(), ok(vec![item]))].into());
    let notes = HashMap::from([(
        "gid://gitlab/WorkItem/78".to_string(),
        vec![Note {
            id: "555".into(),
            body: "@me please check".into(),
            system: false,
            author_username: "marko".into(),
            author_name: "Marko".into(),
            created_at: later(1),
        }],
    )]);
    let c = r.apply_comments(&mut st, &follow.comments, &notes);
    assert!(c.is_empty(), "the mentioned comment doesn't notify twice");
}

#[test]
fn item_in_visible_tab_no_mention_notification() {
    let tabs = vec![tab("a", "A", ListKind::WorkItems)];
    let mut st = PersistedState::default();
    poll_tabs(&mut st, &tabs, Some("a"), t0(), vec![("a", vec![wi(78)])]);
    PollRun::new(&tabs, Some("me"), Some("a"), t0()).apply_mentions(&mut st, &[]);
    let n = PollRun::new(&tabs, Some("me"), Some("a"), later(2)).apply_mentions(
        &mut st,
        &[mention("t2", later(1), "gid://gitlab/WorkItem/78", "1")],
    );
    assert_eq!(
        n,
        [] as [gitlab_tracker_lib::engine::diff::PendingNotification; 0]
    );
    // Not visible: a different item still notifies.
    let n = PollRun::new(&tabs, Some("me"), Some("a"), later(4)).apply_mentions(
        &mut st,
        &[mention("t3", later(3), "gid://gitlab/WorkItem/99", "2")],
    );
    assert_eq!(n.len(), 1);
}

#[test]
fn user_change_resets_baseline() {
    let tabs = vec![tab("a", "A", ListKind::WorkItems)];
    let mut st = PersistedState::default();
    PollRun::new(&tabs, Some("me"), None, t0()).apply_mentions(&mut st, &[]);
    let n = PollRun::new(&tabs, Some("someone-else"), None, later(2)).apply_mentions(
        &mut st,
        &[mention("t9", later(1), "gid://gitlab/WorkItem/1", "3")],
    );
    assert_eq!(
        n,
        [] as [gitlab_tracker_lib::engine::diff::PendingNotification; 0]
    );
    assert_eq!(st.global.mentions_user.as_deref(), Some("someone-else"));
}
