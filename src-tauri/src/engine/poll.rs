//! Applies one poll's fetched data to the persisted state (data-model.md "Poll algorithm").
//! No I/O: the poller fetches, this decides. Steps: mentions → tabs → comments → merged → counts.

use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Utc};

use super::diff::{
    DiffCtx, PendingNotification, detect_comment_candidates, diff_changes, diff_mentions,
    diff_new_items, filter_new_notes, note_counts_for_tab, prune_seen_while_visible, tracked,
};
use super::notify_text;
use crate::error::AppError;
use crate::gitlab::lists::FetchResult;
use crate::gitlab::mentions::Mention;
use crate::gitlab::merged::MergedInfo;
use crate::gitlab::notes::{Note, NoteTarget};
use crate::model::{
    ChangeKind, ListKind, NotificationKind, PersistedState, Tab, TabState, Tracked, url_fingerprint,
};
use crate::url_map::parse_tab_url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentCandidate {
    pub tab_id: String,
    pub gid: String,
    pub target: NoteTarget,
    pub reference: String,
    pub web_url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MergedCandidate {
    pub tab_id: String,
    pub gid: String,
    pub tracked: Tracked,
}

/// Work queued by `apply_tabs` for the second network round.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FollowUp {
    pub comments: Vec<CommentCandidate>,
    pub merged: Vec<MergedCandidate>,
}

impl FollowUp {
    /// Unique note targets, in first-seen order.
    pub fn note_targets(&self) -> Vec<(String, NoteTarget)> {
        let mut seen = HashSet::new();
        self.comments
            .iter()
            .filter(|c| seen.insert(c.gid.clone()))
            .map(|c| (c.gid.clone(), c.target.clone()))
            .collect()
    }
}

/// One poll pass. Holds the per-poll de-duplication set (not persisted).
pub struct PollRun<'a> {
    pub tabs: &'a [Tab],
    pub me: Option<&'a str>,
    /// Active tab id when the window is visible, else `None` (FR-023a).
    pub visible_tab: Option<&'a str>,
    pub now: DateTime<Utc>,
    poll_notified: HashSet<String>,
}

impl<'a> PollRun<'a> {
    pub fn new(
        tabs: &'a [Tab],
        me: Option<&'a str>,
        visible_tab: Option<&'a str>,
        now: DateTime<Utc>,
    ) -> Self {
        PollRun {
            tabs,
            me,
            visible_tab,
            now,
            poll_notified: HashSet::new(),
        }
    }

