//! GraphQL client for gitlab.com (research R11, contracts/gitlab-graphql.md).
//! The token is only ever placed in the `Authorization` header; it never appears in errors.

use std::time::Duration;

use serde_json::{Value, json};

use crate::config::GITLAB_HOST;
use crate::error::{AppError, AppResult};

pub const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone)]
pub struct GitLabClient {
    http: reqwest::Client,
    token: String,
}

impl std::fmt::Debug for GitLabClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GitLabClient { token: <redacted> }")
    }
}

/// Shared HTTP client (also used for avatars).
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(concat!("gitlab-tracker/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("HTTP client")
}

impl GitLabClient {
    pub fn new(http: reqwest::Client, token: impl Into<String>) -> Self {
        GitLabClient {
            http,
            token: token.into(),
        }
    }

    /// Runs a query and returns its `data` object.
    pub async fn graphql(&self, query: &str, variables: Value) -> AppResult<Value> {
        let resp = self
            .http
            .post(format!("{GITLAB_HOST}/api/graphql"))
            .bearer_auth(&self.token)
            .json(&json!({ "query": query, "variables": variables }))
            .send()
            .await
            .map_err(|_| AppError::Unreachable)?;
        let status = resp.status().as_u16();
        let body: Value = if status == 200 {
            resp.json().await.map_err(|_| AppError::Unreachable)?
        } else {
            resp.json().await.unwrap_or(Value::Null)
        };
        classify(status, body)
    }
}

/// Maps an HTTP status and GraphQL body to `data` or an [`AppError`] (pure, unit-tested).
pub fn classify(status: u16, mut body: Value) -> AppResult<Value> {
    match status {
        200 => {}
        401 => return Err(AppError::TokenRejected),
        403 => return Err(AppError::NoPermission),
        429 => return Err(AppError::RateLimited),
        500..=599 => return Err(AppError::Query(format!("GitLab error (HTTP {status})"))),
        _ => {
            return Err(first_error(&body).map_or_else(
                || AppError::Query(format!("HTTP {status}")),
                AppError::Query,
            ));
        }
    }
    if let Some(msg) = first_error(&body) {
        let lower = msg.to_lowercase();
        if lower.contains("invalid token") || lower.contains("unauthorized") {
            return Err(AppError::TokenRejected);
        }
        return Err(AppError::Query(msg));
    }
    match body.get_mut("data").map(Value::take) {
        Some(d) if !d.is_null() => Ok(d),
        _ => Err(AppError::Query("Empty response from GitLab".into())),
    }
}

fn first_error(body: &Value) -> Option<String> {
    body.get("errors")?
        .as_array()?
        .first()
        .and_then(|e| e.get("message"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// True for a GraphQL error caused by the work-item `status` argument (research R3).
pub fn is_status_filter_error(err: &AppError) -> bool {
    match err {
        AppError::Query(msg) => {
            let m = msg.to_lowercase();
            m.contains("status")
                && (m.contains("argument")
                    || m.contains("doesn't accept")
                    || m.contains("does not accept")
                    || m.contains("not exist")
                    || m.contains("isn't defined")
                    || m.contains("invalid")
                    || m.contains("not available")
                    || m.contains("unknown")
                    || m.contains("licensed"))
        }
        _ => false,
    }
}
