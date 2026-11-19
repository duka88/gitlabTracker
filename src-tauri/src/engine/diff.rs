//! Pure detection rules (data-model.md "Poll algorithm", research R6–R8).

use std::collections::{BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Utc};

use super::notify_text;
use crate::gitlab::mentions::Mention;
use crate::gitlab::notes::Note;
use crate::model::{
    ChangeKind, GlobalState, ListItem, NotificationKind, PipelineStatus, Tab, TabState, Tracked,
    TrackedState,
};

/// A notification decided by the engine, not yet sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingNotification {
    pub kind: NotificationKind,
    pub tab_id: Option<String>,
    pub item_gid: Option<String>,
    pub web_url: Option<String>,
    /// Bell text and notification title.
    pub text: String,
    /// Notification body (item title, comment snippet), may be empty.
    pub detail: String,
}

/// Per-poll context. `poll_notified` lives for one poll only (keys `new:<gid>`,
/// `change:<gid>:<kind>`); `seen_while_visible` is the persisted global set (FR-023a).
pub struct DiffCtx<'a> {
    pub tab: &'a Tab,
    /// This tab is the active tab of the visible window.
    pub visible: bool,
    pub poll_notified: &'a mut HashSet<String>,
    pub seen_while_visible: &'a mut BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiffOutcome {
    pub new_items: Vec<String>,
    pub notifications: Vec<PendingNotification>,
}

/// New items (FR-019/FR-020/FR-023a). Nothing on the first load (baseline).
pub fn diff_new_items(prev: &TabState, current: &[ListItem], ctx: &mut DiffCtx) -> DiffOutcome {
    let mut out = DiffOutcome::default();
    if !prev.baseline_done {
        return out;
    }
    let mut to_notify: Vec<&ListItem> = Vec::new();
    for item in current {
        if prev.tracked.contains_key(&item.gid) || out.new_items.contains(&item.gid) {
            continue;
        }
        out.new_items.push(item.gid.clone());
        let key = format!("new:{}", item.gid);
        if ctx.visible {
            ctx.seen_while_visible.insert(item.gid.clone());
            ctx.poll_notified.insert(key);
        } else if ctx.tab.notify_new_items && !ctx.poll_notified.contains(&key) {
            ctx.poll_notified.insert(key);
            to_notify.push(item);
        }
    }
    match to_notify.as_slice() {
        [] => {}
        [one] => out.notifications.push(PendingNotification {
            kind: NotificationKind::NewItem,
            tab_id: Some(ctx.tab.id.clone()),
            item_gid: Some(one.gid.clone()),
            web_url: Some(one.web_url.clone()),
            text: notify_text::new_item(&ctx.tab.name, &one.reference, &one.title),
            detail: String::new(),
        }),
        many => out.notifications.push(PendingNotification {
            kind: NotificationKind::NewItems,
            tab_id: Some(ctx.tab.id.clone()),
            item_gid: None,
            web_url: None,
            text: notify_text::new_items(&ctx.tab.name, many.len()),
            detail: many
                .iter()
                .take(3)
                .map(|i| format!("{} {}", i.reference, i.title))
                .collect::<Vec<_>>()
                .join(" · "),
        }),
    }
    out
}

pub fn tracked_state(item: &ListItem) -> TrackedState {
    if let Some(mr) = &item.mr {
        TrackedState::Mr {
            pipeline: mr.pipeline,
            approved: mr.approvals.approved,
            changes_requested_by: mr
                .reviewers
                .iter()
                .filter(|r| r.review_state.as_deref() == Some("REQUESTED_CHANGES"))
                .map(|r| r.person.username.clone())
                .collect(),
            state: mr.state.clone(),
        }
    } else {
        TrackedState::WorkItem {
            status: item
                .work_item
                .as_ref()
                .and_then(|w| w.status.as_ref())
                .map(|s| s.name.clone()),
            labels: item.labels.iter().map(|l| l.title.clone()).collect(),
        }
    }
}

pub fn tracked(item: &ListItem) -> Tracked {
    Tracked {
        kind: item.kind,
        iid: item.iid.clone(),
        reference: item.reference.clone(),
        title: item.title.clone(),
        web_url: item.web_url.clone(),
        project_path: item.project_path.clone(),
        comment_count: item.comment_count,
        updated_at: item.updated_at,
        state: tracked_state(item),
    }
}

/// Changes between the previous poll and now (research R8). Self-requested changes are skipped.
pub fn diff_changes(prev: &Tracked, current: &ListItem, me: Option<&str>) -> Vec<ChangeKind> {
    let mut out = Vec::new();
    match (&prev.state, tracked_state(current)) {
        (
            TrackedState::Mr {
                pipeline: p0,
                approved: a0,
                changes_requested_by: c0,
                state: s0,
            },
            TrackedState::Mr {
                pipeline: p1,
                approved: a1,
                changes_requested_by: c1,
                state: s1,
            },
        ) => {
            if *p0 != Some(PipelineStatus::Failed) && p1 == Some(PipelineStatus::Failed) {
                out.push(ChangeKind::PipelineFailed);
            }
            if !a0 && a1 {
                out.push(ChangeKind::Approved);
            }
            for user in c1.difference(c0) {
                if Some(user.as_str()) == me {
                    continue;
                }
                let name = current
                    .mr
                    .as_ref()
                    .and_then(|m| m.reviewers.iter().find(|r| &r.person.username == user))
                    .map_or_default(|r| r.person.name.clone());
                out.push(ChangeKind::ChangesRequested {
                    by_username: user.clone(),
                    by_name: name,
                });
            }
            if s0 != "merged" && s1 == "merged" {
                out.push(ChangeKind::Merged);
            }
        }
        (
            TrackedState::WorkItem {
                status: st0,
                labels: l0,
            },
            TrackedState::WorkItem {
                status: st1,
                labels: l1,
            },
        ) => {
            if st0.is_some() && st1.is_some() && *st0 != st1 {
                out.push(ChangeKind::StatusChanged {
                    from: st0.clone(),
                    to: st1,
                });
            }
            let added: Vec<String> = l1.difference(l0).cloned().collect();
            let removed: Vec<String> = l0.difference(&l1).cloned().collect();
            if !added.is_empty() || !removed.is_empty() {
                out.push(ChangeKind::LabelsChanged { added, removed });
            }
        }
        _ => {}
    }
    out
}

