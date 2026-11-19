use gitlab_tracker_lib::model::{ListKind, default_tabs};
use gitlab_tracker_lib::url_map::{Relation, Scope, StatusFilter, check_url, parse_tab_url};
use pretty_assertions::assert_eq;

fn s(v: &[&str]) -> Vec<String> {
    v.iter().map(|x| (*x).to_string()).collect()
}

#[test]
fn default_my_mrs() {
    let q = parse_tab_url(&default_tabs()[0].url).unwrap();
    assert_eq!(q.kind, ListKind::MergeRequests);
    assert_eq!(
        q.scope,
        Scope::Dashboard {
            relation: Relation::Author,
            username: "DusanJ88".into()
        }
    );
    assert_eq!(q.filters.state.as_deref(), Some("opened"));
    assert_eq!(
        q.filters.author, None,
        "the relation isn't repeated as a filter"
    );
    assert_eq!(q.ignored_params, [] as [std::string::String; 0]);
}

#[test]
fn default_reviewing() {
    let q = parse_tab_url(&default_tabs()[2].url).unwrap();
    assert_eq!(
        q.scope,
        Scope::Dashboard {
            relation: Relation::Reviewer,
            username: "DusanJ88".into()
        }
    );
    assert_eq!(q.filters.reviewer, None);
    assert_eq!(q.filters.state.as_deref(), Some("opened"));
}

#[test]
fn default_my_tickets() {
    let q = parse_tab_url(&default_tabs()[3].url).unwrap();
    assert_eq!(q.kind, ListKind::WorkItems);
    assert_eq!(
        q.scope,
        Scope::Group {
            path: "login5".into()
        }
    );
    assert_eq!(q.filters.assignees, s(&["DusanJ88"]));
    assert_eq!(q.filters.status, Some(StatusFilter::Name("To do".into())));
    assert_eq!(q.filters.not_milestone.as_deref(), Some("Backlog"));
    assert_eq!(
        q.filters.not_labels,
        s(&[
            "state::Ready to Merge",
            "state::Ready for Testing",
            "state::In Testing"
        ])
    );
    assert_eq!(q.ignored_params, [] as [std::string::String; 0]);
}

#[test]
fn default_ready_to_merge() {
    let q = parse_tab_url(&default_tabs()[4].url).unwrap();
    assert_eq!(
        q.scope,
        Scope::Group {
            path: "login5".into()
        }
    );
    assert_eq!(q.filters.labels, s(&["state::Ready to Merge"]));
    assert_eq!(q.filters.not_milestone.as_deref(), Some("Backlog"));
}

/// FR-015, FR-034: "On resource-test" is My MRs filtered to the test branch.
#[test]
fn default_on_resource_test() {
    let tabs = default_tabs();
    assert_eq!(tabs.len(), 6);
    assert_eq!(tabs[1].name, "On resource-test");
    assert_eq!(tabs[1].url, tabs[0].url);
    assert_eq!(tabs[1].test_branch.as_deref(), Some("resource-test"));
    assert!(tabs[1].notify_new_items && tabs[1].notify_new_comments && tabs[1].notify_changes);
    assert_eq!(
        parse_tab_url(&tabs[1].url).unwrap().kind,
        ListKind::MergeRequests
    );
}

/// FR-015, FR-038: "Done" lists my closed tickets, newest closed first, grouped by week.
#[test]
fn default_done() {
    let tab = &default_tabs()[5];
    assert_eq!(tab.name, "Done");
    assert!(tab.group_by_week);
    assert!(!tab.notify_new_items && !tab.notify_new_comments && !tab.notify_changes);
    let q = parse_tab_url(&tab.url).unwrap();
    assert_eq!(q.kind, ListKind::WorkItems);
    assert_eq!(q.filters.state.as_deref(), Some("closed"));
    assert_eq!(q.filters.assignees, s(&["DusanJ88"]));
    assert_eq!(q.filters.sort.as_deref(), Some("CLOSED_AT_DESC"));
    assert!(!q.sort_fallback);
    assert_eq!(q.ignored_params, [] as [std::string::String; 0]);
}

#[test]
fn project_mr_with_scoped_label_draft_sort() {
    let q = parse_tab_url(
        "https://gitlab.com/login5/login-eko/login-eko/-/merge_requests?label_name[]=a::b&draft=no&sort=created_date",
    )
    .unwrap();
    assert_eq!(q.kind, ListKind::MergeRequests);
    assert_eq!(
        q.scope,
        Scope::Project {
            path: "login5/login-eko/login-eko".into()
        }
    );
    assert_eq!(q.filters.labels, s(&["a::b"]));
    assert_eq!(q.filters.draft, Some(false));
    assert_eq!(q.filters.sort.as_deref(), Some("CREATED_DESC"));
    assert!(!q.sort_fallback);
}

#[test]
fn group_issues_unknown_param_ignored() {
    let q = parse_tab_url("https://gitlab.com/groups/login5/-/issues?foo=1").unwrap();
    assert_eq!(q.kind, ListKind::WorkItems);
    assert_eq!(
        q.scope,
        Scope::Group {
            path: "login5".into()
        }
    );
    assert_eq!(q.ignored_params, s(&["foo"]));
}

#[test]
fn single_item_rejected() {
    let e = parse_tab_url("https://gitlab.com/login5/x/-/merge_requests/12").unwrap_err();
    assert!(e.0.contains("single item"), "{}", e.0);
    assert!(parse_tab_url("https://gitlab.com/login5/x/-/work_items/45").is_err());
    assert!(parse_tab_url("https://gitlab.com/login5/x/-/issues/45").is_err());
}

