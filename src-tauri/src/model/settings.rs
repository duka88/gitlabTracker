use serde::{Deserialize, Serialize};

use super::tab::{Tab, default_tabs};

pub const ALLOWED_INTERVALS: [u8; 4] = [1, 2, 5, 10];

/// Persisted in `settings.json`. The token is never stored here (Keychain only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub schema_version: u32,
    pub interval_minutes: u8,
    pub notify_mentions: bool,
    pub user: Option<UserInfo>,
    pub tabs: Vec<Tab>,
    /// Branch flagged on every MR in every MR tab (FR-037), e.g. `resource-test`.
    pub test_branch: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            schema_version: 1,
            interval_minutes: 2,
            notify_mentions: true,
            user: None,
            tabs: default_tabs(),
            test_branch: None,
        }
    }
}

impl Settings {
    /// Clamps values a hand-edited file may have broken.
    pub fn normalize(&mut self) {
        if !ALLOWED_INTERVALS.contains(&self.interval_minutes) {
            self.interval_minutes = 2;
        }
        self.test_branch = normalize_branch(self.test_branch.take());
        for tab in &mut self.tabs {
            if tab.id.is_empty() {
                tab.id = uuid::Uuid::new_v4().to_string();
            }
        }
    }
}

/// Trims a branch name; blank or containing whitespace becomes `None`.
pub fn normalize_branch(b: Option<String>) -> Option<String> {
    b.map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.chars().any(char::is_whitespace))
}

/// The signed-in user, from the `currentUser` query. This is all the UI learns about the token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub name: String,
    /// `avatar://` proxy URL, or `None`.
    pub avatar_url: Option<String>,
}
