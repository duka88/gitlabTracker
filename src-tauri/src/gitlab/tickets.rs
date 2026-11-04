//! FR-036b: fills in the ticket of MRs whose `workItemRelations` don't include it, by the
//! ticket number at the start of the branch name, with one query per project.

use std::collections::{BTreeSet, HashMap};

use serde_json::{Value, json};

use super::client::GitLabClient;
use super::parse::{LinkedWorkItemRaw, branch_ticket_iid, ticket_blocks_merge, ticket_from};
use crate::error::AppResult;
use crate::model::{LinkedTicket, ListItem, MergeStatusTag};

pub const TICKETS_BY_IID: &str = include_str!("queries/tickets_by_iid.graphql");

/// Projects → ticket numbers still needed (MRs without a ticket whose branch starts with one).
pub fn missing_tickets(items: &[ListItem]) -> HashMap<String, BTreeSet<String>> {
    let mut out: HashMap<String, BTreeSet<String>> = HashMap::new();
    for i in items {
        let Some(mr) = &i.mr else { continue };
        if mr.ticket.is_some() || i.project_path.is_empty() {
            continue;
        }
        if let Some(iid) = branch_ticket_iid(&mr.source_branch) {
            out.entry(i.project_path.clone()).or_default().insert(iid);
        }
    }
    out
}

/// Sets the found tickets and applies the ticket rule to the merge-status tag (FR-036).
pub fn apply_tickets(items: &mut [ListItem], found: &HashMap<(String, String), LinkedTicket>) {
    for i in items {
        let project = i.project_path.clone();
        let Some(mr) = i.mr.as_mut() else { continue };
        if mr.ticket.is_some() {
            continue;
        }
        let Some(iid) = branch_ticket_iid(&mr.source_branch) else {
            continue;
        };
        if let Some(t) = found.get(&(project, iid)) {
            mr.ticket = Some(t.clone());
            if mr.merge_status == Some(MergeStatusTag::ReadyToMerge)
                && ticket_blocks_merge(mr.ticket.as_ref())
            {
                mr.merge_status = Some(MergeStatusTag::Approved);
            }
        }
    }
}

pub fn parse_tickets(data: &Value) -> Vec<LinkedWorkItemRaw> {
    data.pointer("/project/workItems/nodes")
        .and_then(|n| serde_json::from_value(n.clone()).ok())
        .unwrap_or_default()
}

/// Looks up and fills the missing tickets. A failed lookup leaves the MRs without a ticket
/// rather than failing the tab.
pub async fn fill_missing_tickets(client: &GitLabClient, items: &mut [ListItem]) -> AppResult<()> {
    let mut found = HashMap::new();
    for (project, iids) in missing_tickets(items) {
        let iids: Vec<String> = iids.into_iter().collect();
        let Ok(data) = client
            .graphql(TICKETS_BY_IID, json!({ "fullPath": project, "iids": iids }))
            .await
        else {
            continue;
        };
        for w in parse_tickets(&data) {
            found.insert((project.clone(), w.iid.clone()), ticket_from(&w));
        }
    }
    apply_tickets(items, &found);
    Ok(())
}
