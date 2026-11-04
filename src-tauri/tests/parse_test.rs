//! Recorded GraphQL responses (T002 fixtures) → `ListItem`s.
use gitlab_tracker_lib::gitlab::parse::{parse_mr_page, parse_work_item_page};
use gitlab_tracker_lib::gitlab::user::parse_current_user;
use gitlab_tracker_lib::model::{Children, Health, ListKind, MergeStatusTag, PipelineStatus};
use pretty_assertions::assert_eq;
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/{name}.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn current_user() {
    let u = parse_current_user(&fixture("current_user")["data"]).unwrap();
    assert_eq!(u.username, "DusanJ88");
    assert_eq!(u.id, "gid://gitlab/User/10551939");
    let avatar = u.avatar_url.unwrap();
    assert!(
        avatar.starts_with("avatar://localhost/https%3A%2F%2Fgitlab.com%2Fuploads"),
        "{avatar}"
    );
    assert!(parse_current_user(&serde_json::json!({ "currentUser": null })).is_err());
}

#[test]
fn authored_mrs() {
    let d = fixture("mrs_dashboard_authored");
    let (items, page, count) = parse_mr_page(&d["data"]["user"]["authored"]).unwrap();
    assert_eq!(items.len(), 5);
    assert_eq!(count, Some(5));
    assert!(!page.has_next_page);

    let first = &items[0];
    assert_eq!(first.kind, ListKind::MergeRequests);
    assert_eq!(first.reference, "!19717");
    assert!(first.gid.starts_with("gid://gitlab/MergeRequest/"));
    assert_eq!(first.project_path, "login5/login-eko/login-eko");
    let mr = first.mr.as_ref().unwrap();
    assert_eq!(mr.merge_status, Some(MergeStatusTag::ReadyToMerge));
    assert_eq!(mr.pipeline, Some(PipelineStatus::Success));
    assert_eq!((mr.approvals.given, mr.approvals.required), (1, 1));
    assert!(mr.reviewers[0].approved);
    assert_eq!(mr.reviewers[0].review_state.as_deref(), Some("APPROVED"));

    let tags: Vec<_> = items
        .iter()
        .map(|i| i.mr.as_ref().unwrap().merge_status)
        .collect();
    assert_eq!(
        tags,
        vec![
            Some(MergeStatusTag::ReadyToMerge),
            None, // draft
            Some(MergeStatusTag::Conflicts),
            // conflicts: true wins over detailedMergeStatus NOT_APPROVED (R5 order).
            Some(MergeStatusTag::Conflicts),
            None, // draft with failed pipeline
        ]
    );
    assert!(items[1].mr.as_ref().unwrap().draft);
    assert_eq!(
        items[4].mr.as_ref().unwrap().pipeline,
        Some(PipelineStatus::Failed)
    );
    assert_eq!(items[3].comment_count, Some(772));
}

#[test]
fn reviewing_mrs_avatars_and_labels() {
    let d = fixture("mrs_dashboard_reviewing");
    let (items, _, _) = parse_mr_page(&d["data"]["user"]["reviewing"]).unwrap();
    assert_eq!(items.len(), 4);
    for item in &items {
        for p in item.assignees.iter().chain(item.author.iter()) {
            if let Some(a) = &p.avatar_url {
                assert!(a.starts_with("avatar://localhost/"), "{a}");
            }
        }
        for l in &item.labels {
            assert_ne!(l.color, "");
            if l.title.contains("::") {
                assert!(l.scope.is_some());
            }
        }
    }
    let me = &items[1].mr.as_ref().unwrap().reviewers[0];
    assert_eq!(me.person.username, "DusanJ88");
    assert!(me.approved);
}

#[test]
fn description_preview_is_plain_and_cut() {
    let d = fixture("mrs_dashboard_reviewing");
    let (items, _, _) = parse_mr_page(&d["data"]["user"]["reviewing"]).unwrap();
    let p = &items[0].description_preview;
    assert!(p.chars().count() <= 300, "{}", p.chars().count());
    assert!(p.ends_with('…'));
    assert!(!p.contains("**") && !p.contains("](") && !p.contains('`'));
    assert!(p.starts_with("Goal: make the example feature work"));
}

#[test]
fn project_mrs_page_info() {
    let d = fixture("mrs_project_page1");
    let (items, page, count) = parse_mr_page(&d["data"]["project"]["mergeRequests"]).unwrap();
    assert_eq!(items.len(), 2);
    assert_eq!(count, Some(114));
    assert!(page.has_next_page);
    assert!(page.end_cursor.is_some());
}

