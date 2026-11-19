//! Pasted GitLab list URL → `TabQuery` (contracts/url-mapping.md). Pure, no network.

use crate::config::GITLAB_HOSTNAME;
use crate::model::{ListKind, UrlCheck};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    Author,
    Reviewer,
    Assignee,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    Dashboard {
        relation: Relation,
        username: String,
    },
    Group {
        path: String,
    },
    Project {
        path: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusFilter {
    Name(String),
    Id(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Filters {
    /// `opened`, `closed`, `merged`, `locked`; `None` for all.
    pub state: Option<String>,
    pub author: Option<String>,
    pub assignees: Vec<String>,
    pub reviewer: Option<String>,
    pub labels: Vec<String>,
    pub not_labels: Vec<String>,
    pub milestone: Option<String>,
    pub not_milestone: Option<String>,
    pub not_author: Option<String>,
    pub not_assignees: Vec<String>,
    pub status: Option<StatusFilter>,
    /// GraphQL `IssueType` values (`ISSUE`, `TASK`, …).
    pub types: Vec<String>,
    pub draft: Option<bool>,
    pub search: Option<String>,
    /// GraphQL sort enum value.
    pub sort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabQuery {
    pub kind: ListKind,
    pub scope: Scope,
    pub filters: Filters,
    pub ignored_params: Vec<String>,
    /// The URL's `sort` couldn't be mapped; `UPDATED_DESC` is used instead.
    pub sort_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UrlError(pub String);

impl std::fmt::Display for UrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

fn err<T>(msg: &str) -> Result<T, UrlError> {
    Err(UrlError(msg.to_string()))
}

/// Parameters GitLab adds for paging; they never matter because all items are loaded.
const PAGING_PARAMS: [&str; 6] = [
    "per_page",
    "page",
    "first_page_size",
    "last_page_size",
    "page_after",
    "page_before",
];

pub fn parse_tab_url(input: &str) -> Result<TabQuery, UrlError> {
    let input = input.trim();
    if input.is_empty() {
        return err("Paste a GitLab list URL");
    }
    let url = url::Url::parse(input).map_err(|_| UrlError("Not a valid URL".into()))?;
    if !matches!(url.scheme(), "https" | "http") {
        return err("Not a web URL");
    }
    match url.host_str() {
        Some(h) if h == GITLAB_HOSTNAME || h == "www.gitlab.com" => {}
        _ => return err("Different host — only gitlab.com is supported"),
    }

    let segments: Vec<&str> = url
        .path_segments()
        .map_or_default(|s| s.filter(|p| !p.is_empty()).collect());

    let (kind, scope_kind) = parse_path(&segments)?;

    let mut q = TabQuery {
        kind,
        scope: Scope::Group {
            path: String::new(),
        },
        filters: Filters::default(),
        ignored_params: Vec::new(),
        sort_fallback: false,
    };

    let mut author: Option<String> = None;
    let mut reviewer: Option<String> = None;
    let mut assignees: Vec<String> = Vec::new();
    let mut raw_sort: Option<String> = None;

    for (key, value) in url.query_pairs() {
        let key = key.into_owned();
        let value = value.trim().to_string();
        let is_array = key.ends_with("[]");
        let k = key.strip_suffix("[]").unwrap_or(&key);
        if PAGING_PARAMS.contains(&k) {
            continue;
        }
        match k {
            "state" => match value.as_str() {
                "all" | "" => q.filters.state = None,
                "opened" | "closed" | "locked" => q.filters.state = Some(value),
                "merged" if kind == ListKind::MergeRequests => q.filters.state = Some(value),
                "merged" => return err("state=merged isn't valid for work items"),
                _ => q.ignored_params.push(key.clone()),
            },
            "author_username" => author = non_empty(value),
            "assignee_username" => assignees.extend(split_list(&value, is_array)),
            "reviewer_username" => reviewer = non_empty(value),
            "label_name" => q.filters.labels.extend(split_list(&value, is_array)),
            "not[label_name]" => q.filters.not_labels.extend(split_list(&value, is_array)),
            "milestone_title" => q.filters.milestone = non_empty(value),
            "not[milestone_title]" => q.filters.not_milestone = non_empty(value),
            "not[author_username]" => q.filters.not_author = non_empty(value),
            "not[assignee_username]" => {
                q.filters.not_assignees.extend(split_list(&value, is_array));
            }
            "status" | "status_name" if kind == ListKind::WorkItems => {
                q.filters.status = non_empty(value).map(StatusFilter::Name);
            }
            "status_id" if kind == ListKind::WorkItems => {
                q.filters.status = non_empty(value).map(StatusFilter::Id);
            }
            "type" | "types" if kind == ListKind::WorkItems => {
                q.filters.types.extend(
                    split_list(&value, is_array)
                        .into_iter()
                        .map(|t| t.to_uppercase()),
                );
            }
            "draft" | "wip" if kind == ListKind::MergeRequests => {
                q.filters.draft = match value.as_str() {
                    "yes" | "true" => Some(true),
                    "no" | "false" => Some(false),
                    _ => None,
                };
            }
            "search" => q.filters.search = non_empty(value),
            "sort" => raw_sort = non_empty(value),
            _ => q.ignored_params.push(key.clone()),
        }
    }

    // Scope, and for the dashboard which relation is the root (first of author/reviewer/assignee).
    q.scope = match scope_kind {
        PathScope::Dashboard => {
            let (relation, username) = if let Some(a) = author.take() {
                (Relation::Author, a)
            } else if let Some(r) = reviewer.take() {
                (Relation::Reviewer, r)
            } else if !assignees.is_empty() {
                (Relation::Assignee, assignees.remove(0))
            } else {
                return err("Dashboard URL needs an author, reviewer or assignee filter");
            };
            Scope::Dashboard { relation, username }
        }
        PathScope::Group(path) => Scope::Group { path },
        PathScope::Project(path) => Scope::Project { path },
    };
    q.filters.author = author;
    q.filters.assignees = assignees;
    if kind == ListKind::MergeRequests {
        q.filters.reviewer = reviewer;
    } else if reviewer.is_some() {
        q.ignored_params.push("reviewer_username".into());
    }

    if let Some(s) = raw_sort {
        if let Some(mapped) = map_sort(kind, &s) {
            q.filters.sort = Some(mapped.to_string());
        } else {
            q.filters.sort = Some("UPDATED_DESC".into());
            q.sort_fallback = true;
        }
    }

    q.ignored_params.sort();
    q.ignored_params.dedup();
    Ok(q)
}

enum PathScope {
    Dashboard,
    Group(String),
    Project(String),
}

fn parse_path(segments: &[&str]) -> Result<(ListKind, PathScope), UrlError> {
    match segments {
        ["dashboard", "merge_requests"] => {
            return Ok((ListKind::MergeRequests, PathScope::Dashboard));
        }
        ["dashboard", "issues" | "work_items"] => {
            return err(
                "Use a group or project URL — GitLab's API has no cross-group work-item list",
            );
        }
        ["dashboard", ..] => return err("Not a supported list page"),
        _ => {}
    }
    let Some(dash) = segments.iter().position(|s| *s == "-") else {
        return err("Not a merge request or work item list page");
    };
    let after = &segments[dash + 1..];
    let kind = match after.first() {
        Some(&"merge_requests") => ListKind::MergeRequests,
        Some(&"work_items" | &"issues") => ListKind::WorkItems,
        _ => return err("Not a merge request or work item list page"),
    };
    if after.len() > 1 {
        return err("That's a single item, not a list");
    }
    let (is_group, path_segments) = match segments[..dash].split_first() {
        Some((&"groups", rest)) => (true, rest),
        _ => (false, &segments[..dash]),
    };
    if path_segments.is_empty() {
        return err("Missing group or project path");
    }
    let path = path_segments.join("/");
    // A single namespace segment without `/groups/` is a group, not a project.
    if is_group || path_segments.len() == 1 {
        Ok((kind, PathScope::Group(path)))
    } else {
        Ok((kind, PathScope::Project(path)))
    }
}

fn non_empty(v: String) -> Option<String> {
    if v.is_empty() { None } else { Some(v) }
}

/// `key=a,b` and repeated `key[]=` are both accepted. Array values are kept whole, so a
/// label whose name contains a comma survives.
fn split_list(v: &str, is_array: bool) -> Vec<String> {
    if is_array {
        return non_empty(v.trim().to_string()).into_iter().collect();
    }
    v.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect()
}

/// Maps a URL `sort=` value to the GraphQL enum, per kind (research verification R2.2).
pub fn map_sort(kind: ListKind, s: &str) -> Option<&'static str> {
    let common = match s {
        "created_date" | "created_desc" => Some("CREATED_DESC"),
        "created_asc" => Some("CREATED_ASC"),
        "updated_desc" | "updated_date" => Some("UPDATED_DESC"),
        "updated_asc" => Some("UPDATED_ASC"),
        "title_asc" => Some("TITLE_ASC"),
        "title_desc" => Some("TITLE_DESC"),
        "popularity" | "popularity_desc" => Some("POPULARITY_DESC"),
        "popularity_asc" => Some("POPULARITY_ASC"),
        "priority" | "priority_asc" => Some("PRIORITY_ASC"),
        "priority_desc" => Some("PRIORITY_DESC"),
        "label_priority" | "label_priority_asc" => Some("LABEL_PRIORITY_ASC"),
        "label_priority_desc" => Some("LABEL_PRIORITY_DESC"),
        "milestone_due_asc" | "milestone" => Some("MILESTONE_DUE_ASC"),
        "milestone_due_desc" => Some("MILESTONE_DUE_DESC"),
        "closed_at" | "closed_at_desc" => Some("CLOSED_AT_DESC"),
        "closed_at_asc" => Some("CLOSED_AT_ASC"),
        _ => None,
    };
    if common.is_some() {
        return common;
    }
    match kind {
        ListKind::MergeRequests => match s {
            "merged_at" | "merged_at_desc" => Some("MERGED_AT_DESC"),
            "merged_at_asc" => Some("MERGED_AT_ASC"),
            _ => None,
        },
        ListKind::WorkItems => match s {
            "due_date" | "due_date_asc" => Some("DUE_DATE_ASC"),
            "due_date_desc" => Some("DUE_DATE_DESC"),
            "start_date" | "start_date_asc" => Some("START_DATE_ASC"),
            "start_date_desc" => Some("START_DATE_DESC"),
            "weight" | "weight_asc" => Some("WEIGHT_ASC"),
            "weight_desc" => Some("WEIGHT_DESC"),
            "health_status_asc" => Some("HEALTH_STATUS_ASC"),
            "health_status_desc" => Some("HEALTH_STATUS_DESC"),
            "status_asc" => Some("STATUS_ASC"),
            "status_desc" => Some("STATUS_DESC"),
            "relative_position" | "manual" => Some("RELATIVE_POSITION_ASC"),
            _ => None,
        },
    }
}

/// Human description of a parsed query for the live hint (FR-013).
pub fn describe(q: &TabQuery) -> (String, Vec<String>) {
    let scope = match &q.scope {
        Scope::Dashboard { relation, username } => {
            let rel = match relation {
                Relation::Author => "author",
                Relation::Reviewer => "reviewer",
                Relation::Assignee => "assignee",
            };
            format!("Dashboard · {rel} @{username}")
        }
        Scope::Group { path } => format!("Group {path}"),
        Scope::Project { path } => format!("Project {path}"),
    };
    let f = &q.filters;
    let mut out = Vec::new();
    if let Some(s) = &f.state {
        out.push(format!("state: {s}"));
    }
    if let Some(a) = &f.author {
        out.push(format!("author: {a}"));
    }
    if !f.assignees.is_empty() {
        out.push(format!("assignee: {}", f.assignees.join(", ")));
    }
    if let Some(r) = &f.reviewer {
        out.push(format!("reviewer: {r}"));
    }
    if !f.labels.is_empty() {
        out.push(format!("labels: {}", f.labels.join(", ")));
    }
    if !f.not_labels.is_empty() {
        out.push(format!("not labels: {}", f.not_labels.join(", ")));
    }
    if let Some(m) = &f.milestone {
        out.push(format!("milestone: {m}"));
    }
    if let Some(m) = &f.not_milestone {
        out.push(format!("not milestone: {m}"));
    }
    if let Some(a) = &f.not_author {
        out.push(format!("not author: {a}"));
    }
    if !f.not_assignees.is_empty() {
        out.push(format!("not assignee: {}", f.not_assignees.join(", ")));
    }
    match &f.status {
        Some(StatusFilter::Name(n)) => out.push(format!("status: {n}")),
        Some(StatusFilter::Id(i)) => out.push(format!("status id: {i}")),
        None => {}
    }
    if !f.types.is_empty() {
        out.push(format!("type: {}", f.types.join(", ").to_lowercase()));
    }
    if let Some(d) = f.draft {
        out.push(if d {
            "draft only".into()
        } else {
            "no drafts".into()
        });
    }
    if let Some(s) = &f.search {
        out.push(format!("search: {s}"));
    }
    if let Some(s) = &f.sort {
        if q.sort_fallback {
            out.push("sort: last updated (fallback)".into());
        } else {
            out.push(format!("sort: {}", s.to_lowercase().replace('_', " ")));
        }
    }
    (scope, out)
}

/// `validate_url` result (FR-013).
pub fn check_url(url: &str) -> UrlCheck {
    match parse_tab_url(url) {
        Ok(q) => {
            let (scope, filters) = describe(&q);
            UrlCheck {
                valid: true,
                kind: Some(q.kind),
                scope_label: Some(scope),
                filters_summary: filters,
                ignored_params: q.ignored_params,
                error: None,
            }
        }
        Err(e) => UrlCheck {
            valid: false,
            kind: None,
            scope_label: None,
            filters_summary: Vec::new(),
            ignored_params: Vec::new(),
            error: Some(e.0),
        },
    }
}
