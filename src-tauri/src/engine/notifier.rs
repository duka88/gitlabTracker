//! Sending macOS notifications and routing a notification click (research R9, FR-024).

use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};

use super::diff::PendingNotification;
use crate::model::{NotificationEntry, NotificationLog, PersistedState, RowTarget};

/// Sends one system notification. A trait so a click-aware backend can replace it later.
pub trait Notifier: Send + Sync {
    fn send(&self, title: &str, body: &str);
}

pub struct TauriNotifier {
    app: AppHandle,
    permission_checked: AtomicBool,
}

impl TauriNotifier {
    pub fn new(app: AppHandle) -> Self {
        TauriNotifier {
            app,
            permission_checked: AtomicBool::new(false),
        }
    }
}

impl Notifier for TauriNotifier {
    fn send(&self, title: &str, body: &str) {
        let n = self.app.notification();
        if !self.permission_checked.swap(true, Ordering::SeqCst)
            && let Ok(PermissionState::Prompt | PermissionState::PromptWithRationale) =
                n.permission_state()
        {
            let _ = n.request_permission();
        }
        let mut b = n.builder().title(title);
        if !body.is_empty() {
            b = b.body(body);
        }
        if let Err(e) = b.show() {
            eprintln!("[gitlab-tracker] notification failed: {e}");
        }
    }
}

/// Turns pending notifications into bell entries (newest first in the log) and sends them.
pub fn deliver(
    notifier: &dyn Notifier,
    log: &mut NotificationLog,
    pending: Vec<PendingNotification>,
    now: DateTime<Utc>,
) -> usize {
    let count = pending.len();
    for p in pending {
        notifier.send(&p.text, &p.detail);
        log.push(NotificationEntry {
            id: uuid::Uuid::new_v4().to_string(),
            kind: p.kind,
            tab_id: p.tab_id,
            item_gid: p.item_gid,
            web_url: p.web_url,
            text: p.text,
            created_at: now,
            read: false,
        });
    }
    count
}

/// Where a click should lead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClickRoute {
    Row(RowTarget),
    Bell,
}

/// The tab that holds `gid`, preferring `preferred`.
pub fn tab_with_item(
    state: &PersistedState,
    tab_order: &[String],
    gid: &str,
    preferred: Option<&str>,
) -> Option<String> {
    let has = |id: &str| {
        state
            .tabs
            .get(id)
            .is_some_and(|t| t.items.iter().any(|i| i.gid == gid))
    };
    if let Some(p) = preferred.filter(|p| has(p)) {
        return Some(p.to_string());
    }
    tab_order.iter().find(|id| has(id)).cloned()
}

/// Exactly one unread entry sent since the window was last shown, and its item is in a tab
/// → that row. Otherwise the bell popup (research R9).
pub fn route_click(
    log: &NotificationLog,
    state: &PersistedState,
    tab_order: &[String],
    since: DateTime<Utc>,
) -> ClickRoute {
    let recent: Vec<&NotificationEntry> = log
        .entries
        .iter()
        .filter(|e| !e.read && e.created_at >= since)
        .collect();
    if let [one] = recent.as_slice()
        && let Some(gid) = &one.item_gid
        && let Some(tab_id) = tab_with_item(state, tab_order, gid, one.tab_id.as_deref())
    {
        return ClickRoute::Row(RowTarget {
            tab_id,
            gid: gid.clone(),
        });
    }
    ClickRoute::Bell
}
