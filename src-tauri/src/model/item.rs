use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::tab::ListKind;

/// A row in a tab: an MR or a work item (data-model.md "`ListItem`").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    /// Global id (`gid://gitlab/MergeRequest/1`, `gid://gitlab/WorkItem/2`), the cross-tab key.
    pub gid: String,
    pub kind: ListKind,
    pub iid: String,
    /// `!123` or `#45`.
    pub reference: String,
    pub title: String,
    pub web_url: String,
    pub project_path: String,
    pub author: Option<Person>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub labels: Vec<Label>,
    pub assignees: Vec<Person>,
    pub milestone: Option<String>,
    /// MR: user notes. Work item: user discussions (research, verification results).
    pub comment_count: Option<u32>,
    pub upvotes: u32,
    pub downvotes: u32,
    /// At most 300 characters of plain text.
    pub description_preview: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mr: Option<MrFields>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_item: Option<WorkItemFields>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MrFields {
    pub draft: bool,
    pub target_branch: String,
    /// `opened`, `merged`, `closed`, `locked`.
    pub state: String,
    pub pipeline: Option<PipelineStatus>,
    pub approvals: Approvals,
    /// Resolvable threads not yet resolved (FR-025b).
    #[serde(default)]
    pub unresolved_threads: u32,
    #[serde(default)]
    pub source_branch: String,
    /// Latest commit of the source branch.
    #[serde(default)]
    pub head_sha: String,
    /// Set in tabs with a test branch, or on every MR when a global test branch is set
    /// (FR-034, FR-037).
    #[serde(default)]
    pub on_branch: Option<OnBranch>,
    /// The ticket this MR resolves (FR-036).
    #[serde(default)]
    pub ticket: Option<LinkedTicket>,
    pub reviewers: Vec<Reviewer>,
    pub merge_status: Option<MergeStatusTag>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkItemFields {
    pub type_name: String,
    pub type_icon: Option<String>,
    pub confidential: bool,
    pub iteration: Option<String>,
    pub due_date: Option<NaiveDate>,
    pub weight: Option<u32>,
    pub status: Option<WorkStatus>,
    pub health: Option<Health>,
    pub children: Option<Children>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub username: String,
    pub name: String,
    /// `avatar://localhost/<percent-encoded GitLab URL>`, never a GitLab URL (research R16).
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Label {
    pub title: String,
    pub color: String,
    pub text_color: String,
    /// `a::b::c` → scope `a::b`, value `c` (split on the last `::`, as GitLab does).
    pub scope: Option<String>,
    pub value: String,
}

impl Label {
    pub fn new(title: &str, color: &str, text_color: &str) -> Self {
        let (scope, value) = split_scoped(title);
        Label {
            title: title.to_string(),
            color: color.to_string(),
            text_color: text_color.to_string(),
            scope,
            value,
        }
    }
}

/// Splits a scoped label on its last `::`. Unscoped labels have no scope.
pub fn split_scoped(title: &str) -> (Option<String>, String) {
    match title.rfind("::") {
        Some(i) if i > 0 && i + 2 < title.len() => {
            (Some(title[..i].to_string()), title[i + 2..].to_string())
        }
        _ => (None, title.to_string()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PipelineStatus {
    Success,
    Failed,
    Running,
    Pending,
    Canceled,
    Skipped,
    Manual,
    Created,
    Scheduled,
    Other,
}

impl PipelineStatus {
    pub fn from_graphql(s: &str) -> Self {
        match s {
            "SUCCESS" => Self::Success,
            "FAILED" => Self::Failed,
            "RUNNING" | "CANCELING" => Self::Running,
            "PENDING" | "WAITING_FOR_RESOURCE" | "PREPARING" | "WAITING_FOR_CALLBACK" => {
                Self::Pending
            }
            "CANCELED" => Self::Canceled,
            "SKIPPED" => Self::Skipped,
            "MANUAL" => Self::Manual,
            "CREATED" => Self::Created,
            "SCHEDULED" => Self::Scheduled,
            _ => Self::Other,
        }
    }
}

/// The work item an MR resolves (FR-036): its "closes" link, else the linked item whose
/// number starts the source branch name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkedTicket {
    pub reference: String,
    pub title: String,
    pub web_url: String,
    pub open: bool,
    /// The `state::<value>` label, the team's workflow state (e.g. "Ready for Testing").
    pub state_label: Option<Label>,
    /// GitLab's own status widget, when it has one.
    pub status: Option<WorkStatus>,
}

/// Workflow state that allows merging (FR-036), compared case-insensitively.
pub const READY_TO_MERGE_STATE: &str = "Ready to Merge";

/// Whether an MR's source branch is merged into a tab's test branch (FR-034).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnBranch {
    pub branch: String,
    /// False when commits were pushed after the last merge into the test branch.
    pub up_to_date: bool,
    /// When the source branch was last merged into the test branch, if found.
    pub merged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeStatusTag {
    ReadyToMerge,
    /// Approved and mergeable, but the linked ticket isn't in the ready-to-merge state.
    Approved,
    NeedsApproval,
    CiFailed,
    Conflicts,
    ChangesRequested,
    UnresolvedThreads,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Approvals {
    pub approved: bool,
    pub required: u32,
    pub given: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reviewer {
    #[serde(flatten)]
    pub person: Person,
    pub approved: bool,
    /// GraphQL `MergeRequestReviewState`, e.g. `REQUESTED_CHANGES`.
    pub review_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkStatus {
    pub name: String,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Health {
    OnTrack,
    NeedsAttention,
    AtRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Children {
    pub done: u32,
    pub total: u32,
}

#[cfg(test)]
mod tests {
    use super::split_scoped;

    #[test]
    fn scoped_split() {
        assert_eq!(split_scoped("a::b::c"), (Some("a::b".into()), "c".into()));
        assert_eq!(
            split_scoped("state::Ready to Merge"),
            (Some("state".into()), "Ready to Merge".into())
        );
        assert_eq!(split_scoped("plain"), (None, "plain".into()));
        assert_eq!(split_scoped("E2E::*"), (Some("E2E".into()), "*".into()));
        assert_eq!(split_scoped("::x"), (None, "::x".into()));
        assert_eq!(split_scoped("x::"), (None, "x::".into()));
    }
}