    /// Visible tab first, then the others in tab order (the visible-tab rule depends on it).
    fn ordered_tabs(&self) -> Vec<&'a Tab> {
        let mut v: Vec<&Tab> = self.tabs.iter().collect();
        v.sort_by_key(|t| Some(t.id.as_str()) != self.visible_tab);
        v
    }

    fn is_visible(&self, tab_id: &str) -> bool {
        self.visible_tab == Some(tab_id)
    }

    /// Step 2: mentions, before the tabs so de-duplication favours them.
    pub fn apply_mentions(
        &mut self,
        state: &mut PersistedState,
        mentions: &[Mention],
    ) -> Vec<PendingNotification> {
        let Some(me) = self.me else {
            return Vec::new();
        };
        let visible_gids: HashSet<String> = self
            .visible_tab
            .and_then(|id| state.tabs.get(id))
            .map_or_default(|s| s.items.iter().map(|i| i.gid.clone()).collect());
        diff_mentions(mentions, &mut state.global, me, &visible_gids, self.now)
    }

    /// Steps 3–5: store each tab's result, baseline new tabs, detect new items and changes,
    /// and queue comment fetches and merged checks.
    pub fn apply_tabs(
        &mut self,
        state: &mut PersistedState,
        fetched: HashMap<String, Result<FetchResult, AppError>>,
    ) -> (Vec<PendingNotification>, FollowUp) {
        let mut notifications = Vec::new();
        let mut follow = FollowUp::default();
        let mut fetched = fetched;
        for tab in self.ordered_tabs() {
            let Some(result) = fetched.remove(&tab.id) else {
                continue;
            };
            let fingerprint = url_fingerprint(&tab.fingerprint_source());
            let ts = state
                .tabs
                .entry(tab.id.clone())
                .or_insert_with(|| TabState::for_url(&tab.fingerprint_source()));
            if ts.url_fingerprint != fingerprint {
                *ts = TabState::for_url(&tab.fingerprint_source());
            }
            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    if e.is_warning() {
                        ts.warning = Some(e.to_issue());
                    } else if !e.is_global() {
                        ts.error = Some(e.to_issue());
                    }
                    continue;
                }
            };
            if !ts.baseline_done {
                ts.tracked = result
                    .items
                    .iter()
                    .map(|i| (i.gid.clone(), tracked(i)))
                    .collect();
                ts.items = result.items;
                ts.baseline_done = true;
                ts.baseline_at = Some(self.now);
                finish_tab(ts, result.warning, result.truncated_total, self.now);
                continue;
            }

            let visible = self.is_visible(&tab.id);

            // New items.
            let outcome = {
                let mut ctx = DiffCtx {
                    tab,
                    visible,
                    poll_notified: &mut self.poll_notified,
                    seen_while_visible: &mut state.global.seen_while_visible,
                };
                diff_new_items(ts, &result.items, &mut ctx)
            };
            ts.new_items.extend(outcome.new_items.iter().cloned());
            notifications.extend(outcome.notifications);
            let added: HashSet<&str> = outcome.new_items.iter().map(String::as_str).collect();

            // Changes (only items seen before; "new" wins over "changed").
            if tab.notify_changes {
                for item in &result.items {
                    if added.contains(item.gid.as_str()) {
                        continue;
                    }
                    let Some(prev) = ts.tracked.get(&item.gid) else {
                        continue;
                    };
                    for change in diff_changes(prev, item, self.me) {
                        let key = format!("change:{}:{}", item.gid, change.key());
                        let marker = ts.changes.entry(item.gid.clone()).or_default();
                        marker.retain(|c| c.key() != change.key());
                        marker.push(change.clone());
                        if visible {
                            state.global.seen_while_visible.insert(item.gid.clone());
                            self.poll_notified.insert(key);
                        } else if self.poll_notified.insert(key) {
                            notifications.push(PendingNotification {
                                kind: NotificationKind::Change,
                                tab_id: Some(tab.id.clone()),
                                item_gid: Some(item.gid.clone()),
                                web_url: Some(item.web_url.clone()),
                                text: notify_text::change(&item.reference, &change),
                                detail: item.title.clone(),
                            });
                        }
                    }
                }
            }

            // Comment fetches.
            if tab.notify_new_comments {
                for gid in detect_comment_candidates(ts, &result.items) {
                    let Some(item) = result.items.iter().find(|i| i.gid == gid) else {
                        continue;
                    };
                    let target = match item.kind {
                        ListKind::MergeRequests => NoteTarget::Mr {
                            project_path: item.project_path.clone(),
                            iid: item.iid.clone(),
                        },
                        ListKind::WorkItems => NoteTarget::WorkItem {
                            gid: item.gid.clone(),
                        },
                    };
                    follow.comments.push(CommentCandidate {
                        tab_id: tab.id.clone(),
                        gid,
                        target,
                        reference: item.reference.clone(),
                        web_url: item.web_url.clone(),
                    });
                }
            }

            // MRs that left an open-MR list → merged check.
            if tab.kind == ListKind::MergeRequests && tab.notify_changes && is_open_mr_list(tab) {
                let current: HashSet<&str> = result.items.iter().map(|i| i.gid.as_str()).collect();
                for (gid, t) in &ts.tracked {
                    if !current.contains(gid.as_str()) {
                        follow.merged.push(MergedCandidate {
                            tab_id: tab.id.clone(),
                            gid: gid.clone(),
                            tracked: t.clone(),
                        });
                    }
                }
            }

            ts.tracked = result
                .items
                .iter()
                .map(|i| (i.gid.clone(), tracked(i)))
                .collect();
            ts.items = result.items;
            // Markers only for rows still in the list.
            let present: HashSet<String> = ts.items.iter().map(|i| i.gid.clone()).collect();
            ts.new_items.retain(|g| present.contains(g));
            ts.new_comments.retain(|g, _| present.contains(g));
            ts.changes.retain(|g, _| present.contains(g));
            finish_tab(ts, result.warning, result.truncated_total, self.now);
        }
        (notifications, follow)
    }

    /// Step 6: new comments from others. `notes` is keyed by item gid.
    pub fn apply_comments(
        &mut self,
        state: &mut PersistedState,
        candidates: &[CommentCandidate],
        notes: &HashMap<String, Vec<Note>>,
    ) -> Vec<PendingNotification> {
        let mut out = Vec::new();
        let order: Vec<String> = self.ordered_tabs().iter().map(|t| t.id.clone()).collect();
        let mut gids: Vec<&str> = Vec::new();
        for c in candidates {
            if !gids.contains(&c.gid.as_str()) {
                gids.push(&c.gid);
            }
        }
        for gid in gids {
            let Some(item_notes) = notes.get(gid) else {
                continue;
            };
            let mut tabs_for_gid: Vec<&CommentCandidate> =
                candidates.iter().filter(|c| c.gid == gid).collect();
            tabs_for_gid.sort_by_key(|c| order.iter().position(|id| *id == c.tab_id));
            let fresh: Vec<Note> =
                filter_new_notes(item_notes, self.me, &state.global.seen_comment_ids)
                    .into_iter()
                    .cloned()
                    .collect();
            for note in &fresh {
                let mut counted_in: Vec<&CommentCandidate> = Vec::new();
                for c in &tabs_for_gid {
                    let Some(ts) = state.tabs.get_mut(&c.tab_id) else {
                        continue;
                    };
                    if note_counts_for_tab(note, ts.baseline_at) {
                        *ts.new_comments.entry(gid.to_string()).or_insert(0) += 1;
                        counted_in.push(c);
                    }
                }
                let Some(first) = counted_in.first() else {
                    continue;
                };
                if counted_in.iter().any(|c| self.is_visible(&c.tab_id)) {
                    state.global.seen_while_visible.insert(gid.to_string());
                    continue;
                }
                let who = if note.author_name.is_empty() {
                    &note.author_username
                } else {
                    &note.author_name
                };
                out.push(PendingNotification {
                    kind: NotificationKind::Comment,
                    tab_id: Some(first.tab_id.clone()),
                    item_gid: Some(gid.to_string()),
                    web_url: Some(first.web_url.clone()),
                    text: notify_text::comment(who, &first.reference),
                    detail: notify_text::snippet(&note.body),
                });
            }
            for note in item_notes {
                state.global.seen_comment_ids.insert(note.id.clone());
            }
        }
        out
    }

    /// Step 7: MRs that left a tab and were merged by someone else.
    pub fn apply_merged(
        &mut self,
        candidates: &[MergedCandidate],
        results: &[Option<MergedInfo>],
    ) -> Vec<PendingNotification> {
        let mut out = Vec::new();
        for (c, info) in candidates.iter().zip(results) {
            let Some(info) = info else { continue };
            if info.state != "merged" || info.merge_user.as_deref() == self.me && self.me.is_some()
            {
                continue;
            }
            let key = format!("change:{}:merged", c.gid);
            if self.is_visible(&c.tab_id) {
                self.poll_notified.insert(key);
                continue;
            }
            if self.poll_notified.insert(key) {
                out.push(PendingNotification {
                    kind: NotificationKind::Change,
                    tab_id: Some(c.tab_id.clone()),
                    item_gid: Some(c.gid.clone()),
                    web_url: Some(c.tracked.web_url.clone()),
                    text: notify_text::change(&c.tracked.reference, &ChangeKind::Merged),
                    detail: c.tracked.title.clone(),
                });
            }
        }
        out
    }
}

