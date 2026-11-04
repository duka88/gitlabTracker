//! Every row of the research R5 merge-status derivation table.
#![allow(clippy::needless_pass_by_value)]
use gitlab_tracker_lib::gitlab::parse::{
    MrRaw, effectively_approved, merge_status, mr_to_item, pick_ticket, ticket_blocks_merge,
};
use gitlab_tracker_lib::model::MergeStatusTag as T;
use serde_json::json;

fn mr(v: serde_json::Value) -> MrRaw {
    let mut base = json!({
        "id": "gid://gitlab/MergeRequest/1", "iid": "1", "state": "opened", "draft": false,
        "conflicts": false, "approved": true, "detailedMergeStatus": "MERGEABLE",
        "headPipeline": { "status": "SUCCESS" }, "reviewers": { "nodes": [] },
        "approvalsRequired": 0, "approvalsLeft": 0,
        "approvedBy": { "nodes": [ { "username": "approver" } ] },
        "resolvableDiscussionsCount": 2, "resolvedDiscussionsCount": 2
    });
    for (k, val) in v.as_object().unwrap() {
        base[k] = val.clone();
    }
    serde_json::from_value(base).unwrap()
}

fn reviewer(state: &str) -> serde_json::Value {
    json!({ "nodes": [ { "username": "r", "name": "R", "mergeRequestInteraction": { "approved": false, "reviewState": state } } ] })
}

#[test]
fn not_open_or_draft_has_no_tag() {
    assert_eq!(merge_status(&mr(json!({ "state": "merged" }))), None);
    assert_eq!(
        merge_status(&mr(json!({ "state": "closed", "conflicts": true }))),
        None
    );
    assert_eq!(
        merge_status(&mr(json!({ "draft": true, "conflicts": true }))),
        None
    );
}

#[test]
fn conflicts_first() {
    assert_eq!(
        merge_status(&mr(
            json!({ "conflicts": true, "headPipeline": { "status": "FAILED" } })
        )),
        Some(T::Conflicts)
    );
    assert_eq!(
        merge_status(&mr(json!({ "detailedMergeStatus": "CONFLICT" }))),
        Some(T::Conflicts)
    );
}

#[test]
fn ci_failed() {
    assert_eq!(
        merge_status(&mr(
            json!({ "headPipeline": { "status": "FAILED" }, "reviewers": reviewer("REQUESTED_CHANGES") })
        )),
        Some(T::CiFailed)
    );
    assert_eq!(
        merge_status(&mr(
            json!({ "detailedMergeStatus": "CI_MUST_PASS", "headPipeline": { "status": "FAILED" } })
        )),
        Some(T::CiFailed)
    );
    // CI must pass but the pipeline is still running → no tag.
    assert_eq!(
        merge_status(&mr(
            json!({ "detailedMergeStatus": "CI_MUST_PASS", "headPipeline": { "status": "RUNNING" } })
        )),
        None
    );
}

#[test]
fn changes_requested() {
    assert_eq!(
        merge_status(&mr(json!({ "reviewers": reviewer("REQUESTED_CHANGES") }))),
        Some(T::ChangesRequested)
    );
    assert_eq!(
        merge_status(&mr(json!({ "detailedMergeStatus": "REQUESTED_CHANGES" }))),
        Some(T::ChangesRequested)
    );
}

#[test]
fn unresolved_threads() {
    assert_eq!(
        merge_status(&mr(
            json!({ "detailedMergeStatus": "DISCUSSIONS_NOT_RESOLVED", "approved": false })
        )),
        Some(T::UnresolvedThreads)
    );
}

#[test]
fn needs_approval() {
    assert_eq!(
        merge_status(&mr(
            json!({ "approved": false, "detailedMergeStatus": "UNCHECKED" })
        )),
        Some(T::NeedsApproval)
    );
    assert_eq!(
        merge_status(&mr(json!({ "detailedMergeStatus": "NOT_APPROVED" }))),
        Some(T::NeedsApproval)
    );
}

#[test]
fn ready_to_merge() {
    assert_eq!(merge_status(&mr(json!({}))), Some(T::ReadyToMerge));
}

