//! `TabQuery` → paged GraphQL fetch → `Vec<ListItem>` (research R2/R3, url-mapping contract).

use serde_json::{Map, Value, json};

use super::client::{GitLabClient, is_status_filter_error};
use super::parse::{PageInfo, parse_mr_page, parse_work_item_page};
use crate::error::{AppError, AppResult, Issue};
use crate::model::{ListItem, ListKind};
use crate::url_map::{Relation, Scope, StatusFilter, TabQuery};

pub const MRS_DASHBOARD: &str = include_str!("queries/mrs_dashboard.graphql");
pub const MRS_GROUP: &str = include_str!("queries/mrs_group.graphql");
pub const MRS_PROJECT: &str = include_str!("queries/mrs_project.graphql");
pub const WORK_ITEMS_GROUP: &str = include_str!("queries/work_items_group.graphql");
pub const WORK_ITEMS_PROJECT: &str = include_str!("queries/work_items_project.graphql");

/// Items loaded per tab at most (FR-016); 10 pages of 50.
pub const MAX_ITEMS: usize = 500;

pub const SORT_FALLBACK_WARNING: &str = "Sort not supported — sorted by last updated";
pub const STATUS_FALLBACK_WARNING: &str = "Status filter not supported — showing all statuses";

#[derive(Debug, Clone, PartialEq)]
pub struct FetchResult {
    pub items: Vec<ListItem>,
    /// Total reported by GitLab when more than [`MAX_ITEMS`] matched.
    pub truncated_total: Option<u32>,
    pub warning: Option<Issue>,
}

fn put(m: &mut Map<String, Value>, k: &str, v: Option<Value>) {
    if let Some(v) = v {
        m.insert(k.to_string(), v);
    }
}

fn opt_str(v: Option<&String>) -> Option<Value> {
    v.map(|s| json!(s))
}

fn list(v: &[String]) -> Option<Value> {
    if v.is_empty() { None } else { Some(json!(v)) }
}

fn object(m: Map<String, Value>) -> Option<Value> {
    if m.is_empty() {
        None
    } else {
        Some(Value::Object(m))
    }
}

/// GraphQL variables for one page. Unset filters are left out entirely, so GitLab treats the
/// argument as absent rather than `null`.
pub fn variables(q: &TabQuery, after: Option<&str>, include_status: bool) -> Value {
    let f = &q.filters;
    let mut m = Map::new();
    put(&mut m, "after", after.map(|a| json!(a)));
    put(&mut m, "state", opt_str(f.state.as_ref()));
    put(&mut m, "search", opt_str(f.search.as_ref()));
    put(&mut m, "sort", opt_str(f.sort.as_ref()));
    match q.kind {
        ListKind::MergeRequests => {
            put(&mut m, "labels", list(&f.labels));
            put(&mut m, "milestoneTitle", opt_str(f.milestone.as_ref()));
            put(&mut m, "draft", f.draft.map(|d| json!(d)));
            let mut not = Map::new();
            put(&mut not, "labels", list(&f.not_labels));
            put(
                &mut not,
                "milestoneTitle",
                opt_str(f.not_milestone.as_ref()),
            );
            put(&mut not, "authorUsername", opt_str(f.not_author.as_ref()));
            put(&mut not, "assigneeUsernames", list(&f.not_assignees));
            put(&mut m, "not", object(not));
            let author = opt_str(f.author.as_ref());
            let assignee = opt_str(f.assignees.first());
            let reviewer = opt_str(f.reviewer.as_ref());
            match &q.scope {
                Scope::Dashboard { relation, username } => {
                    m.insert("username".into(), json!(username));
                    m.insert("authored".into(), json!(*relation == Relation::Author));
                    m.insert("reviewing".into(), json!(*relation == Relation::Reviewer));
                    m.insert("assigned".into(), json!(*relation == Relation::Assignee));
                    // Each connection rejects the filter for its own relation.
                    if *relation != Relation::Author {
                        put(&mut m, "authorUsername", author);
                    }
                    if *relation != Relation::Assignee {
                        put(&mut m, "assigneeUsername", assignee);
                    }
                    if *relation != Relation::Reviewer {
                        put(&mut m, "reviewerUsername", reviewer);
                    }
                }
                Scope::Group { path } | Scope::Project { path } => {
                    m.insert("fullPath".into(), json!(path));
                    put(&mut m, "authorUsername", author);
                    put(&mut m, "assigneeUsername", assignee);
                    put(&mut m, "reviewerUsername", reviewer);
                }
            }
        }
        ListKind::WorkItems => {
            if let Scope::Group { path } | Scope::Project { path } = &q.scope {
                m.insert("fullPath".into(), json!(path));
            }
            put(&mut m, "authorUsername", opt_str(f.author.as_ref()));
            put(&mut m, "assigneeUsernames", list(&f.assignees));
            put(&mut m, "labelName", list(&f.labels));
            put(
                &mut m,
                "milestoneTitle",
                f.milestone.as_ref().map(|s| json!([s])),
            );
            put(&mut m, "types", list(&f.types));
            let mut not = Map::new();
            put(&mut not, "labelName", list(&f.not_labels));
            put(
                &mut not,
                "milestoneTitle",
                f.not_milestone.as_ref().map(|s| json!([s])),
            );
            put(
                &mut not,
                "authorUsername",
                f.not_author.as_ref().map(|s| json!([s])),
            );
            put(&mut not, "assigneeUsernames", list(&f.not_assignees));
            put(&mut m, "not", object(not));
            if include_status {
                put(
                    &mut m,
                    "status",
                    f.status.as_ref().map(|s| match s {
                        StatusFilter::Name(n) => json!({ "name": n }),
                        StatusFilter::Id(i) => json!({ "id": i }),
                    }),
                );
            }
        }
    }
    Value::Object(m)
}

