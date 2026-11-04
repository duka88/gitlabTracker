//! Was an MR that left a tab merged? (FR-022b, research R8). Aliased batches of 10.

use serde_json::{Value, json};

use super::client::GitLabClient;
use crate::error::AppResult;

pub const BATCH: usize = 10;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergedInfo {
    pub state: String,
    pub merge_user: Option<String>,
}

pub fn build_query(targets: &[(String, String)]) -> (String, Value) {
    let mut decls = Vec::new();
    let mut body = Vec::new();
    let mut vars = serde_json::Map::new();
    for (i, (path, iid)) in targets.iter().enumerate() {
        decls.push(format!("$p{i}: ID!, $i{i}: String!"));
        body.push(format!(
            "m{i}: project(fullPath: $p{i}) {{ mergeRequest(iid: $i{i}) {{ state mergedAt mergeUser {{ username }} }} }}"
        ));
        vars.insert(format!("p{i}"), json!(path));
        vars.insert(format!("i{i}"), json!(iid));
    }
    (
        format!(
            "query MergedCheckBatch({}) {{ {} }}",
            decls.join(", "),
            body.join(" ")
        ),
        Value::Object(vars),
    )
}

/// One entry per target; `None` when the MR or project is gone.
pub fn parse_batch(len: usize, data: &Value) -> Vec<Option<MergedInfo>> {
    (0..len)
        .map(|i| {
            let mr = data.get(format!("m{i}"))?.get("mergeRequest")?;
            Some(MergedInfo {
                state: mr.get("state")?.as_str()?.to_string(),
                merge_user: mr
                    .get("mergeUser")
                    .and_then(|u| u.get("username"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        })
        .collect()
}

/// `targets` are `(project_path, iid)` pairs.
pub async fn check_merged(
    client: &GitLabClient,
    targets: &[(String, String)],
) -> AppResult<Vec<Option<MergedInfo>>> {
    let mut out = Vec::with_capacity(targets.len());
    for chunk in targets.chunks(BATCH) {
        let (q, vars) = build_query(chunk);
        let data = client.graphql(&q, vars).await?;
        out.extend(parse_batch(chunk.len(), &data));
    }
    Ok(out)
}
