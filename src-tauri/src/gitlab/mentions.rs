//! @mentions via the To-Do list (research R7, FR-022e).

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use super::client::GitLabClient;
use super::notes::numeric_id;
use crate::error::AppResult;

pub const MENTIONS: &str = include_str!("queries/mentions.graphql");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mention {
    pub id: String,
    pub author_username: String,
    pub author_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    /// `!123` / `#45`.
    pub target_ref: String,
    pub target_url: Option<String>,
    /// List gid of the target (`Issue/N` is normalised to `WorkItem/N`).
    pub target_gid: Option<String>,
    /// Numeric note id, when the mention is in a comment.
    pub note_id: Option<String>,
}

/// To-dos point at `gid://gitlab/Issue/N`; list rows use `gid://gitlab/WorkItem/N` (same N).
pub fn normalize_gid(gid: &str) -> String {
    match gid.strip_prefix("gid://gitlab/Issue/") {
        Some(n) => format!("gid://gitlab/WorkItem/{n}"),
        None => gid.to_string(),
    }
}

pub fn parse_mentions(data: &Value) -> Vec<Mention> {
    let Some(nodes) = data
        .pointer("/currentUser/todos/nodes")
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    nodes
        .iter()
        .filter_map(|n| {
            let s = |p: &str| n.pointer(p).and_then(Value::as_str).map(str::to_string);
            let target = n.get("target");
            let target_ref = target
                .and_then(|t| t.get("reference"))
                .and_then(Value::as_str)
                .unwrap_or("an item")
                .to_string();
            Some(Mention {
                id: s("/id")?,
                author_username: s("/author/username").unwrap_or_default(),
                author_name: s("/author/name").unwrap_or_default(),
                body: s("/body").unwrap_or_default(),
                created_at: s("/createdAt")?.parse().ok()?,
                target_ref,
                target_url: s("/target/webUrl").or_else(|| s("/targetUrl")),
                target_gid: s("/target/id").map(|g| normalize_gid(&g)),
                note_id: s("/note/id").map(|g| numeric_id(&g)),
            })
        })
        .collect()
}

pub async fn fetch_mentions(client: &GitLabClient) -> AppResult<Vec<Mention>> {
    let data = client.graphql(MENTIONS, json!({})).await?;
    Ok(parse_mentions(&data))
}
