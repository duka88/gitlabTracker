//! In-memory app state behind one async mutex, the view builder, and persistence helpers.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use tauri::{AppHandle, Emitter, Manager};

use crate::engine::diff::{menu_count, tab_badge};
use crate::error::Issue;
use crate::model::{
    AppView, MAX_TAB_NAME, NotificationLog, PersistedState, Settings, Tab, TabState, TabView,
    url_fingerprint,
};
use crate::store;
use crate::url_map::parse_tab_url;

pub type Shared = Arc<tokio::sync::Mutex<Core>>;

pub struct Paths {
    pub settings: PathBuf,
    pub state: PathBuf,
    pub notifications: PathBuf,
}

impl Paths {
    pub fn in_dir(dir: &std::path::Path) -> Self {
        Paths {
            settings: dir.join(store::SETTINGS_FILE),
            state: dir.join(store::STATE_FILE),
            notifications: dir.join(store::NOTIFICATIONS_FILE),
        }
    }
}

pub struct Core {
    pub settings: Settings,
    pub state: PersistedState,
    pub log: NotificationLog,
    /// In memory only; never serialized, logged or sent to the webview (constitution II).
    pub token: Option<String>,
    pub active_tab_id: Option<String>,
    pub window_visible: bool,
    pub window_shown_at: DateTime<Utc>,
    pub window_hidden_at: Option<Instant>,
    pub polling: bool,
    pub last_poll_at: Option<DateTime<Utc>>,
    pub global_error: Option<Issue>,
    pub last_notification_at: Option<Instant>,
    pub paths: Paths,
}

impl Core {
    /// Loads the three JSON files (defaults on missing/corrupt files).
    pub fn load(paths: Paths) -> Self {
        let settings = store::load_or_default::<Settings>(&paths.settings);
        let state = store::load_or_default::<PersistedState>(&paths.state);
        let log = store::load_or_default::<NotificationLog>(&paths.notifications);
        for w in [&settings.warning, &state.warning, &log.warning]
            .into_iter()
            .flatten()
        {
            eprintln!("[gitlab-tracker] {w}");
        }
        let mut settings = settings.value;
        settings.normalize();
        let mut state = state.value;
        // Drop state for tabs that no longer exist, and reset tabs whose URL changed.
        let ids: HashSet<&str> = settings.tabs.iter().map(|t| t.id.as_str()).collect();
        state.tabs.retain(|id, _| ids.contains(id.as_str()));
        for t in &settings.tabs {
            if let Some(ts) = state.tabs.get_mut(&t.id)
                && ts.url_fingerprint != url_fingerprint(&t.fingerprint_source())
            {
                *ts = TabState::for_url(&t.fingerprint_source());
            }
        }
        Core {
            active_tab_id: settings.tabs.first().map(|t| t.id.clone()),
            settings,
            state,
            log: log.value,
            token: None,
            window_visible: false,
            window_shown_at: Utc::now(),
            window_hidden_at: None,
            polling: false,
            last_poll_at: None,
            global_error: None,
            last_notification_at: None,
            paths,
        }
    }

    /// The active tab if it exists, else the first tab.
    pub fn active_tab(&self) -> Option<String> {
        let tabs = &self.settings.tabs;
        match &self.active_tab_id {
            Some(id) if tabs.iter().any(|t| &t.id == id) => Some(id.clone()),
            _ => tabs.first().map(|t| t.id.clone()),
        }
    }

    /// Active tab id while the window is visible (FR-023a), else `None`.
    pub fn visible_tab(&self) -> Option<String> {
        if self.window_visible {
            self.active_tab()
        } else {
            None
        }
    }

    pub fn menu_count(&self) -> u32 {
        menu_count(
            &self.state.tabs,
            &self.settings.tabs,
            &self.state.global.seen_while_visible,
        )
    }