/// Items whose comments must be fetched: the count went up, or (work items, whose count is
/// threads only) the item was updated. Never for items seen for the first time.
pub fn detect_comment_candidates(prev: &TabState, current: &[ListItem]) -> Vec<String> {
    if !prev.baseline_done {
        return Vec::new();
    }
    current
        .iter()
        .filter(|item| {
            let Some(t) = prev.tracked.get(&item.gid) else {
                return false;
            };
            let count_up = match (t.comment_count, item.comment_count) {
                (Some(a), Some(b)) => b > a,
                (None, Some(b)) => b > 0,
                _ => false,
            };
            let wi_updated = item.work_item.is_some() && item.updated_at > t.updated_at;
            count_up || wi_updated
        })
        .map(|i| i.gid.clone())
        .collect()
}

/// Notes that may notify: not system, not mine, not seen. (`baseline_at` is checked per tab.)
pub fn filter_new_notes<'n>(
    notes: &'n [Note],
    me: Option<&str>,
    seen: &crate::model::BoundedSet<{ crate::model::SEEN_COMMENTS_CAP }>,
) -> Vec<&'n Note> {
    notes
        .iter()
        .filter(|n| !n.system)
        .filter(|n| Some(n.author_username.as_str()) != me)
        .filter(|n| !seen.contains(&n.id))
        .collect()
}

pub fn note_counts_for_tab(note: &Note, baseline_at: Option<DateTime<Utc>>) -> bool {
    baseline_at.is_none_or(|b| note.created_at >= b)
}

/// Mentions (FR-022e/f). The first run records a baseline and sends nothing. A mention adds
/// its note id to `seen_comment_ids`, so the comment itself never notifies again.
pub fn diff_mentions(
    mentions: &[Mention],
    global: &mut GlobalState,
    me: &str,
    visible_gids: &HashSet<String>,
    now: DateTime<Utc>,
) -> Vec<PendingNotification> {
    let fresh_baseline =
        global.mentions_baseline_at.is_none() || global.mentions_user.as_deref() != Some(me);
    if fresh_baseline {
        for m in mentions {
            global.seen_mention_ids.insert(m.id.clone());
        }
        global.mentions_baseline_at = Some(now);
        global.mentions_user = Some(me.to_string());
        return Vec::new();
    }
    let baseline = global.mentions_baseline_at.unwrap_or(now);
    let mut out = Vec::new();
    // Oldest first, so the bell lists them in order.
    let mut ordered: Vec<&Mention> = mentions.iter().collect();
    ordered.sort_by_key(|m| m.created_at);
    for m in ordered {
        if !global.seen_mention_ids.insert(m.id.clone()) || m.created_at < baseline {
            continue;
        }
        if let Some(note) = &m.note_id {
            global.seen_comment_ids.insert(note.clone());
        }
        if m.target_gid
            .as_ref()
            .is_some_and(|g| visible_gids.contains(g))
        {
            continue;
        }
        let who = if m.author_name.is_empty() {
            &m.author_username
        } else {
            &m.author_name
        };
        out.push(PendingNotification {
            kind: NotificationKind::Mention,
            tab_id: None,
            item_gid: m.target_gid.clone(),
            web_url: m.target_url.clone(),
            text: notify_text::mention(who, &m.target_ref),
            detail: notify_text::snippet(&m.body),
        });
    }
    out
}

/// Tab badge: gids with markers, minus the global `seen_while_visible` (FR-030, FR-023a).
pub fn tab_badge<'a>(
    state: &'a TabState,
    seen_while_visible: &BTreeSet<String>,
) -> BTreeSet<&'a str> {
    state
        .marked_gids()
        .into_iter()
        .filter(|g| !seen_while_visible.contains(*g))
        .collect()
}

/// Menu bar count: distinct gids across all tab badges (FR-004). Mentions never count.
pub fn menu_count(
    states: &HashMap<String, TabState>,
    tabs: &[Tab],
    seen_while_visible: &BTreeSet<String>,
) -> u32 {
    let mut all: BTreeSet<&str> = BTreeSet::new();
    for tab in tabs {
        if let Some(s) = states.get(&tab.id) {
            all.extend(tab_badge(s, seen_while_visible));
        }
    }
    all.len() as u32
}

/// Drops gids from `seen_while_visible` once no tab has a marker for them.
pub fn prune_seen_while_visible(
    states: &HashMap<String, TabState>,
    seen_while_visible: &mut BTreeSet<String>,
) {
    let marked: HashSet<&str> = states.values().flat_map(TabState::marked_gids).collect();
    seen_while_visible.retain(|g| marked.contains(g.as_str()));
}
