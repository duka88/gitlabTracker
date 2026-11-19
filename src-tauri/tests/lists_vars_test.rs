//! Variables sent for each default tab must match what was verified live in T002.
use gitlab_tracker_lib::gitlab::lists::{parse_page, query_for, variables};
use gitlab_tracker_lib::model::default_tabs;
use gitlab_tracker_lib::url_map::parse_tab_url;
use pretty_assertions::assert_eq;
use serde_json::json;

#[test]
fn my_mrs_vars() {
    let q = parse_tab_url(&default_tabs()[0].url).unwrap();
    assert_eq!(
        variables(&q, None, true),
        json!({ "username": "DusanJ88", "authored": true, "reviewing": false, "assigned": false, "state": "opened" })
    );
    assert_eq!(query_for(&q).1, vec!["user", "authored"]);
}

#[test]
fn my_tickets_vars_with_and_without_status() {
    let q = parse_tab_url(&default_tabs()[3].url).unwrap();
    let with = variables(&q, Some("CUR"), true);
    assert_eq!(
        with,
        json!({
            "after": "CUR",
            "fullPath": "login5",
            "state": "opened",
            "assigneeUsernames": ["DusanJ88"],
            "status": { "name": "To do" },
            "not": {
                "milestoneTitle": ["Backlog"],
                "labelName": ["state::Ready to Merge", "state::Ready for Testing", "state::In Testing"]
            }
        })
    );
    let without = variables(&q, None, false);
    assert!(without.get("status").is_none());
}

#[test]
fn project_mr_vars() {
    let q = parse_tab_url(
        "https://gitlab.com/a/b/-/merge_requests?label_name[]=x&draft=no&sort=created_date&milestone_title=M&not[label_name][]=y&assignee_username=u",
    )
    .unwrap();
    assert_eq!(
        variables(&q, None, true),
        json!({
            "fullPath": "a/b", "labels": ["x"], "draft": false, "sort": "CREATED_DESC",
            "milestoneTitle": "M", "not": { "labels": ["y"] }, "assigneeUsername": "u"
        })
    );
}

#[test]
fn dashboard_never_sends_own_relation_filter() {
    let q = parse_tab_url(
        "https://gitlab.com/dashboard/merge_requests?assignee_username[]=me&assignee_username[]=other&author_username=a",
    )
    .unwrap();
    // Author is the relation; assignee goes as a filter.
    let v = variables(&q, None, true);
    assert_eq!(v["authored"], json!(true));
    assert!(v.get("authorUsername").is_none());
    assert_eq!(v["assigneeUsername"], json!("me"));
}

#[test]
fn null_root_is_not_found() {
    let q = parse_tab_url("https://gitlab.com/groups/nope/-/work_items").unwrap();
    let e = parse_page(&q, &json!({ "group": null })).unwrap_err();
    assert_eq!(e.to_string(), "Group not found");
    let q = parse_tab_url("https://gitlab.com/a/b/-/merge_requests").unwrap();
    assert_eq!(
        parse_page(&q, &json!({ "project": null }))
            .unwrap_err()
            .to_string(),
        "Project not found"
    );
}
