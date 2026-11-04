//! T090: `save_tabs` state handling.
use gitlab_tracker_lib::app_state::apply_tab_edits;
use gitlab_tracker_lib::model::{ListKind, PersistedState, TabState, default_tabs};

fn state_with_baselines(tabs: &[gitlab_tracker_lib::model::Tab]) -> PersistedState {
    let mut st = PersistedState::default();
    for t in tabs {
        let mut ts = TabState::for_url(&t.url);
        ts.baseline_done = true;
        st.tabs.insert(t.id.clone(), ts);
    }
    st
}

#[test]
fn rename_keeps_baseline() {
    let tabs = default_tabs();
    let mut st = state_with_baselines(&tabs);
    let mut edited = tabs.clone();
    edited[0].name = "  Mine  ".into();
    let out = apply_tab_edits(edited, &mut st).unwrap();
    assert_eq!(out[0].name, "Mine");
    assert_eq!(out[0].id, tabs[0].id);
    assert!(st.tabs[&tabs[0].id].baseline_done);
}

#[test]
fn url_change_resets_state() {
    let tabs = default_tabs();
    let mut st = state_with_baselines(&tabs);
    let mut edited = tabs.clone();
    edited[1].url = "https://gitlab.com/groups/login5/-/merge_requests?state=opened".into();
    apply_tab_edits(edited, &mut st).unwrap();
    assert!(!st.tabs[&tabs[1].id].baseline_done);
    assert!(st.tabs[&tabs[0].id].baseline_done);
}

#[test]
fn delete_removes_state_and_reorder_keeps_ids() {
    let tabs = default_tabs();
    let mut st = state_with_baselines(&tabs);
    let edited = vec![tabs[3].clone(), tabs[0].clone()];
    let out = apply_tab_edits(edited, &mut st).unwrap();
    assert_eq!(
        out.iter().map(|t| t.id.clone()).collect::<Vec<_>>(),
        vec![tabs[3].id.clone(), tabs[0].id.clone()]
    );
    assert_eq!(st.tabs.len(), 2);
    assert!(!st.tabs.contains_key(&tabs[1].id));
    assert!(st.tabs[&tabs[3].id].baseline_done);
}

#[test]
fn new_tab_gets_id_and_kind_from_url() {
    let mut st = PersistedState::default();
    let mut t = default_tabs()[0].clone();
    t.id = String::new();
    t.kind = ListKind::WorkItems; // wrong on purpose
    let out = apply_tab_edits(vec![t], &mut st).unwrap();
    assert_ne!(out[0].id, "");
    assert_eq!(out[0].kind, ListKind::MergeRequests);
    assert!(st.tabs.contains_key(&out[0].id));
}

#[test]
fn invalid_input_rejected() {
    let mut st = PersistedState::default();
    let mut t = default_tabs()[0].clone();
    t.url = "https://gitlab.com/x/y/-/merge_requests/1".into();
    assert!(
        apply_tab_edits(vec![t.clone()], &mut st)
            .unwrap_err()
            .contains("single item")
    );
    t.url = default_tabs()[0].url.clone();
    t.name = " ".into();
    assert!(apply_tab_edits(vec![t.clone()], &mut st).is_err());
    t.name = "x".repeat(41);
    assert!(apply_tab_edits(vec![t], &mut st).is_err());
}

#[test]
fn duplicate_ids_get_fresh_ids() {
    let mut st = PersistedState::default();
    let t = default_tabs()[0].clone();
    let out = apply_tab_edits(vec![t.clone(), t], &mut st).unwrap();
    assert_ne!(out[0].id, out[1].id);
}

/// FR-034: the test branch is trimmed, kept only on MR tabs, and changing it resets the
/// baseline like a URL edit.
#[test]
fn test_branch_is_normalized_and_resets_baseline() {
    let tabs = default_tabs();
    let mut st = state_with_baselines(&tabs);
    let mut edited = tabs.clone();
    edited[0].test_branch = Some("  resource-test ".into()); // "My MRs" (MR tab)
    edited[2].test_branch = Some("resource-test".into()); // work-item tab: dropped
    edited[1].test_branch = Some("   ".into()); // blank: dropped
    let out = apply_tab_edits(edited, &mut st).unwrap();
    assert_eq!(out[0].test_branch.as_deref(), Some("resource-test"));
    assert_eq!(out[1].test_branch, None);
    assert_eq!(out[2].kind, ListKind::WorkItems);
    assert_eq!(out[2].test_branch, None);
    assert!(!st.tabs[&tabs[0].id].baseline_done);
    assert!(st.tabs[&tabs[1].id].baseline_done);
    assert!(st.tabs[&tabs[2].id].baseline_done);

    // Saving again unchanged keeps the new baseline id.
    let fp = st.tabs[&tabs[0].id].url_fingerprint.clone();
    apply_tab_edits(out, &mut st).unwrap();
    assert_eq!(st.tabs[&tabs[0].id].url_fingerprint, fp);
}

#[test]
fn test_branch_with_spaces_is_rejected() {
    let tabs = default_tabs();
    let mut st = state_with_baselines(&tabs);
    let mut edited = tabs.clone();
    edited[0].test_branch = Some("resource test".into());
    assert!(apply_tab_edits(edited, &mut st).is_err());
}

/// FR-037: the global test branch is trimmed; blank or with spaces is dropped.
#[test]
fn global_test_branch_is_normalized() {
    use gitlab_tracker_lib::model::normalize_branch;
    assert_eq!(
        normalize_branch(Some(" resource-test ".into())).as_deref(),
        Some("resource-test")
    );
    assert_eq!(normalize_branch(Some("  ".into())), None);
    assert_eq!(normalize_branch(Some("a b".into())), None);
    assert_eq!(normalize_branch(None), None);
}