/// Query text for a tab, and the path from `data` to the connection.
pub fn query_for(q: &TabQuery) -> (&'static str, Vec<&'static str>) {
    match (q.kind, &q.scope) {
        (ListKind::MergeRequests, Scope::Dashboard { relation, .. }) => {
            let alias = match relation {
                Relation::Author => "authored",
                Relation::Reviewer => "reviewing",
                Relation::Assignee => "assigned",
            };
            (MRS_DASHBOARD, vec!["user", alias])
        }
        (ListKind::MergeRequests, Scope::Group { .. }) => {
            (MRS_GROUP, vec!["group", "mergeRequests"])
        }
        (ListKind::MergeRequests, Scope::Project { .. }) => {
            (MRS_PROJECT, vec!["project", "mergeRequests"])
        }
        (ListKind::WorkItems, Scope::Project { .. }) => {
            (WORK_ITEMS_PROJECT, vec!["project", "workItems"])
        }
        // Dashboard work items are rejected by the URL parser; group is the only other scope.
        (ListKind::WorkItems, _) => (WORK_ITEMS_GROUP, vec!["group", "workItems"]),
    }
}

fn not_found(root: &str) -> AppError {
    AppError::NotFound(match root {
        "user" => "User not found".into(),
        "project" => "Project not found".into(),
        _ => "Group not found".into(),
    })
}

type Parsed = (Vec<ListItem>, PageInfo, Option<u32>);

/// Picks the connection out of `data` and parses one page.
pub fn parse_page(q: &TabQuery, data: &Value) -> AppResult<Parsed> {
    let (_, path) = query_for(q);
    let root = data
        .get(path[0])
        .filter(|v| !v.is_null())
        .ok_or_else(|| not_found(path[0]))?;
    let conn = root
        .get(path[1])
        .filter(|v| !v.is_null())
        .ok_or_else(|| AppError::Query(format!("GitLab returned no {} list", path[1])))?;
    let parsed = match q.kind {
        ListKind::MergeRequests => parse_mr_page(conn),
        ListKind::WorkItems => parse_work_item_page(conn),
    };
    parsed.map_err(|e| AppError::Query(format!("Unexpected GitLab response: {e}")))
}

async fn fetch_all(
    client: &GitLabClient,
    q: &TabQuery,
    with_status: bool,
) -> AppResult<FetchResult> {
    let (query, _) = query_for(q);
    let mut items: Vec<ListItem> = Vec::new();
    let mut after: Option<String> = None;
    let mut total: Option<u32> = None;
    loop {
        let data = client
            .graphql(query, variables(q, after.as_deref(), with_status))
            .await?;
        let (page_items, page, count) = parse_page(q, &data)?;
        total = total.or(count);
        items.extend(page_items);
        if items.len() >= MAX_ITEMS {
            items.truncate(MAX_ITEMS);
            let more = page.has_next_page || total.is_some_and(|t| t as usize > MAX_ITEMS);
            let truncated_total = if more {
                Some(
                    total
                        .unwrap_or(MAX_ITEMS as u32 + 1)
                        .max(MAX_ITEMS as u32 + 1),
                )
            } else {
                None
            };
            return Ok(FetchResult {
                items,
                truncated_total,
                warning: None,
            });
        }
        match page.end_cursor {
            Some(c) if page.has_next_page => after = Some(c),
            _ => break,
        }
    }
    // The same item can move between pages while paging; keep the first copy.
    let mut seen = std::collections::HashSet::new();
    items.retain(|i| seen.insert(i.gid.clone()));
    Ok(FetchResult {
        items,
        truncated_total: None,
        warning: None,
    })
}

/// Loads every page of a tab (up to [`MAX_ITEMS`]), with the status-filter retry (R3).
pub async fn fetch_tab(client: &GitLabClient, q: &TabQuery) -> AppResult<FetchResult> {
    let has_status = q.kind == ListKind::WorkItems && q.filters.status.is_some();
    let mut warnings: Vec<String> = Vec::new();
    let mut result = match fetch_all(client, q, true).await {
        Err(e) if has_status && is_status_filter_error(&e) => {
            warnings.push(STATUS_FALLBACK_WARNING.into());
            fetch_all(client, q, false).await?
        }
        other => other?,
    };
    if q.sort_fallback {
        warnings.push(SORT_FALLBACK_WARNING.into());
    }
    if let Some(total) = result.truncated_total {
        warnings.push(format!(
            "Showing {MAX_ITEMS} of {total} — narrow the filter in GitLab"
        ));
    }
    if !warnings.is_empty() {
        result.warning = Some(Issue::warning("warning", warnings.join(" · ")));
    }
    Ok(result)
}
