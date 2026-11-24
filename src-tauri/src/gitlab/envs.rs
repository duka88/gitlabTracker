//! What's deployed on which env (FR-040), from GitLab's Environments and Deployments REST API.
//!
//! - An env's state is its latest successful deployment (ref, sha, who, when).
//! - An MR is on an env when its source branch is the deployed ref, or when its head commit is
//!   an ancestor of the deployed sha (e.g. merged into `resource-test`, which qa-1 deploys).
//!   It's "outdated" there when newer commits were pushed (same rule as FR-034).
//! - A ticket is on an env when an MR that resolves it is, or when the deployed ref is its
//!   branch (`14337-some-title` → #14337).

use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::branch_check::{ProjectBranch, check_cached, enc, get_opt};
use super::client::GitLabClient;
use crate::config::GITLAB_HOST;
use crate::error::{AppError, AppResult};
use crate::model::{EnvBadge, EnvDeployment, EnvStatus, ListItem, TrackedEnv};

/// `(id, state)` of the environment named exactly `name` in an `environments` response.
pub fn parse_environment(v: &Value, name: &str) -> Option<(u64, Option<String>)> {
    v.as_array()?
        .iter()
        .find(|e| e.get("name").and_then(Value::as_str) == Some(name))
        .and_then(|e| {
            Some((
                e.get("id")?.as_u64()?,
                e.get("state").and_then(Value::as_str).map(str::to_string),
            ))
        })
}

/// The first deployment of a `deployments` response (newest first).
pub fn parse_deployment(v: &Value) -> Option<EnvDeployment> {
    let d = v.as_array()?.first()?;
    let at = |k: &str| {
        d.get(k)
            .and_then(Value::as_str)
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|t| t.with_timezone(&Utc))
    };
    Some(EnvDeployment {
        id: d.get("id")?.as_u64()?,
        git_ref: d.get("ref")?.as_str()?.to_string(),
        sha: d.get("sha")?.as_str()?.to_string(),
        finished_at: at("finished_at").or_else(|| at("updated_at")),
        user: d
            .pointer("/user/username")
            .and_then(Value::as_str)
            .map(str::to_string),
        job_url: d
            .pointer("/deployable/web_url")
            .and_then(Value::as_str)
            .map(str::to_string),
    })
}

/// Fetches one env. Errors other than global ones (token, offline) end up in `error`.
pub async fn fetch_env(client: &GitLabClient, env: &TrackedEnv, now: DateTime<Utc>) -> EnvStatus {
    let mut out = EnvStatus::empty(env.clone());
    out.checked_at = Some(now);
    match fetch_env_inner(client, env, &mut out).await {
        Ok(()) => {}
        Err(e) => out.error = Some(e.to_string()),
    }
    out
}

async fn fetch_env_inner(
    client: &GitLabClient,
    env: &TrackedEnv,
    out: &mut EnvStatus,
) -> AppResult<()> {
    let project = enc(&env.project);
    let envs = get_opt(
        client,
        &format!(
            "projects/{project}/environments?name={}&per_page=20",
            enc(&env.name)
        ),
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Project {} not found", env.project)))?;
    let (id, state) = parse_environment(&envs, &env.name).ok_or_else(|| {
        AppError::NotFound(format!(
            "Environment {} not found in {}",
            env.name, env.project
        ))
    })?;
    out.web_url = Some(format!("{GITLAB_HOST}/{}/-/environments/{id}", env.project));
    out.state = state;
    let deps = client
        .rest_get(&format!(
            "projects/{project}/deployments?environment={}&status=success&order_by=finished_at&sort=desc&per_page=1",
            enc(&env.name)
        ))
        .await?;
    out.deployment = parse_deployment(&deps);
    Ok(())
}

/// `(project, iid)` from a GitLab issue / work item / MR URL.
pub fn item_key(web_url: &str) -> Option<(String, String)> {
    let path = web_url.strip_prefix(GITLAB_HOST)?.trim_start_matches('/');
    let (project, rest) = path.split_once("/-/")?;
    let mut parts = rest.split('/');
    let kind = parts.next()?;
    if !matches!(kind, "issues" | "work_items" | "merge_requests") {
        return None;
    }
    let iid = parts.next()?.split(['?', '#']).next()?;
    (!iid.is_empty() && iid.bytes().all(|b| b.is_ascii_digit()))
        .then(|| (project.to_string(), iid.to_string()))
}

/// Ticket number a branch is named after: `14337-refuel-two…` → `14337`.
pub fn ticket_of_branch(branch: &str) -> Option<&str> {
    let n = branch.split(['-', '_']).next()?;
    (!n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())).then_some(n)
}

/// Adds a badge, keeping the "up to date" one if the env is already there.
fn add_badge(badges: &mut Vec<EnvBadge>, env: &str, up_to_date: bool) {
    match badges.iter_mut().find(|b| b.env == env) {
        Some(b) => b.up_to_date |= up_to_date,
        None => badges.push(EnvBadge {
            env: env.to_string(),
            up_to_date,
        }),
    }
}

