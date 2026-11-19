//! Plain-text helpers: Markdown stripping and word-boundary truncation (pure).

/// Strips the Markdown/HTML that GitLab descriptions and comments commonly contain and
/// collapses whitespace into single spaces.
pub fn markdown_to_plain(md: &str) -> String {
    let mut out = String::with_capacity(md.len());
    let mut in_fence = false;
    for raw_line in md.lines() {
        let line = raw_line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let mut l = line;
        // Block markers: headings, quotes, list bullets, numbered lists, rules.
        loop {
            let before = l;
            l = l.trim_start_matches('#').trim_start();
            l = l.trim_start_matches('>').trim_start();
            for bullet in ["- [ ] ", "- [x] ", "* [ ] ", "* [x] ", "- ", "* ", "+ "] {
                if let Some(rest) = l.strip_prefix(bullet) {
                    l = rest;
                }
            }
            if let Some(pos) = l.find(". ")
                && pos > 0
                && pos <= 3
                && l[..pos].chars().all(|c| c.is_ascii_digit())
            {
                l = &l[pos + 2..];
            }
            if l == before {
                break;
            }
        }
        if l.chars().all(|c| matches!(c, '-' | '*' | '_' | '=')) {
            continue;
        }
        out.push_str(&inline_to_plain(l));
        out.push(' ');
    }
    collapse_ws(&out)
}

fn inline_to_plain(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match c {
            // HTML tags: drop.
            '<' => {
                if let Some(end) = chars[i..].iter().position(|&ch| ch == '>') {
                    let tag: String = chars[i + 1..i + end].iter().collect();
                    let looks_like_tag = tag
                        .trim_start_matches('/')
                        .chars()
                        .next()
                        .is_some_and(|ch| ch.is_ascii_alphabetic());
                    if looks_like_tag {
                        i += end + 1;
                        out.push(' ');
                        continue;
                    }
                }
                out.push(c);
            }
            // Images ![alt](url) → alt; links [text](url) → text.
            '!' if chars.get(i + 1) == Some(&'[') => {
                i += 1;
                continue;
            }
            '[' => {
                if let Some(close) = chars[i..].iter().position(|&ch| ch == ']') {
                    let text: String = chars[i + 1..i + close].iter().collect();
                    let after = i + close + 1;
                    if chars.get(after) == Some(&'(')
                        && let Some(paren) = chars[after..].iter().position(|&ch| ch == ')')
                    {
                        out.push_str(&inline_to_plain(&text));
                        i = after + paren + 1;
                        continue;
                    }
                    out.push_str(&inline_to_plain(&text));
                    i = after;
                    continue;
                }
                out.push(c);
            }
            '*' | '_' | '`' | '~' => {
                // Emphasis/code markers. Keep underscores inside words (snake_case).
                if c == '_' {
                    let prev_alnum = i > 0 && chars[i - 1].is_alphanumeric();
                    let next_alnum = chars.get(i + 1).is_some_and(|ch| ch.is_alphanumeric());
                    if prev_alnum && next_alnum {
                        out.push(c);
                    }
                }
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cuts `s` to at most `max` characters (counting the trailing "…") at a word boundary.
pub fn truncate_words(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    let limit = max.saturating_sub(1);
    let cut: String = s.chars().take(limit).collect();
    // Prefer the last space inside the cut, unless that would drop most of it.
    let trimmed = match cut.rfind(char::is_whitespace) {
        Some(pos) if pos >= limit / 2 => cut[..pos].trim_end().to_string(),
        _ => cut.trim_end().to_string(),
    };
    let trimmed = trimmed
        .trim_end_matches([',', ';', ':', '.', '-'])
        .to_string();
    format!("{trimmed}…")
}

/// Plain-text preview of a description: Markdown stripped, at most `max` characters.
pub fn preview(md: Option<&str>, max: usize) -> String {
    md.map_or_default(|m| truncate_words(&markdown_to_plain(m), max))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markdown() {
        let md = "## Title\n\n**Bold** and _it_ with `code` and [link](https://x.y).\n\n- item one\n1. first\n```\nfenced\n```\n<details><summary>More</summary>\nhidden text\n</details>\n> quote\n---\nsnake_case_name";
        assert_eq!(
            markdown_to_plain(md),
            "Title Bold and it with code and link. item one first More hidden text quote snake_case_name"
        );
    }

    #[test]
    fn truncates_at_word_boundary() {
        assert_eq!(truncate_words("short", 10), "short");
        let t = truncate_words("one two three four five", 12);
        assert_eq!(t, "one two…");
        assert!(t.chars().count() <= 12);
        let long = "word ".repeat(200);
        let p = truncate_words(&long, 300);
        assert!(p.chars().count() <= 300);
        assert!(p.ends_with('…'));
        assert!(!p.contains("wor…"));
    }
}