#[test]
fn otherwise_none() {
    for s in ["CI_STILL_RUNNING", "CHECKING", "UNCHECKED", "NEED_REBASE"] {
        assert_eq!(
            merge_status(&mr(json!({ "detailedMergeStatus": s }))),
            None,
            "{s}"
        );
    }
}

/// FR-025a: "Ready to merge" needs at least one real approval, even with no approval rules.
#[test]
fn no_approver_is_not_ready() {
    let m = mr(json!({ "approvedBy": { "nodes": [] } }));
    assert!(!effectively_approved(&m));
    assert_eq!(merge_status(&m), Some(T::NeedsApproval));
}

/// FR-025a: with rules, every required approval must be given.
#[test]
fn approvals_must_cover_required() {
    let partial = mr(json!({
        "approved": false, "approvalsRequired": 2, "approvalsLeft": 1,
        "approvedBy": { "nodes": [ { "username": "a" } ] }
    }));
    assert_eq!(merge_status(&partial), Some(T::NeedsApproval));
    let full = mr(json!({
        "approvalsRequired": 2, "approvalsLeft": 0,
        "approvedBy": { "nodes": [ { "username": "a" }, { "username": "b" } ] }
    }));
    assert_eq!(merge_status(&full), Some(T::ReadyToMerge));
}

/// FR-025b: open threads block "Ready to merge" even when the project doesn't require
/// resolving them, and the row carries the count.
#[test]
fn open_threads_show_even_when_mergeable() {
    let m = mr(json!({ "resolvableDiscussionsCount": 3, "resolvedDiscussionsCount": 1 }));
    assert_eq!(merge_status(&m), Some(T::UnresolvedThreads));
    assert_eq!(mr_to_item(&m).mr.unwrap().unresolved_threads, 2);
}

/// Without `approvedBy` (older fixtures), fall back to required - left.
#[test]
fn approvals_given_falls_back_without_approver_list() {
    let mut v = json!({
        "id": "gid://gitlab/MergeRequest/1", "iid": "1", "state": "opened",
        "approved": true, "approvalsRequired": 1, "approvalsLeft": 0,
        "detailedMergeStatus": "MERGEABLE"
    });
    let m: MrRaw = serde_json::from_value(v.take()).unwrap();
    let item = mr_to_item(&m).mr.unwrap();
    assert_eq!(item.approvals.given, 1);
    assert!(item.approvals.approved);
    assert_eq!(merge_status(&m), Some(T::ReadyToMerge));
}

fn rel(link: &str, iid: &str, state_label: Option<&str>) -> serde_json::Value {
    let labels: Vec<_> = state_label
        .map(|l| json!({ "title": l, "color": "#428bca", "textColor": "#fff" }))
        .into_iter()
        .collect();
    json!({ "linkType": link, "workItem": {
        "iid": iid, "reference": format!("#{iid}"), "title": format!("Ticket {iid}"),
        "webUrl": format!("https://gitlab.com/g/p/-/work_items/{iid}"), "state": "OPEN",
        "widgets": [
            { "__typename": "WorkItemWidgetLabels", "labels": { "nodes": labels } },
            { "__typename": "WorkItemWidgetStatus", "status": { "name": "To do", "color": "#737278" } }
        ]
    }})
}

/// FR-036: a CLOSES link wins over the branch-number match.
#[test]
fn ticket_prefers_closes_link() {
    let m = mr(
        json!({ "sourceBranch": "19368-x", "workItemRelations": { "nodes": [
        rel("MENTIONED", "19368", Some("state::Ready for Testing")),
        rel("CLOSES", "19912", Some("state::In Testing")),
    ] } }),
    );
    let t = pick_ticket(&m).unwrap();
    assert_eq!(t.reference, "#19912");
    assert_eq!(t.state_label.unwrap().value, "In Testing");
    assert_eq!(t.status.unwrap().name, "To do");
}