/// Sets `envs` on every item. `statuses` are in Settings order, which is the badge order.
pub async fn flag_envs(
    client: &GitLabClient,
    items: &mut [&mut ListItem],
    statuses: &[EnvStatus],
) -> AppResult<()> {
    // (project, iid) of a ticket → badges from its MRs and from deployed ticket branches.
    let mut tickets: HashMap<(String, String), Vec<EnvBadge>> = HashMap::new();
    // Per MR head: one check per env even when the MR is in several tabs.
    let mut by_mr: BTreeMap<(String, String), Vec<EnvBadge>> = BTreeMap::new();

    for st in statuses {
        let Some(dep) = &st.deployment else { continue };
        let env = &st.env;
        if let Some(n) = ticket_of_branch(&dep.git_ref) {
            add_badge(
                tickets
                    .entry((env.project.clone(), n.to_string()))
                    .or_default(),
                &env.name,
                true,
            );
        }
        let label = format!("env:{}", env.name);
        let mut pb = ProjectBranch::new(dep.sha.clone());
        for item in items.iter() {
            let Some(mr) = item.mr.as_ref() else { continue };
            if item.project_path != env.project || mr.head_sha.is_empty() {
                continue;
            }
            let key = (item.gid.clone(), mr.head_sha.clone());
            if by_mr
                .get(&key)
                .is_some_and(|b| b.iter().any(|b| b.env == env.name))
            {
                continue;
            }
            let up_to_date = if mr.source_branch == dep.git_ref {
                Some(mr.head_sha == dep.sha)
            } else {
                check_cached(
                    client,
                    &env.project,
                    &label,
                    &mut pb,
                    &mr.source_branch,
                    &mr.head_sha,
                )
                .await?
                .map(|ob| ob.up_to_date)
            };
            if let Some(up) = up_to_date {
                add_badge(by_mr.entry(key).or_default(), &env.name, up);
                if let Some(t) = mr.ticket.as_ref().and_then(|t| item_key(&t.web_url)) {
                    add_badge(tickets.entry(t).or_default(), &env.name, up);
                }
            }
        }
    }

    let order = |b: &EnvBadge| statuses.iter().position(|s| s.env.name == b.env);
    for item in items.iter_mut() {
        let mut badges = match item.mr.as_ref() {
            Some(mr) => by_mr
                .get(&(item.gid.clone(), mr.head_sha.clone()))
                .cloned()
                .unwrap_or_default(),
            None => item_key(&item.web_url)
                .or_else(|| Some((item.project_path.clone(), item.iid.clone())))
                .and_then(|k| tickets.get(&k).cloned())
                .unwrap_or_default(),
        };
        badges.sort_by_key(order);
        item.envs = badges;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_environment_by_exact_name() {
        let v = json!([
            {"id": 1, "name": "qa-10", "state": "available"},
            {"id": 2, "name": "qa-1", "state": "stopped"}
        ]);
        assert_eq!(
            parse_environment(&v, "qa-1"),
            Some((2, Some("stopped".into())))
        );
        assert_eq!(parse_environment(&v, "qa-2"), None);
    }

    #[test]
    fn parses_latest_deployment() {
        let v = json!([{
            "id": 7, "ref": "resource-test", "sha": "abc",
            "finished_at": "2026-11-23T16:07:00.462Z",
            "user": {"username": "bot"},
            "deployable": {"web_url": "https://gitlab.com/g/p/-/jobs/9"}
        }]);
        let d = parse_deployment(&v).unwrap();
        assert_eq!(d.id, 7);
        assert_eq!(d.git_ref, "resource-test");
        assert_eq!(d.user.as_deref(), Some("bot"));
        assert_eq!(
            d.job_url.as_deref(),
            Some("https://gitlab.com/g/p/-/jobs/9")
        );
        assert!(d.finished_at.is_some());
        assert_eq!(parse_deployment(&json!([])), None);
    }

    #[test]
    fn item_keys_and_ticket_branches() {
        assert_eq!(
            item_key("https://gitlab.com/a/b/c/-/work_items/14337"),
            Some(("a/b/c".into(), "14337".into()))
        );
        assert_eq!(
            item_key("https://gitlab.com/a/b/-/issues/5#note_1"),
            Some(("a/b".into(), "5".into()))
        );
        assert_eq!(item_key("https://gitlab.com/a/b/-/jobs/5"), None);
        assert_eq!(item_key("https://example.com/a/-/issues/5"), None);
        assert_eq!(
            ticket_of_branch("14337-refuel-two-fuel-items"),
            Some("14337")
        );
        assert_eq!(ticket_of_branch("resource-test"), None);
        assert_eq!(ticket_of_branch("main"), None);
        assert_eq!(ticket_of_branch("v5.24.0"), None);
    }

    #[test]
    fn badge_merge_keeps_up_to_date() {
        let mut b = Vec::new();
        add_badge(&mut b, "qa-1", false);
        add_badge(&mut b, "qa-1", true);
        add_badge(&mut b, "qa-2", false);
        assert_eq!(b.len(), 2);
        assert!(b[0].up_to_date);
        assert!(!b[1].up_to_date);
    }
}
