//! UI view model, the payload of `state://updated` (data-model.md "UI view model").

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::item::ListItem;
use super::settings::UserInfo;
use super::state::ChangeKind;
use super::tab::{ListKind, Tab};
use crate::error::Issue;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppView {
    pub signed_in: bool,
    pub user: Option<UserInfo>,
    pub interval_minutes: u8,
    pub notify_mentions: bool,
    pub polling: bool,
    pub last_poll_at: Option<DateTime<Utc>>,
    pub active_tab_id: Option<String>,
    pub tabs: Vec<TabView>,
    pub menu_count: u32,
    pub bell_unread: u32,
    pub global_error: Option<Issue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabView {
    pub tab: Tab,
    pub items: Vec<ListItem>,
    pub item_count: u32,
    /// Tab badge: marked gids minus `seen_while_visible` (FR-030).
    pub new_count: u32,
    pub new_items: BTreeSet<String>,
    pub new_comments: BTreeMap<String, u32>,
    pub changes: BTreeMap<String, Vec<ChangeKind>>,
    pub error: Option<Issue>,
    pub warning: Option<Issue>,
    pub truncated_total: Option<u32>,
    pub last_updated: Option<DateTime<Utc>>,
    pub loaded: bool,
}

/// Result of `validate_url` (FR-013).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlCheck {
    pub valid: bool,
    pub kind: Option<ListKind>,
    pub scope_label: Option<String>,
    pub filters_summary: Vec<String>,
    pub ignored_params: Vec<String>,
    pub error: Option<String>,
}

/// Result of `open_entry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum OpenTarget {
    #[serde(rename_all = "camelCase")]
    Row {
        tab_id: String,
        gid: String,
    },
    Tab {
        #[serde(rename = "tabId")]
        tab_id: String,
    },
    Browser,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowTarget {
    pub tab_id: String,
    pub gid: String,
}