/// FR-036: without CLOSES, the mention whose iid starts the branch name; other mentions
/// (e.g. "see #18442") don't count.
#[test]
fn ticket_falls_back_to_branch_number() {
    let m = mr(
        json!({ "sourceBranch": "19450-web-sign-in", "workItemRelations": { "nodes": [
        rel("MENTIONED", "18442", None),
        rel("MENTIONED", "19450", Some("state::Ready for Testing")),
    ] } }),
    );
    assert_eq!(pick_ticket(&m).unwrap().reference, "#19450");
    let other = mr(
        json!({ "sourceBranch": "resource-test", "workItemRelations": { "nodes": [
        rel("MENTIONED", "18442", None)
    ] } }),
    );
    assert_eq!(pick_ticket(&other), None);
}

/// FR-036: approved + mergeable, but the ticket is "Ready for Testing" → "Approved", not
/// "Ready to merge" (MR !19717 / #19368 on 2026-11-04).
#[test]
fn ticket_not_ready_means_approved_not_ready() {
    let m = mr(
        json!({ "sourceBranch": "19368-email", "workItemRelations": { "nodes": [
        rel("MENTIONED", "19368", Some("state::Ready for Testing"))
    ] } }),
    );
    assert!(ticket_blocks_merge(pick_ticket(&m).as_ref()));
    assert_eq!(merge_status(&m), Some(T::Approved));
    assert_eq!(
        mr_to_item(&m).mr.unwrap().ticket.unwrap().reference,
        "#19368"
    );
}

#[test]
fn ticket_ready_to_merge_or_without_state_allows_ready() {
    let ready = mr(
        json!({ "sourceBranch": "1-a", "workItemRelations": { "nodes": [
        rel("CLOSES", "1", Some("state::ready to merge"))
    ] } }),
    );
    assert_eq!(merge_status(&ready), Some(T::ReadyToMerge));
    let no_state = mr(
        json!({ "sourceBranch": "1-a", "workItemRelations": { "nodes": [
        rel("CLOSES", "1", Some("priority::P3"))
    ] } }),
    );
    assert!(!ticket_blocks_merge(pick_ticket(&no_state).as_ref()));
    assert_eq!(merge_status(&no_state), Some(T::ReadyToMerge));
}

/// Earlier blockers still win over the ticket state.
#[test]
fn ticket_state_does_not_hide_needs_approval() {
    let m = mr(
        json!({ "approvedBy": { "nodes": [] }, "sourceBranch": "1-a",
        "workItemRelations": { "nodes": [ rel("CLOSES", "1", Some("state::Ready for Testing")) ] } }),
    );
    assert_eq!(merge_status(&m), Some(T::NeedsApproval));
}

/// FR-036b: MRs without a linked ticket get the one named by their branch number; the
/// ticket rule then turns "Ready to merge" into "Approved".
#[test]
fn missing_ticket_is_looked_up_by_branch_number() {
    use gitlab_tracker_lib::gitlab::parse::ticket_from;
    use gitlab_tracker_lib::gitlab::tickets::{apply_tickets, missing_tickets, parse_tickets};
    use std::collections::HashMap;

    let mut a = mr_to_item(&mr(json!({ "sourceBranch": "14796-bulk-import" })));
    a.project_path = "g/p".into();
    let mut b = mr_to_item(&mr(json!({ "sourceBranch": "resource-test" })));
    b.project_path = "g/p".into();
    assert_eq!(a.mr.as_ref().unwrap().merge_status, Some(T::ReadyToMerge));
    let need = missing_tickets(&[a.clone(), b.clone()]);
    assert_eq!(need["g/p"].iter().collect::<Vec<_>>(), vec!["14796"]);

    let data = json!({ "project": { "workItems": { "nodes": [ rel("MENTIONED", "14796", Some("state::In Development"))["workItem"].clone() ] } } });
    let found: HashMap<_, _> = parse_tickets(&data)
        .iter()
        .map(|w| (("g/p".to_string(), w.iid.clone()), ticket_from(w)))
        .collect();
    let mut items = vec![a, b];
    apply_tickets(&mut items, &found);
    let m = items[0].mr.as_ref().unwrap();
    assert_eq!(
        m.ticket
            .as_ref()
            .unwrap()
            .state_label
            .as_ref()
            .unwrap()
            .value,
        "In Development"
    );
    assert_eq!(m.merge_status, Some(T::Approved));
    assert_eq!(items[1].mr.as_ref().unwrap().ticket, None);
}
