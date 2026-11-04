//! Notification strings (pure). `text` is the bell entry and the macOS notification title;
//! `detail` is the notification body.

use crate::model::ChangeKind;
use crate::text::{markdown_to_plain, truncate_words};

pub const SNIPPET_MAX: usize = 100;

pub fn new_item(tab_name: &str, reference: &str, title: &str) -> String {
    format!("New in {tab_name}: {reference} {title}")
}

pub fn new_items(tab_name: &str, count: usize) -> String {
    format!("{count} new in {tab_name}")
}

pub fn comment(author_name: &str, reference: &str) -> String {
    format!("💬 {author_name} on {reference}")
}

/// Markdown-stripped comment text, at most [`SNIPPET_MAX`] characters.
pub fn snippet(body: &str) -> String {
    truncate_words(&markdown_to_plain(body), SNIPPET_MAX)
}

pub fn mention(author_name: &str, reference: &str) -> String {
    format!("📣 {author_name} mentioned you on {reference}")
}

pub fn change(reference: &str, kind: &ChangeKind) -> String {
    match kind {
        ChangeKind::PipelineFailed => format!("❌ Pipeline failed on {reference}"),
        ChangeKind::Approved => format!("✅ {reference} approved"),
        ChangeKind::ChangesRequested {
            by_name,
            by_username,
        } => {
            let who = if by_name.is_empty() {
                by_username
            } else {
                by_name
            };
            format!("✋ Changes requested on {reference} by {who}")
        }
        ChangeKind::Merged => format!("🔀 {reference} merged"),
        ChangeKind::StatusChanged { from, to } => format!(
            "{reference} {} → {}",
            from.as_deref().unwrap_or("none"),
            to.as_deref().unwrap_or("none")
        ),
        ChangeKind::LabelsChanged { added, removed } => {
            let mut parts = vec![reference.to_string()];
            parts.extend(added.iter().map(|l| format!("+{l}")));
            parts.extend(removed.iter().map(|l| format!("−{l}")));
            parts.join(" ")
        }
    }
}
