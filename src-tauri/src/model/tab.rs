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
        }
    }
}

/// The 4 default tabs (FR-015, research R14, labels verified in T002).
pub fn default_tabs() -> Vec<Tab> {
    vec![
        Tab::new(
            "My MRs",
            "https://gitlab.com/dashboard/merge_requests?author_username=DusanJ88&state=opened",
            ListKind::MergeRequests,
        ),
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
    ]
}
