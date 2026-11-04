//! T058 / T072 / T080 notification strings.
use gitlab_tracker_lib::engine::notify_text::*;
use gitlab_tracker_lib::model::ChangeKind;
use pretty_assertions::assert_eq;

#[test]
fn new_item_texts() {
    assert_eq!(
        new_item("Reviewing", "!123", "Title"),
        "New in Reviewing: !123 Title"
    );
    assert_eq!(new_items("My tickets", 3), "3 new in My tickets");
}

#[test]
fn comment_texts() {
    assert_eq!(comment("Marko", "!123"), "💬 Marko on !123");
    assert_eq!(comment("Marko", "#45"), "💬 Marko on #45");
    assert_eq!(
        snippet("**Looks** good, see [docs](https://x.y) and `code`."),
        "Looks good, see docs and code."
    );
    let long = format!("{} end", "word ".repeat(40));
    let s = snippet(&long);
    assert!(s.chars().count() <= 100, "{}", s.chars().count());
    assert!(s.ends_with('…'));
    assert!(!s.ends_with("wor…"));
}

#[test]
fn change_texts() {
    assert_eq!(
        change("!123", &ChangeKind::PipelineFailed),
        "❌ Pipeline failed on !123"
    );
    assert_eq!(change("!123", &ChangeKind::Approved), "✅ !123 approved");
    assert_eq!(
        change(
            "!123",
            &ChangeKind::ChangesRequested {
                by_username: "m".into(),
                by_name: "Name".into()
            }
        ),
        "✋ Changes requested on !123 by Name"
    );
    assert_eq!(change("!123", &ChangeKind::Merged), "🔀 !123 merged");
    assert_eq!(
        change(
            "#45",
            &ChangeKind::StatusChanged {
                from: Some("To do".into()),
                to: Some("In progress".into())
            }
        ),
        "#45 To do → In progress"
    );
    assert_eq!(
        change(
            "#45",
            &ChangeKind::LabelsChanged {
                added: vec!["Ready to Merge".into()],
                removed: vec!["In Testing".into()]
            }
        ),
        "#45 +Ready to Merge −In Testing"
    );
}

#[test]
fn mention_text() {
    assert_eq!(mention("Name", "#78"), "📣 Name mentioned you on #78");
}
