use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::item::{ListItem, PipelineStatus};
use super::tab::ListKind;
use crate::error::Issue;

/// Per-tab state, persisted in `state.json` keyed by tab id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TabState {
    /// Hash of the tab URL; a mismatch resets the state (FR-019).
    pub url_fingerprint: String,
    pub baseline_done: bool,
    pub baseline_at: Option<DateTime<Utc>>,
    pub items: Vec<ListItem>,
    pub tracked: HashMap<String, Tracked>,
    pub new_items: BTreeSet<String>,
    pub new_comments: BTreeMap<String, u32>,
    pub changes: BTreeMap<String, Vec<ChangeKind>>,
    pub last_updated: Option<DateTime<Utc>>,
    pub error: Option<Issue>,
    pub warning: Option<Issue>,
    pub truncated_total: Option<u32>,
}

impl TabState {
    pub fn for_url(url: &str) -> Self {
        TabState {
            url_fingerprint: url_fingerprint(url),
            ..Default::default()
        }
    }

    /// gids with any unseen marker in this tab.
    pub fn marked_gids(&self) -> BTreeSet<&str> {
        self.new_items
            .iter()
            .chain(self.new_comments.keys())
            .chain(self.changes.keys())
            .map(String::as_str)
            .collect()
    }

    pub fn clear_markers(&mut self) {
        self.new_items.clear();
        self.new_comments.clear();
        self.changes.clear();
    }

    pub fn has_markers(&self) -> bool {
        !(self.new_items.is_empty() && self.new_comments.is_empty() && self.changes.is_empty())
    }
}

/// Stable fingerprint of a tab URL (FNV-1a, so it doesn't change between Rust versions).
pub fn url_fingerprint(url: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in url.trim().bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// What the diff remembers about an item between polls.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tracked {
    pub kind: ListKind,
    pub iid: String,
    pub reference: String,
    pub title: String,
    pub web_url: String,
    pub project_path: String,
    pub comment_count: Option<u32>,
    pub updated_at: DateTime<Utc>,
    pub state: TrackedState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum TrackedState {
    #[serde(rename_all = "camelCase")]
    Mr {
        pipeline: Option<PipelineStatus>,
        approved: bool,
        changes_requested_by: BTreeSet<String>,
        state: String,
    },
    #[serde(rename_all = "camelCase")]
    WorkItem {
        status: Option<String>,
        labels: BTreeSet<String>,
    },
}

/// A detected change (research R8). Serialized for the "changed" tag (FR-022d).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ChangeKind {
    PipelineFailed,
    Approved,
    #[serde(rename_all = "camelCase")]
    ChangesRequested {
        by_username: String,
        by_name: String,
    },
    Merged,
    #[serde(rename_all = "camelCase")]
    StatusChanged {
        from: Option<String>,
        to: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    LabelsChanged {
        added: Vec<String>,
        removed: Vec<String>,
    },
}

impl ChangeKind {
    /// Key part used in `change:<gid>:<key>` for per-poll de-duplication.
    pub fn key(&self) -> String {
        match self {
            ChangeKind::PipelineFailed => "pipeline_failed".into(),
            ChangeKind::Approved => "approved".into(),
            ChangeKind::ChangesRequested { by_username, .. } => {
                format!("changes_requested:{by_username}")
            }
            ChangeKind::Merged => "merged".into(),
            ChangeKind::StatusChanged { .. } => "status".into(),
            ChangeKind::LabelsChanged { .. } => "labels".into(),
        }
    }
}

/// An ordered set that forgets its oldest entries past `CAP` (FIFO).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundedSet<const CAP: usize> {
    order: VecDeque<String>,
    set: HashSet<String>,
}

impl<const CAP: usize> BoundedSet<CAP> {
    pub fn new() -> Self {
        Self {
            order: VecDeque::new(),
            set: HashSet::new(),
        }
    }

    pub fn contains(&self, v: &str) -> bool {
        self.set.contains(v)
    }

    /// Returns true if the value was not present.
    pub fn insert(&mut self, v: impl Into<String>) -> bool {
        let v = v.into();
        if self.set.contains(&v) {
            return false;
        }
        self.set.insert(v.clone());
        self.order.push_back(v);
        while self.order.len() > CAP {
            if let Some(old) = self.order.pop_front() {
                self.set.remove(&old);
            }
        }
        true
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    pub fn clear(&mut self) {
        self.order.clear();
        self.set.clear();
    }
}

impl<const CAP: usize> Serialize for BoundedSet<CAP> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.order.serialize(s)
    }
}

impl<'de, const CAP: usize> Deserialize<'de> for BoundedSet<CAP> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let items: Vec<String> = Vec::deserialize(d)?;
        let mut set = Self::new();
        for v in items {
            set.insert(v);
        }
        Ok(set)
    }
}

pub const SEEN_COMMENTS_CAP: usize = 5_000;
pub const SEEN_MENTIONS_CAP: usize = 2_000;

/// Top-level de-duplication state, persisted in `state.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GlobalState {
    /// Numeric note ids (the gid type prefix varies, research verification).
    pub seen_comment_ids: BoundedSet<SEEN_COMMENTS_CAP>,
    pub seen_mention_ids: BoundedSet<SEEN_MENTIONS_CAP>,
    /// gids whose markers were created while their tab was visible (FR-023a).
    pub seen_while_visible: BTreeSet<String>,
    pub mentions_baseline_at: Option<DateTime<Utc>>,
    /// Username the mention baseline belongs to; a different user resets it.
    pub mentions_user: Option<String>,
}

/// Everything in `state.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistedState {
    pub schema_version: u32,
    pub global: GlobalState,
    pub tabs: HashMap<String, TabState>,
}
