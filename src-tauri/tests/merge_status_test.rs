//! Every row of the research R5 merge-status derivation table.
#![allow(clippy::needless_pass_by_value)]
use gitlab_tracker_lib::gitlab::parse::{MrRaw, effectively_approved, merge_status, mr_to_item};
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
