//! Tracked envs between polls (FR-040): keep the last good data on errors, and notify about
//! new deployments. Pure, so it's unit-tested without the network.

use std::collections::HashMap;
use std::fmt::Write as _;

use super::diff::PendingNotification;
use crate::model::{EnvStatus, NotificationKind, TrackedEnv};

/// Merges this poll's statuses into the persisted ones and returns the notifications.
///
/// - The first time an env is seen it's a silent baseline.
/// - A failed fetch keeps the previous deployment and shows the error next to it.
/// - Envs no longer tracked are dropped.
pub fn apply_env_statuses(
    stored: &mut HashMap<String, EnvStatus>,
    tracked: &[TrackedEnv],
    fetched: Vec<EnvStatus>,
) -> Vec<PendingNotification> {
    let mut pending = Vec::new();
    for mut new in fetched {
        let key = new.env.key();
        let prev = stored.get(&key);
        if new.error.is_some()
            && let Some(p) = prev
        {
            new.deployment = new.deployment.or_else(|| p.deployment.clone());
            new.web_url = new.web_url.or_else(|| p.web_url.clone());
            new.state = new.state.or_else(|| p.state.clone());
        }
        if let (Some(p), Some(d)) = (prev.and_then(|p| p.deployment.as_ref()), &new.deployment)
            && p.id != d.id
        {
            let short: String = d.sha.chars().take(8).collect();
            let mut detail = short;
            if let Some(u) = &d.user {
                let _ = write!(detail, " · by {u}");
            }
            pending.push(PendingNotification {
                kind: NotificationKind::Deploy,
                tab_id: None,
                item_gid: None,
                web_url: d.job_url.clone().or_else(|| new.web_url.clone()),
                text: format!("🚀 {}: {} deployed", new.env.name, d.git_ref),
                detail,
            });
        }
        stored.insert(key, new);
    }
    let keys: Vec<String> = tracked.iter().map(TrackedEnv::key).collect();
    stored.retain(|k, _| keys.contains(k));
    pending
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::EnvDeployment;

    fn status(id: Option<u64>, error: bool) -> EnvStatus {
        let mut s = EnvStatus::empty(TrackedEnv::new("g/p", "qa-1"));
        s.deployment = id.map(|id| EnvDeployment {
            id,
            git_ref: "resource-test".into(),
            sha: "0123456789abcdef".into(),
            finished_at: None,
            user: Some("bot".into()),
            job_url: Some("https://gitlab.com/g/p/-/jobs/1".into()),
        });
        s.error = error.then(|| "boom".into());
        s
    }

    #[test]
    fn baseline_then_notifies_on_new_deployment() {
        let tracked = vec![TrackedEnv::new("g/p", "qa-1")];
        let mut stored = HashMap::new();
        assert_eq!(
            apply_env_statuses(&mut stored, &tracked, vec![status(Some(1), false)]).len(),
            0
        );
        assert_eq!(
            apply_env_statuses(&mut stored, &tracked, vec![status(Some(1), false)]).len(),
            0
        );
        let n = apply_env_statuses(&mut stored, &tracked, vec![status(Some(2), false)]);
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].text, "🚀 qa-1: resource-test deployed");
        assert_eq!(n[0].detail, "01234567 · by bot");
        assert_eq!(n[0].kind, NotificationKind::Deploy);
    }

    #[test]
    fn error_keeps_previous_deployment() {
        let tracked = vec![TrackedEnv::new("g/p", "qa-1")];
        let mut stored = HashMap::new();
        apply_env_statuses(&mut stored, &tracked, vec![status(Some(1), false)]);
        let n = apply_env_statuses(&mut stored, &tracked, vec![status(None, true)]);
        assert_eq!(n.len(), 0);
        let s = &stored[&tracked[0].key()];
        assert_eq!(s.deployment.as_ref().map(|d| d.id), Some(1));
        assert_eq!(s.error.as_deref(), Some("boom"));
    }

    #[test]
    fn drops_untracked_envs() {
        let mut stored = HashMap::new();
        apply_env_statuses(
            &mut stored,
            &[TrackedEnv::new("g/p", "qa-1")],
            vec![status(Some(1), false)],
        );
        apply_env_statuses(&mut stored, &[], vec![]);
        assert!(stored.is_empty());
    }
}
