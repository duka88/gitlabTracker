//! Latest notes for up to 10 items per request, via aliases (research R6, verification).

use chrono::{DateTime, Utc};
use serde_json::{Value, json};

use super::client::GitLabClient;
use crate::error::AppResult;

pub const BATCH: usize = 10;
pub const NOTES_PER_ITEM: u32 = 20;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NoteTarget {
    Mr { project_path: String, iid: String },
    WorkItem { gid: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Numeric note id; the gid type prefix varies (`Note/`, `DiscussionNote/`, `DiffNote/`).
    pub id: String,
    pub body: String,
    pub system: bool,
    pub author_username: String,
    pub author_name: String,
    pub created_at: DateTime<Utc>,
}

/// `gid://gitlab/DiffNote/123` → `123`.
pub fn numeric_id(gid: &str) -> String {
    gid.rsplit('/').next().unwrap_or(gid).to_string()
}

/// Builds the aliased query for a batch. Values go in as variables, never into the text.
pub fn build_query(targets: &[NoteTarget]) -> (String, Value) {
    let fields = format!(
        "notes(last: {NOTES_PER_ITEM}, filter: ONLY_COMMENTS) {{ nodes {{ id body system createdAt author {{ username name }} }} }}"
    );
    let mut decls = Vec::new();
    let mut body = Vec::new();
    let mut vars = serde_json::Map::new();
    for (i, t) in targets.iter().enumerate() {
        match t {
            NoteTarget::Mr { project_path, iid } => {
                decls.push(format!("$p{i}: ID!, $i{i}: String!"));
                body.push(format!(
                    "n{i}: project(fullPath: $p{i}) {{ mergeRequest(iid: $i{i}) {{ {fields} }} }}"
                ));
                vars.insert(format!("p{i}"), json!(project_path));
                vars.insert(format!("i{i}"), json!(iid));
            }
            NoteTarget::WorkItem { gid } => {
                decls.push(format!("$w{i}: WorkItemID!"));
                body.push(format!(
                    "n{i}: workItem(id: $w{i}) {{ widgets {{ ... on WorkItemWidgetNotes {{ {fields} }} }} }}"
                ));
                vars.insert(format!("w{i}"), json!(gid));
            }
        }
    }
    let query = format!(
        "query RecentNotesBatch({}) {{ {} }}",
        decls.join(", "),
        body.join(" ")
    );
    (query, Value::Object(vars))
}

fn parse_nodes(conn: Option<&Value>) -> Vec<Note> {
    let Some(nodes) = conn.and_then(|c| c.get("nodes")).and_then(Value::as_array) else {
        return Vec::new();
    };
    nodes
        .iter()
        .filter_map(|n| {
            let author = n.get("author");
            Some(Note {
                id: numeric_id(n.get("id")?.as_str()?),
                body: n
                    .get("body")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                system: n.get("system").and_then(Value::as_bool).unwrap_or(false),
                author_username: author
                    .and_then(|a| a.get("username"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                author_name: author
                    .and_then(|a| a.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                created_at: n.get("createdAt")?.as_str()?.parse().ok()?,
            })
        })
        .collect()
}

/// Notes per target, in the same order as `targets`. A deleted item gives an empty list.
pub fn parse_batch(targets: &[NoteTarget], data: &Value) -> Vec<Vec<Note>> {
    targets
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let root = data.get(format!("n{i}"));
            match t {
                NoteTarget::Mr { .. } => parse_nodes(
                    root.and_then(|r| r.get("mergeRequest"))
                        .and_then(|m| m.get("notes")),
                ),
                NoteTarget::WorkItem { .. } => {
                    let widgets = root
                        .and_then(|r| r.get("widgets"))
                        .and_then(Value::as_array);
                    let notes = widgets.and_then(|ws| ws.iter().find_map(|w| w.get("notes")));
                    parse_nodes(notes)
                }
            }
        })
        .collect()
}

/// Fetches notes for all targets in batches of [`BATCH`].
pub async fn fetch_notes(
    client: &GitLabClient,
    targets: &[NoteTarget],
) -> AppResult<Vec<Vec<Note>>> {
    let mut out = Vec::with_capacity(targets.len());
    for chunk in targets.chunks(BATCH) {
        let (q, vars) = build_query(chunk);
        let data = client.graphql(&q, vars).await?;
        out.extend(parse_batch(chunk, &data));
    }
    Ok(out)
}