#[test]
fn group_work_items() {
    let d = fixture("work_items_group");
    let (items, _, _) = parse_work_item_page(&d["data"]["group"]["workItems"]).unwrap();
    assert_ne!(items, [] as [gitlab_tracker_lib::model::ListItem; 0]);
    for i in &items {
        assert_eq!(i.kind, ListKind::WorkItems);
        assert!(i.gid.starts_with("gid://gitlab/WorkItem/"));
        assert!(i.reference.starts_with('#'));
        let wi = i.work_item.as_ref().unwrap();
        assert_eq!(wi.type_name, "Issue");
        assert_eq!(wi.status.as_ref().unwrap().name, "To do");
        assert_eq!(wi.children, None, "0 children → no progress");
        assert!(i.assignees.iter().any(|a| a.username == "DusanJ88"));
    }
    assert!(items.iter().any(|i| i.milestone.is_some()));
}

#[test]
fn synthetic_work_item_every_widget() {
    let d = fixture("work_items_synthetic");
    let (items, _, _) = parse_work_item_page(&d["data"]["project"]["workItems"]).unwrap();
    let i = &items[0];
    let wi = i.work_item.as_ref().unwrap();
    assert_eq!(i.reference, "#45");
    assert_eq!(i.comment_count, Some(7));
    assert_eq!((i.upvotes, i.downvotes), (2, 1));
    assert_eq!(i.milestone.as_deref(), Some("5.23.x"));
    assert_eq!(i.description_preview, "Short description.");
    assert_eq!(i.project_path, "login5/x");
    assert_eq!(wi.type_name, "Task");
    assert!(wi.confidential);
    assert_eq!(wi.iteration.as_deref(), Some("Sprints · Nov 3 – Nov 16"));
    assert_eq!(wi.due_date.unwrap().to_string(), "2026-11-10");
    assert_eq!(wi.weight, Some(3));
    assert_eq!(wi.status.as_ref().unwrap().name, "In progress");
    assert_eq!(wi.health, Some(Health::NeedsAttention));
    assert_eq!(wi.children, Some(Children { done: 2, total: 5 }));
    // Gravatar is still wrapped; the avatar:// handler refuses non-GitLab hosts.
    assert!(
        i.author
            .as_ref()
            .unwrap()
            .avatar_url
            .as_ref()
            .unwrap()
            .starts_with("avatar://localhost/https%3A%2F%2Fsecure.gravatar.com")
    );
    assert_eq!(i.labels[0].scope.as_deref(), Some("state"));
    assert_eq!(i.labels[0].value, "In Development");
}

#[test]
fn notes_batch_from_fixtures() {
    use gitlab_tracker_lib::gitlab::notes::{NoteTarget, build_query, parse_batch};
    let mr = vec![NoteTarget::Mr {
        project_path: "login5/login-eko/login-eko".into(),
        iid: "19540".into(),
    }];
    let notes = parse_batch(&mr, &fixture("mr_notes")["data"]);
    assert_eq!(notes[0].len(), 6);
    assert!(
        notes[0]
            .iter()
            .all(|n| n.id.chars().all(|c| c.is_ascii_digit()))
    );
    assert!(
        notes[0]
            .windows(2)
            .all(|w| w[0].created_at <= w[1].created_at),
        "oldest first"
    );
    let wi = vec![NoteTarget::WorkItem {
        gid: "gid://gitlab/WorkItem/186100809".into(),
    }];
    let notes = parse_batch(&wi, &fixture("work_item_notes")["data"]);
    assert_eq!(notes[0].len(), 6);
    assert_ne!(notes[0][0].author_username, "");
    // Query text uses variables, never inlined values.
    let (q, vars) = build_query(&[mr[0].clone(), wi[0].clone()]);
    assert!(q.contains("n0: project(fullPath: $p0)") && q.contains("n1: workItem(id: $w1)"));
    assert!(!q.contains("19540"));
    assert_eq!(vars["i0"], "19540");
    // Missing item → empty list.
    assert_eq!(
        parse_batch(&mr, &serde_json::json!({ "n0": null }))[0].len(),
        0
    );
}

#[test]
fn merged_check_from_fixture() {
    use gitlab_tracker_lib::gitlab::merged::parse_batch;
    let r = parse_batch(3, &fixture("merged_check")["data"]);
    let m0 = r[0].as_ref().unwrap();
    assert_eq!(m0.state, "merged");
    assert_eq!(m0.merge_user.as_deref(), Some("colleague12"));
    assert_eq!(r[1].as_ref().unwrap().state, "opened");
    assert!(r[2].is_none());
}

#[test]
fn mentions_from_fixture() {
    use gitlab_tracker_lib::gitlab::mentions::parse_mentions;
    let m = parse_mentions(&fixture("todos")["data"]);
    assert_eq!(m.len(), 5);
    for x in &m {
        assert!(x.id.starts_with("gid://gitlab/Todo/"));
        if let Some(g) = &x.target_gid {
            assert!(!g.contains("/Issue/"), "normalised: {g}");
        }
        if let Some(n) = &x.note_id {
            assert!(n.chars().all(|c| c.is_ascii_digit()));
        }
    }
    assert!(m.iter().any(|x| {
        x.target_gid
            .as_deref()
            .is_some_and(|g| g.starts_with("gid://gitlab/WorkItem/"))
    }));
    assert!(m.iter().any(|x| x.target_ref.starts_with('!')));
}