fn finish_tab(
    ts: &mut TabState,
    warning: Option<crate::error::Issue>,
    truncated: Option<u32>,
    now: DateTime<Utc>,
) {
    ts.error = None;
    ts.warning = warning;
    ts.truncated_total = truncated;
    ts.last_updated = Some(now);
}

fn is_open_mr_list(tab: &Tab) -> bool {
    parse_tab_url(&tab.url).is_ok_and(|q| q.filters.state.as_deref() == Some("opened"))
}

/// Step 9: prune `seen_while_visible`; returns the menu count.
pub fn recompute_counts(state: &mut PersistedState, tabs: &[Tab]) -> u32 {
    prune_seen_while_visible(&state.tabs, &mut state.global.seen_while_visible);
    super::diff::menu_count(&state.tabs, tabs, &state.global.seen_while_visible)
}

/// Clears a tab's markers when it is opened (FR-023) and prunes `seen_while_visible`.
pub fn clear_tab_markers(state: &mut PersistedState, tab_id: &str) {
    if let Some(ts) = state.tabs.get_mut(tab_id) {
        ts.clear_markers();
    }
    prune_seen_while_visible(&state.tabs, &mut state.global.seen_while_visible);
}

/// Badge for one tab.
pub fn tab_new_count(state: &PersistedState, tab_id: &str) -> u32 {
    state.tabs.get(tab_id).map_or(0, |ts| {
        super::diff::tab_badge(ts, &state.global.seen_while_visible).len() as u32
    })
}

/// gids with any marker in any tab (used by tests and the view).
pub fn all_marked(state: &PersistedState) -> BTreeSet<String> {
    state
        .tabs
        .values()
        .flat_map(|t| t.marked_gids().into_iter().map(str::to_string))
        .collect()
}
