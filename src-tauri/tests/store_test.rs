use gitlab_tracker_lib::model::{BoundedSet, NotificationLog, PersistedState, Settings, TabState};
use gitlab_tracker_lib::store::{load_or_default, save};
use pretty_assertions::assert_eq;

#[test]
fn settings_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut s = Settings {
        interval_minutes: 5,
        ..Default::default()
    };
    s.tabs[0].name = "Renamed".into();
    save(&path, &s).unwrap();
    let loaded = load_or_default::<Settings>(&path);
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.value, s);
    // No temp file left behind.
    assert!(!dir.path().join("settings.json.tmp").exists());
}

#[test]
fn missing_file_gives_default_without_warning() {
    let dir = tempfile::tempdir().unwrap();
    let loaded = load_or_default::<Settings>(&dir.path().join("nope.json"));
    assert!(loaded.warning.is_none());
    assert_eq!(loaded.value.tabs.len(), 6);
}

#[test]
fn corrupt_file_falls_back_with_warning() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    std::fs::write(&path, b"{ not json").unwrap();
    let loaded = load_or_default::<PersistedState>(&path);
    assert!(loaded.warning.unwrap().contains("corrupt"));
    assert_eq!(loaded.value, PersistedState::default());
    assert!(dir.path().join("state.json.corrupt").exists());
}

#[test]
fn state_and_log_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    let mut st = PersistedState::default();
    st.global.seen_comment_ids.insert("123");
    st.global
        .seen_while_visible
        .insert("gid://gitlab/MergeRequest/1".into());
    let mut tab = TabState::for_url("https://gitlab.com/groups/a/-/merge_requests");
    tab.baseline_done = true;
    tab.new_items.insert("gid://gitlab/MergeRequest/1".into());
    tab.new_comments
        .insert("gid://gitlab/MergeRequest/1".into(), 2);
    st.tabs.insert("t1".into(), tab);
    save(&path, &st).unwrap();
    assert_eq!(load_or_default::<PersistedState>(&path).value, st);

    let log_path = dir.path().join("notifications.json");
    let log = NotificationLog::default();
    save(&log_path, &log).unwrap();
    assert_eq!(load_or_default::<NotificationLog>(&log_path).value, log);
}

#[test]
fn bounded_set_caps_fifo() {
    let mut s: BoundedSet<3> = BoundedSet::new();
    assert!(s.insert("a"));
    assert!(!s.insert("a"));
    s.insert("b");
    s.insert("c");
    s.insert("d");
    assert_eq!(s.len(), 3);
    assert!(!s.contains("a"), "oldest entry is dropped first");
    assert!(s.contains("d"));
    // Deserializing more than the cap keeps the newest.
    let json = serde_json::to_string(&vec!["1", "2", "3", "4", "5"]).unwrap();
    let s2: BoundedSet<3> = serde_json::from_str(&json).unwrap();
    assert!(!s2.contains("2") && s2.contains("3") && s2.contains("5"));
    assert_eq!(serde_json::to_string(&s2).unwrap(), r#"["3","4","5"]"#);
}

#[test]
fn settings_normalize_fixes_bad_values() {
    let mut s: Settings = serde_json::from_str(r#"{"intervalMinutes": 7, "tabs": []}"#).unwrap();
    s.normalize();
    assert_eq!(s.interval_minutes, 2);
    assert!(s.notify_mentions, "missing field takes the default");
}
