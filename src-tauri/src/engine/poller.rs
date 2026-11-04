//! The poll loop: one tokio task, interval + manual refresh + wake watchdog, never overlapping
//! (FR-018, research R11). Network here; decisions in `engine::poll`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

use chrono::Utc;
use futures::StreamExt;
use tauri::{AppHandle, Emitter};
use tokio::sync::{Notify, watch};
use tokio::time::MissedTickBehavior;

use super::notifier::{Notifier, deliver};
use super::poll::{PollRun, recompute_counts};
use crate::app_state::{Shared, emit_view};
use crate::error::{AppError, Issue};
use crate::gitlab::GitLabClient;
use crate::gitlab::lists::{FetchResult, fetch_tab};
use crate::gitlab::mentions::fetch_mentions;
use crate::gitlab::merged::check_merged;
use crate::gitlab::notes::fetch_notes;
use crate::url_map::parse_tab_url;

const MAX_CONCURRENT_TABS: usize = 4;
const WATCHDOG_EVERY: Duration = Duration::from_secs(30);
const WAKE_JUMP: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct PollerHandle {
    refresh: Arc<Notify>,
    interval_tx: Arc<watch::Sender<u8>>,
    in_flight: Arc<AtomicBool>,
}

impl PollerHandle {
    /// Starts a poll unless one is running (FR-018).
    pub fn refresh(&self) {
        if !self.in_flight.load(Ordering::SeqCst) {
            self.refresh.notify_one();
        }
    }

    pub fn is_polling(&self) -> bool {
        self.in_flight.load(Ordering::SeqCst)
    }

    /// Restarts the timer with a new interval.
    pub fn set_interval(&self, minutes: u8) {
        let _ = self.interval_tx.send(minutes);
    }
}

pub fn spawn(
    app: AppHandle,
    core: Shared,
    http: reqwest::Client,
    notifier: Arc<dyn Notifier>,
    minutes: u8,
) -> PollerHandle {
    let (tx, mut rx) = watch::channel(minutes);
    let handle = PollerHandle {
        refresh: Arc::new(Notify::new()),
        interval_tx: Arc::new(tx),
        in_flight: Arc::new(AtomicBool::new(false)),
    };
    let h = handle.clone();
    tauri::async_runtime::spawn(async move {
        // First poll right away.
        run_guarded(&app, &core, &http, notifier.as_ref(), &h.in_flight).await;
        loop {
            let mins = (*rx.borrow_and_update()).max(1);
            let period = Duration::from_secs(u64::from(mins) * 60);
            let mut ticker = tokio::time::interval_at(tokio::time::Instant::now() + period, period);
            ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = ticker.tick() => run_guarded(&app, &core, &http, notifier.as_ref(), &h.in_flight).await,
                    () = h.refresh.notified() => run_guarded(&app, &core, &http, notifier.as_ref(), &h.in_flight).await,
                    changed = rx.changed() => {
                        if changed.is_err() { return; }
                        break;
                    }
                }
            }
        }
    });
    spawn_watchdog(handle.clone());
    handle
}

/// Tokio's clock stops while the Mac sleeps; a wall-clock jump means we just woke up (R11).
fn spawn_watchdog(handle: PollerHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last = SystemTime::now();
        loop {
            tokio::time::sleep(WATCHDOG_EVERY).await;
            let now = SystemTime::now();
            let elapsed = now.duration_since(last).unwrap_or_default();
            last = now;
            if elapsed > WATCHDOG_EVERY + WAKE_JUMP {
                handle.refresh();
            }
        }
    });
}

async fn run_guarded(
    app: &AppHandle,
    core: &Shared,
    http: &reqwest::Client,
    notifier: &dyn Notifier,
    in_flight: &AtomicBool,
) {
    if in_flight.swap(true, Ordering::SeqCst) {
        return;
    }
    run_poll(app, core, http, notifier).await;
    in_flight.store(false, Ordering::SeqCst);
}

