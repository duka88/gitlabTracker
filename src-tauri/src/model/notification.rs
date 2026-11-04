use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const LOG_CAP: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NotificationKind {
    NewItem,
    NewItems,
    Comment,
    Change,
    Mention,
}

/// One bell entry (persisted in `notifications.json`, newest 100).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationEntry {
    pub id: String,
    pub kind: NotificationKind,
    pub tab_id: Option<String>,
    pub item_gid: Option<String>,
    pub web_url: Option<String>,
    pub text: String,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

/// Bell history, newest first.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotificationLog {
    pub entries: Vec<NotificationEntry>,
}

impl NotificationLog {
    pub fn push(&mut self, entry: NotificationEntry) {
        self.entries.insert(0, entry);
        self.entries.truncate(LOG_CAP);
    }

    pub fn unread_count(&self) -> u32 {
        self.entries.iter().filter(|e| !e.read).count() as u32
    }

    pub fn mark_all_read(&mut self) {
        for e in &mut self.entries {
            e.read = true;
        }
    }

    pub fn get(&self, id: &str) -> Option<&NotificationEntry> {
        self.entries.iter().find(|e| e.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(n: u32) -> NotificationEntry {
        NotificationEntry {
            id: n.to_string(),
            kind: NotificationKind::NewItem,
            tab_id: None,
            item_gid: None,
            web_url: None,
            text: format!("e{n}"),
            created_at: Utc::now(),
            read: false,
        }
    }

    #[test]
    fn caps_and_counts() {
        let mut log = NotificationLog::default();
        for n in 0..120 {
            log.push(entry(n));
        }
        assert_eq!(log.entries.len(), LOG_CAP);
        assert_eq!(log.entries[0].id, "119");
        assert_eq!(log.unread_count(), 100);
        log.mark_all_read();
        assert_eq!(log.unread_count(), 0);
    }
}
