//! "Only MRs merged into branch X" tabs (FR-034): which open MRs have their source branch
//! merged into a shared test branch (e.g. `resource-test`), and whether it's up to date there.
//!
//! - Up to date: the MR's head commit is an ancestor of the test branch.
//! - Outdated: an earlier head was merged (found through the test branch's
//!   "Merge branch '<source>' … into <test>" commits) and is an ancestor of today's head.
//! - Otherwise the MR isn't on the test branch and is left out of the tab.
//!
//! Ancestry is checked with `merge_base(a, b) == a`, which is exact even when the branches
//! share several merge bases (a plain merge-base comparison is not, see research R19).

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use chrono::{DateTime, Utc};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde_json::Value;

use super::client::GitLabClient;
use crate::error::{AppError, AppResult};
use crate::model::{ListItem, OnBranch};

/// First-parent commits of the test branch searched for merges of a source branch.
pub const MERGE_HISTORY: u32 = 100;

/// One merge commit on the test branch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchMerge {
    pub title: String,
    /// Second parent: the source branch's head when it was merged.
    pub merged_head: String,
    pub at: Option<DateTime<Utc>>,
}

/// Results per (project, MR head, test-branch head): both shas fix the answer.
type CacheKey = (String, String, String);
static CACHE: LazyLock<Mutex<HashMap<CacheKey, Option<OnBranch>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
const CACHE_MAX: usize = 2000;

fn enc(s: &str) -> String {
    utf8_percent_encode(s, NON_ALPHANUMERIC).to_string()
}

/// Merge commits from a `repository/commits` response, newest first.
pub fn parse_merges(commits: &Value) -> Vec<BranchMerge> {
    commits.as_array().map_or_default(|a| {
        a.iter()
            .filter_map(|c| {
                let parents = c.get("parent_ids")?.as_array()?;
                let merged_head = parents.get(1)?.as_str()?.to_string();
                Some(BranchMerge {
                    title: c.get("title")?.as_str()?.to_string(),
                    merged_head,
                    at: c
                        .get("committed_date")
                        .and_then(Value::as_str)
                        .and_then(|d| DateTime::parse_from_rfc3339(d).ok())
                        .map(|d| d.with_timezone(&Utc)),
                })
            })
            .collect()
    })
}

/// Newest merge of `source_branch`, matched on git's merge title
/// ("Merge branch 'x' …" or "Merge remote-tracking branch 'origin/x' …").
pub fn find_branch_merge<'a>(
    merges: &'a [BranchMerge],
    source_branch: &str,
) -> Option<&'a BranchMerge> {
    let quoted = format!("'{source_branch}'");
    let remote = format!("/{source_branch}'");
    merges
        .iter()
        .find(|m| m.title.contains(&quoted) || m.title.contains(&remote))
}

/// `Ok(None)` for a 404, so a missing branch or commit reads as "absent".
async fn get_opt(client: &GitLabClient, path: &str) -> AppResult<Option<Value>> {
    match client.rest_get(path).await {
        Ok(v) => Ok(Some(v)),
        Err(AppError::NotFound(_)) => Ok(None),
        Err(e) => Err(e),
    }
}

/// True when `ancestor` is reachable from `descendant`.
async fn is_ancestor(
    client: &GitLabClient,
    project: &str,
    ancestor: &str,
    descendant: &str,
) -> AppResult<bool> {
    let path = format!(
        "projects/{}/repository/merge_base?refs%5B%5D={}&refs%5B%5D={}",
        enc(project),
        enc(ancestor),
        enc(descendant)
    );
    Ok(get_opt(client, &path)
        .await?
        .and_then(|v| v.get("id").and_then(Value::as_str).map(str::to_string))
        .is_some_and(|id| id == ancestor))
}

struct ProjectBranch {
    head: String,
    merges: Option<Vec<BranchMerge>>,
}

/// Keeps the MRs that are on `branch` and sets their [`OnBranch`]. With `keep_all`, every
/// MR is kept and only the ones on the branch are flagged (FR-037). Returns a warning when
/// the branch exists in none of the MRs' projects (filter mode only).
pub async fn filter_on_branch(
    client: &GitLabClient,
    items: Vec<ListItem>,
    branch: &str,
    keep_all: bool,
) -> AppResult<(Vec<ListItem>, Option<String>)> {
    let mut projects: HashMap<String, Option<ProjectBranch>> = HashMap::new();
    let mut out = Vec::with_capacity(items.len());
    for mut item in items {
        let Some(mr) = item.mr.as_ref() else {
            if keep_all {
                out.push(item);
            }
            continue;
        };
        if mr.source_branch == branch || mr.head_sha.is_empty() {
            if keep_all {
                out.push(item);
            }
            continue;
        }
        let project = item.project_path.clone();
        if !projects.contains_key(&project) {
            let path = format!(
                "projects/{}/repository/branches/{}",
                enc(&project),
                enc(branch)
            );
            let head = get_opt(client, &path).await?.and_then(|v| {
                v.pointer("/commit/id")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            });
            projects.insert(
                project.clone(),
                head.map(|head| ProjectBranch { head, merges: None }),
            );
        }
        let Some(Some(pb)) = projects.get_mut(&project) else {
            if keep_all {
                out.push(item);
            }
            continue;
        };

        let key = (project.clone(), mr.head_sha.clone(), pb.head.clone());
        let cached = CACHE.lock().ok().and_then(|c| c.get(&key).cloned());
        let status = if let Some(s) = cached {
            s
        } else {
            let s = check(
                client,
                &project,
                branch,
                pb,
                &mr.source_branch,
                &mr.head_sha,
            )
            .await?;
            if let Ok(mut c) = CACHE.lock() {
                if c.len() >= CACHE_MAX {
                    c.clear();
                }
                c.insert(key, s.clone());
            }
            s
        };
        let on = status.is_some();
        if let Some(m) = item.mr.as_mut() {
            m.on_branch = status;
        }
        if on || keep_all {
            out.push(item);
        }
    }
    let warning = (!keep_all && !projects.is_empty() && projects.values().all(Option::is_none))
        .then(|| format!("Branch '{branch}' not found"));
    Ok((out, warning))
}

async fn check(
    client: &GitLabClient,
    project: &str,
    branch: &str,
    pb: &mut ProjectBranch,
    source_branch: &str,
    head: &str,
) -> AppResult<Option<OnBranch>> {
    if pb.merges.is_none() {
        let path = format!(
            "projects/{}/repository/commits?ref_name={}&first_parent=true&per_page={MERGE_HISTORY}",
            enc(project),
            enc(branch)
        );
        pb.merges = Some(
            get_opt(client, &path)
                .await?
                .map_or_default(|v| parse_merges(&v)),
        );
    }
    let last = pb
        .merges
        .as_deref()
        .and_then(|m| find_branch_merge(m, source_branch))
        .cloned();
    if is_ancestor(client, project, head, &pb.head).await? {
        return Ok(Some(OnBranch {
            branch: branch.to_string(),
            up_to_date: true,
            merged_at: last.and_then(|m| m.at),
        }));
    }
    match last {
        Some(m) if is_ancestor(client, project, &m.merged_head, head).await? => {
            Ok(Some(OnBranch {
                branch: branch.to_string(),
                up_to_date: false,
                merged_at: m.at,
            }))
        }
        _ => Ok(None),
    }
}