/// One full pass (data-model.md "Poll algorithm").
pub async fn run_poll(
    app: &AppHandle,
    core: &Shared,
    http: &reqwest::Client,
    notifier: &dyn Notifier,
) {
    let _ = app.emit("poll://started", ());
    let (token, tabs, me, visible, notify_mentions) = {
        let mut c = core.lock().await;
        c.polling = true;
        (
            c.token.clone(),
            c.settings.tabs.clone(),
            c.settings.user.as_ref().map(|u| u.username.clone()),
            c.visible_tab(),
            c.settings.notify_mentions,
        )
    };
    let Some(token) = token else {
        let mut c = core.lock().await;
        c.polling = false;
        c.global_error = Some(AppError::NoToken.to_issue());
        emit_view(app, &c);
        return;
    };
    let client = GitLabClient::new(http.clone(), token);

    // Mentions (step 2) and tabs (step 3), fetched concurrently.
    let mentions_fut = async {
        if notify_mentions && me.is_some() {
            Some(fetch_mentions(&client).await)
        } else {
            None
        }
    };
    // Each future owns its data (avoids higher-ranked lifetime issues in the spawned task).
    let tab_futs: Vec<_> = tabs
        .iter()
        .map(|t| {
            let client = client.clone();
            let id = t.id.clone();
            let url = t.url.clone();
            async move {
                let result = match parse_tab_url(&url) {
                    Ok(q) => fetch_tab(&client, &q).await,
                    Err(e) => Err(AppError::Invalid(e.0)),
                };
                (id, result)
            }
        })
        .collect();
    let tabs_fut = futures::stream::iter(tab_futs)
        .buffer_unordered(MAX_CONCURRENT_TABS)
        .collect::<Vec<(String, Result<FetchResult, AppError>)>>();
    let (mentions, fetched) = tokio::join!(mentions_fut, tabs_fut);

    // A global error (token rejected, offline) shows once, above the tabs (FR-031).
    let mut global: Option<Issue> = fetched
        .iter()
        .filter_map(|(_, r)| r.as_ref().err())
        .chain(mentions.as_ref().and_then(|m| m.as_ref().err()))
        .find(|e| e.is_global())
        .map(AppError::to_issue);
    let fetched: HashMap<String, Result<FetchResult, AppError>> = fetched.into_iter().collect();

    let now = Utc::now();
    let mut run = PollRun::new(&tabs, me.as_deref(), visible.as_deref(), now);
    let (mut pending, follow) = {
        let mut c = core.lock().await;
        let mut pending = match &mentions {
            Some(Ok(m)) => run.apply_mentions(&mut c.state, m),
            _ => Vec::new(),
        };
        let (n, follow) = run.apply_tabs(&mut c.state, fetched);
        pending.extend(n);
        (pending, follow)
    };

    // Steps 6–7: comments and merged checks (second network round).
    let targets = follow.note_targets();
    let notes = if targets.is_empty() {
        Ok(Vec::new())
    } else {
        let only: Vec<_> = targets.iter().map(|(_, t)| t.clone()).collect();
        fetch_notes(&client, &only).await
    };
    let mut seen = HashSet::new();
    let merged_cands: Vec<_> = follow
        .merged
        .iter()
        .filter(|c| seen.insert(c.gid.clone()))
        .cloned()
        .collect();
    let merged_targets: Vec<(String, String)> = merged_cands
        .iter()
        .map(|c| (c.tracked.project_path.clone(), c.tracked.iid.clone()))
        .collect();
    let merged = if merged_targets.is_empty() {
        Ok(Vec::new())
    } else {
        check_merged(&client, &merged_targets).await
    };

    let mut c = core.lock().await;
    match notes {
        Ok(lists) => {
            let by_gid: HashMap<String, _> =
                targets.into_iter().map(|(g, _)| g).zip(lists).collect();
            pending.extend(run.apply_comments(&mut c.state, &follow.comments, &by_gid));
        }
        Err(e) => {
            if e.is_global() {
                global = global.or(Some(e.to_issue()));
            }
            eprintln!("[gitlab-tracker] comment fetch failed: {e}");
        }
    }
    match merged {
        Ok(results) => pending.extend(run.apply_merged(&merged_cands, &results)),
        Err(e) => eprintln!("[gitlab-tracker] merged check failed: {e}"),
    }

    // Step 9: counts, persist, notify, emit.
    let tab_list = c.settings.tabs.clone();
    recompute_counts(&mut c.state, &tab_list);
    if deliver(notifier, &mut c.log, pending, now) > 0 {
        c.last_notification_at = Some(Instant::now());
        c.save_log();
    }
    c.global_error = global;
    c.polling = false;
    c.last_poll_at = Some(now);
    c.save_state();
    emit_view(app, &c);
}