    pub fn build_view(&self) -> AppView {
        let swv = &self.state.global.seen_while_visible;
        let tabs = self
            .settings
            .tabs
            .iter()
            .map(|tab| {
                let ts = self.state.tabs.get(&tab.id);
                TabView {
                    tab: tab.clone(),
                    items: ts.map_or_default(|s| s.items.clone()),
                    item_count: ts.map_or(0, |s| s.items.len() as u32),
                    new_count: ts.map_or(0, |s| tab_badge(s, swv).len() as u32),
                    new_items: ts.map_or_default(|s| s.new_items.clone()),
                    new_comments: ts.map_or_default(|s| s.new_comments.clone()),
                    changes: ts.map_or_default(|s| s.changes.clone()),
                    error: ts.and_then(|s| s.error.clone()),
                    warning: ts.and_then(|s| s.warning.clone()),
                    truncated_total: ts.and_then(|s| s.truncated_total),
                    last_updated: ts.and_then(|s| s.last_updated),
                    loaded: ts.is_some_and(|s| s.baseline_done),
                }
            })
            .collect();
        AppView {
            signed_in: self.token.is_some() && self.settings.user.is_some(),
            user: self.settings.user.clone(),
            interval_minutes: self.settings.interval_minutes,
            notify_mentions: self.settings.notify_mentions,
            test_branch: self.settings.test_branch.clone(),
            polling: self.polling,
            last_poll_at: self.last_poll_at,
            active_tab_id: self.active_tab(),
            tabs,
            menu_count: self.menu_count(),
            bell_unread: self.log.unread_count(),
            global_error: self.global_error.clone(),
        }
    }

    pub fn save_settings(&self) {
        report(
            store::save(&self.paths.settings, &self.settings),
            "settings",
        );
    }

    pub fn save_state(&self) {
        report(store::save(&self.paths.state, &self.state), "state");
    }

    pub fn save_log(&self) {
        report(
            store::save(&self.paths.notifications, &self.log),
            "notifications",
        );
    }
}

fn report(r: std::io::Result<()>, what: &str) {
    if let Err(e) = r {
        eprintln!("[gitlab-tracker] couldn't save {what}: {e}");
    }
}

/// Validates an edited tab list and applies it to the state (FR-012/FR-014, T089).
/// New tabs get UUIDs; a changed URL resets that tab's state; deleted tabs lose their state.
pub fn apply_tab_edits(new_tabs: Vec<Tab>, state: &mut PersistedState) -> Result<Vec<Tab>, String> {
    let mut out = Vec::with_capacity(new_tabs.len());
    let mut ids = HashSet::new();
    for (i, mut tab) in new_tabs.into_iter().enumerate() {
        tab.name = tab.name.trim().to_string();
        let n = tab.name.chars().count();
        if n == 0 || n > MAX_TAB_NAME {
            return Err(format!(
                "Tab {}: the name must be 1–{MAX_TAB_NAME} characters",
                i + 1
            ));
        }
        tab.url = tab.url.trim().to_string();
        let q = parse_tab_url(&tab.url).map_err(|e| format!("{}: {e}", tab.name))?;
        tab.kind = q.kind;
        tab.test_branch = match (tab.kind, tab.test_branch.as_deref().map(str::trim)) {
            (crate::model::ListKind::MergeRequests, Some(b)) if !b.is_empty() => {
                if b.chars().any(char::is_whitespace) {
                    return Err(format!("{}: a branch name can't contain spaces", tab.name));
                }
                Some(b.to_string())
            }
            _ => None,
        };
        tab.group_by_week = tab.group_by_week && tab.kind == crate::model::ListKind::WorkItems;
        if tab.id.is_empty() || !ids.insert(tab.id.clone()) {
            tab.id = uuid::Uuid::new_v4().to_string();
            ids.insert(tab.id.clone());
        }
        out.push(tab);
    }
    state.tabs.retain(|id, _| ids.contains(id));
    for tab in &out {
        let fp = url_fingerprint(&tab.fingerprint_source());
        match state.tabs.get_mut(&tab.id) {
            Some(ts) if ts.url_fingerprint != fp => {
                *ts = TabState::for_url(&tab.fingerprint_source());
            }
            Some(_) => {}
            None => {
                state
                    .tabs
                    .insert(tab.id.clone(), TabState::for_url(&tab.fingerprint_source()));
            }
        }
    }
    Ok(out)
}

pub fn emit_view(app: &AppHandle, core: &Core) {
    let _ = app.emit("state://updated", core.build_view());
    crate::tray::set_badge(app, core.menu_count());
}

pub fn shared(app: &AppHandle) -> Shared {
    app.state::<crate::AppCtx>().core.clone()
}
