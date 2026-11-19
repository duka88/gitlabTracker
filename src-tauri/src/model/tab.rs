use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListKind {
    MergeRequests,
    WorkItems,
}

/// A user-defined list: a name plus a pasted GitLab list URL.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tab {
    /// UUID v4, stable across renames and reorders. Empty for a tab the UI just added.
    #[serde(default)]
    pub id: String,
    pub name: String,
    pub url: String,
    pub kind: ListKind,
    #[serde(default = "yes")]
    pub notify_new_items: bool,
    #[serde(default = "yes")]
    pub notify_new_comments: bool,
    #[serde(default = "yes")]
    pub notify_changes: bool,
    /// MR tabs only: show just the MRs whose source branch is merged into this branch
    /// (e.g. a shared test branch), FR-034.
    #[serde(default)]
    pub test_branch: Option<String>,
    /// Work-item tabs only: show rows grouped by month, then by week, with counts
    /// (closed date, else last update).
    #[serde(default)]
    pub group_by_week: bool,
    /// The tab's default row order; the list view can change it until restart.
    #[serde(default)]
    pub sort: TabSort,
}

/// How a tab's rows are ordered in the list view (sorted in the UI, not by GitLab).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TabSort {
    /// The order the tab's URL asks GitLab for.
    #[default]
    Gitlab,
    /// By label titles, A–Z; rows without labels last.
    Label,
    /// Last updated, newest first.
    Updated,
    /// Created, newest first.
    Created,
    /// Title, A–Z.
    Title,
    /// `priority::` label, P1 first; ties by stage.
    Priority,
    /// `state::` label in workflow order (In Development → Ready for Testing → In Testing →
    /// Ready to Merge); ties by priority.
    Stage,
}

fn yes() -> bool {
    true
}

pub const MAX_TAB_NAME: usize = 40;

impl Tab {
    pub fn new(name: &str, url: &str, kind: ListKind) -> Self {
        Tab {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            url: url.to_string(),
            kind,
            notify_new_items: true,
            notify_new_comments: true,
            notify_changes: true,
            test_branch: None,
            group_by_week: false,
            sort: TabSort::Gitlab,
        }
    }

    /// What identifies the tab's list: the URL, plus the test branch when set. A change
    /// resets the tab's baseline like a URL edit (FR-014, FR-034).
    pub fn fingerprint_source(&self) -> String {
        match &self.test_branch {
            Some(b) => format!("{}\ntest_branch={b}", self.url),
            None => self.url.clone(),
        }
    }
}

/// The 6 default tabs (FR-015, research R14, labels verified in T002).
pub fn default_tabs() -> Vec<Tab> {
    vec![
        Tab::new(
            "My MRs",
            "https://gitlab.com/dashboard/merge_requests?author_username=DusanJ88&state=opened",
            ListKind::MergeRequests,
        ),
        // My open MRs that are merged into the shared test branch (FR-034).
        Tab {
            test_branch: Some("resource-test".into()),
            ..Tab::new(
                "On resource-test",
                "https://gitlab.com/dashboard/merge_requests?author_username=DusanJ88&state=opened",
                ListKind::MergeRequests,
            )
        },
        Tab::new(
            "Reviewing",
            "https://gitlab.com/dashboard/merge_requests?reviewer_username=DusanJ88&state=opened",
            ListKind::MergeRequests,
        ),
        Tab::new(
            "My tickets",
            "https://gitlab.com/groups/login5/-/work_items?state=opened&assignee_username[]=DusanJ88&status=To%20do&not[milestone_title]=Backlog&not[label_name][]=state::Ready%20to%20Merge&not[label_name][]=state::Ready%20for%20Testing&not[label_name][]=state::In%20Testing",
            ListKind::WorkItems,
        ),
        Tab::new(
            "Ready to merge",
            "https://gitlab.com/groups/login5/-/work_items?state=opened&assignee_username[]=DusanJ88&label_name[]=state::Ready%20to%20Merge&not[milestone_title]=Backlog",
            ListKind::WorkItems,
        ),
        // My closed tickets, counted per month and week; no notifications (FR-038).
        Tab {
            group_by_week: true,
            notify_new_items: false,
            notify_new_comments: false,
            notify_changes: false,
            ..Tab::new(
                "Done",
                "https://gitlab.com/groups/login5/-/work_items?state=closed&assignee_username[]=DusanJ88&sort=closed_at_desc",
                ListKind::WorkItems,
            )
        },
    ]
}
