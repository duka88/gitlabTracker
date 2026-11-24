//! Deployment environments tracked from GitLab (FR-040): what's deployed on which env.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Project whose environments are tracked by default.
pub const DEFAULT_ENV_PROJECT: &str = "login5/login-eko/login-eko";

/// One environment to track, set in Settings.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackedEnv {
    /// Full project path, e.g. `login5/login-eko/login-eko`.
    pub project: String,
    /// Environment name as GitLab knows it, e.g. `qa-1`.
    pub name: String,
}

impl TrackedEnv {
    pub fn new(project: &str, name: &str) -> Self {
        TrackedEnv {
            project: project.to_string(),
            name: name.to_string(),
        }
    }

    /// Key in `state.json` and in the view.
    pub fn key(&self) -> String {
        format!("{}#{}", self.project, self.name)
    }
}

pub fn default_tracked_envs() -> Vec<TrackedEnv> {
    vec![TrackedEnv::new(DEFAULT_ENV_PROJECT, "qa-1")]
}

/// Trims, drops blank and duplicate entries, and strips a pasted `https://gitlab.com/` prefix.
pub fn normalize_envs(envs: Vec<TrackedEnv>) -> Result<Vec<TrackedEnv>, String> {
    let mut out: Vec<TrackedEnv> = Vec::with_capacity(envs.len());
    for e in envs {
        let project = e
            .project
            .trim()
            .trim_start_matches("https://gitlab.com/")
            .trim_matches('/')
            .to_string();
        let name = e.name.trim().to_string();
        if project.is_empty() && name.is_empty() {
            continue;
        }
        if project.is_empty() || name.is_empty() {
            return Err("Each environment needs a project path and a name".into());
        }
        if project.chars().any(char::is_whitespace) || name.chars().any(char::is_whitespace) {
            return Err(format!("{project} {name}: names can't contain spaces"));
        }
        let env = TrackedEnv { project, name };
        if !out.contains(&env) {
            out.push(env);
        }
    }
    Ok(out)
}

/// The latest successful deployment to an environment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvDeployment {
    pub id: u64,
    /// Branch or tag that was deployed.
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub sha: String,
    pub finished_at: Option<DateTime<Utc>>,
    /// Username of who started it (often a project bot for pipeline deploys).
    pub user: Option<String>,
    /// The deploy job on gitlab.com.
    pub job_url: Option<String>,
}

/// What the app knows about one tracked environment, persisted in `state.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvStatus {
    pub env: TrackedEnv,
    /// The environment page on gitlab.com.
    pub web_url: Option<String>,
    /// `available` or `stopped`.
    pub state: Option<String>,
    pub deployment: Option<EnvDeployment>,
    pub error: Option<String>,
    pub checked_at: Option<DateTime<Utc>>,
}

impl EnvStatus {
    pub fn empty(env: TrackedEnv) -> Self {
        EnvStatus {
            env,
            web_url: None,
            state: None,
            deployment: None,
            error: None,
            checked_at: None,
        }
    }
}

/// "This item is on env X" chip on a row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvBadge {
    pub env: String,
    /// False when the MR has commits newer than what's deployed.
    pub up_to_date: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_envs() {
        let got = normalize_envs(vec![
            TrackedEnv::new(" https://gitlab.com/a/b/ ", " qa-1 "),
            TrackedEnv::new("a/b", "qa-1"),
            TrackedEnv::new("", ""),
            TrackedEnv::new("a/b", "qa-2"),
        ])
        .unwrap();
        assert_eq!(
            got,
            vec![
                TrackedEnv::new("a/b", "qa-1"),
                TrackedEnv::new("a/b", "qa-2")
            ]
        );
        assert!(normalize_envs(vec![TrackedEnv::new("a/b", "")]).is_err());
        assert!(normalize_envs(vec![TrackedEnv::new("a/b", "qa 1")]).is_err());
    }
}
