//! FR-034: pure parts of the test-branch check (merge parsing and branch matching).
use gitlab_tracker_lib::gitlab::branch_check::{find_branch_merge, parse_merges};
use serde_json::json;

fn history() -> serde_json::Value {
    json!([
        { "title": "Merge branch '19368-email-subjects' of gitlab.com:login5/x into resource-test",
          "parent_ids": ["a1", "h19368"], "committed_date": "2026-11-03T13:47:44.000+01:00" },
        { "title": "Translate the keys the Croatian bundles lack", "parent_ids": ["a0"],
          "committed_date": "2026-10-30T16:18:00.000+01:00" },
        { "title": "Merge remote-tracking branch 'origin/14796-bulk-import' into resource-test",
          "parent_ids": ["z9", "h14796-new"], "committed_date": "2026-11-03T11:42:00.000+01:00" },
        { "title": "Merge branch '14796-bulk-import' of gitlab.com:login5/x into resource-test",
          "parent_ids": ["z8", "h14796-old"], "committed_date": "2026-11-02T12:16:00.000+01:00" },
        { "title": "Merge branch '14796-bulk-import-v2' into resource-test",
          "parent_ids": ["z7", "h-v2"], "committed_date": "2026-11-01T10:00:00.000+01:00" }
    ])
}

#[test]
fn keeps_only_merge_commits_with_their_second_parent() {
    let m = parse_merges(&history());
    assert_eq!(m.len(), 4);
    assert_eq!(m[0].merged_head, "h19368");
    assert_eq!(m[0].at.unwrap().to_rfc3339(), "2026-11-03T12:47:44+00:00");
}

#[test]
fn finds_the_newest_merge_of_a_branch_in_both_title_styles() {
    let m = parse_merges(&history());
    assert_eq!(
        find_branch_merge(&m, "14796-bulk-import")
            .unwrap()
            .merged_head,
        "h14796-new"
    );
    assert_eq!(
        find_branch_merge(&m, "19368-email-subjects")
            .unwrap()
            .merged_head,
        "h19368"
    );
}

#[test]
fn a_branch_name_prefix_does_not_match() {
    let m = parse_merges(&history());
    assert_eq!(
        find_branch_merge(&m, "14796-bulk").map(|x| &x.merged_head),
        None
    );
    assert_eq!(
        find_branch_merge(&m, "14796-bulk-import-v2")
            .unwrap()
            .merged_head,
        "h-v2"
    );
}

#[test]
fn bad_input_gives_no_merges() {
    assert_eq!(parse_merges(&json!({ "message": "404" })), vec![]);
}
