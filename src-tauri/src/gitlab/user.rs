use serde_json::{Value, json};

use super::client::GitLabClient;
use crate::config::avatar_proxy_url;
use crate::error::{AppError, AppResult};
use crate::model::UserInfo;

pub const CURRENT_USER: &str = include_str!("queries/current_user.graphql");

/// Validates the token: `currentUser` must be non-null (FR-006).
pub async fn fetch_current_user(client: &GitLabClient) -> AppResult<UserInfo> {
    let data = client.graphql(CURRENT_USER, json!({})).await?;
    parse_current_user(&data)
}

pub fn parse_current_user(data: &Value) -> AppResult<UserInfo> {
    let u = data
        .get("currentUser")
        .filter(|v| !v.is_null())
        .ok_or(AppError::TokenRejected)?;
    let s = |k: &str| {
        u.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    Ok(UserInfo {
        id: s("id"),
        username: s("username"),
        name: s("name"),
        avatar_url: avatar_proxy_url(&s("avatarUrl")),
    })
}