#[test]
fn other_host_rejected() {
    let e = parse_tab_url("https://example.com/groups/a/-/merge_requests").unwrap_err();
    assert!(e.0.contains("host"), "{}", e.0);
}

#[test]
fn dashboard_needs_relation() {
    let e = parse_tab_url("https://gitlab.com/dashboard/merge_requests").unwrap_err();
    assert!(e.0.contains("author, reviewer or assignee"), "{}", e.0);
}

#[test]
fn dashboard_work_items_rejected() {
    for u in [
        "https://gitlab.com/dashboard/issues?assignee_username=x",
        "https://gitlab.com/dashboard/work_items",
    ] {
        let e = parse_tab_url(u).unwrap_err();
        assert!(e.0.contains("group or project"), "{}", e.0);
    }
}

#[test]
fn dashboard_extra_relations_become_filters() {
    let q = parse_tab_url(
        "https://gitlab.com/dashboard/merge_requests?reviewer_username=bob&assignee_username=carol&author_username=al",
    )
    .unwrap();
    assert_eq!(
        q.scope,
        Scope::Dashboard {
            relation: Relation::Author,
            username: "al".into()
        }
    );
    assert_eq!(q.filters.reviewer.as_deref(), Some("bob"));
    assert_eq!(q.filters.assignees, s(&["carol"]));
    let q = parse_tab_url("https://gitlab.com/dashboard/merge_requests?assignee_username[]=carol")
        .unwrap();
    assert_eq!(
        q.scope,
        Scope::Dashboard {
            relation: Relation::Assignee,
            username: "carol".into()
        }
    );
}

#[test]
fn array_and_comma_label_forms() {
    let a =
        parse_tab_url("https://gitlab.com/groups/g/-/work_items?label_name[]=x&label_name[]=y%20z")
            .unwrap();
    let b = parse_tab_url("https://gitlab.com/groups/g/-/work_items?label_name=x,y+z").unwrap();
    assert_eq!(a.filters.labels, s(&["x", "y z"]));
    assert_eq!(b.filters.labels, a.filters.labels);
}

#[test]
fn unmappable_sort_falls_back() {
    let q =
        parse_tab_url("https://gitlab.com/groups/g/-/merge_requests?sort=something_new").unwrap();
    assert_eq!(q.filters.sort.as_deref(), Some("UPDATED_DESC"));
    assert!(q.sort_fallback);
    // A work-item-only sort is unmappable for MRs.
    let q = parse_tab_url("https://gitlab.com/groups/g/-/merge_requests?sort=weight").unwrap();
    assert!(q.sort_fallback);
    let q = parse_tab_url("https://gitlab.com/groups/g/-/work_items?sort=weight").unwrap();
    assert_eq!(q.filters.sort.as_deref(), Some("WEIGHT_ASC"));
}

#[test]
fn paging_params_dropped_and_state_all_omitted() {
    let q = parse_tab_url(
        "https://gitlab.com/groups/g/-/merge_requests?state=all&per_page=20&page=3&first_page_size=20",
    )
    .unwrap();
    assert_eq!(q.filters.state, None);
    assert_eq!(q.ignored_params, [] as [std::string::String; 0]);
}

#[test]
fn work_item_specific_params() {
    let q = parse_tab_url(
        "https://gitlab.com/a/b/-/work_items?type[]=issue&type[]=task&status_id=gid://gitlab/WorkItems::Statuses::SystemDefined::Status/1&reviewer_username=x&draft=yes",
    )
    .unwrap();
    assert_eq!(q.scope, Scope::Project { path: "a/b".into() });
    assert_eq!(q.filters.types, s(&["ISSUE", "TASK"]));
    assert_eq!(
        q.filters.status,
        Some(StatusFilter::Id(
            "gid://gitlab/WorkItems::Statuses::SystemDefined::Status/1".into()
        ))
    );
    assert_eq!(q.ignored_params, s(&["draft", "reviewer_username"]));
    assert!(parse_tab_url("https://gitlab.com/a/b/-/work_items?state=merged").is_err());
}

#[test]
fn not_author_and_assignee() {
    let q = parse_tab_url(
        "https://gitlab.com/groups/g/-/merge_requests?not[author_username]=bot&not[assignee_username][]=x",
    )
    .unwrap();
    assert_eq!(q.filters.not_author.as_deref(), Some("bot"));
    assert_eq!(q.filters.not_assignees, s(&["x"]));
}

#[test]
fn single_namespace_without_groups_prefix_is_group() {
    let q = parse_tab_url("https://gitlab.com/login5/-/merge_requests/").unwrap();
    assert_eq!(
        q.scope,
        Scope::Group {
            path: "login5".into()
        }
    );
}

#[test]
fn check_url_hint() {
    let c = check_url(&default_tabs()[3].url);
    assert!(c.valid);
    assert_eq!(c.kind, Some(ListKind::WorkItems));
    assert_eq!(c.scope_label.as_deref(), Some("Group login5"));
    assert!(c.filters_summary.contains(&"status: To do".to_string()));
    let bad = check_url("not a url");
    assert!(!bad.valid);
    assert!(bad.error.is_some());
}

#[test]
fn array_value_with_comma_kept_whole() {
    let q = parse_tab_url("https://gitlab.com/groups/g/-/work_items?label_name[]=a%2Cb").unwrap();
    assert_eq!(q.filters.labels, s(&["a,b"]));
}
