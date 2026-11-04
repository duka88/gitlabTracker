//! Host configuration. v1 supports gitlab.com only (research R17).

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};

/// The only GitLab host the app talks to.
pub const GITLAB_HOST: &str = "https://gitlab.com";

/// Host name part of [`GITLAB_HOST`].
pub const GITLAB_HOSTNAME: &str = "gitlab.com";

/// Prefix of avatar URLs served by the `avatar://` URI scheme (research R16).
pub const AVATAR_SCHEME_PREFIX: &str = "avatar://localhost/";

/// Encode everything except unreserved characters, so the whole URL fits in one path segment.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// Turns a GitLab path (`/uploads/...`) into an absolute URL. Absolute URLs pass through.
pub fn absolute_url(path_or_url: &str) -> String {
    if path_or_url.starts_with("http://") || path_or_url.starts_with("https://") {
        path_or_url.to_string()
    } else if path_or_url.starts_with('/') {
        format!("{GITLAB_HOST}{path_or_url}")
    } else {
        format!("{GITLAB_HOST}/{path_or_url}")
    }
}

/// Wraps a GitLab avatar URL (relative or absolute) in the `avatar://` proxy scheme.
/// Returns `None` for an empty URL.
pub fn avatar_proxy_url(url: &str) -> Option<String> {
    let url = url.trim();
    if url.is_empty() {
        return None;
    }
    let abs = absolute_url(url);
    Some(format!(
        "{AVATAR_SCHEME_PREFIX}{}",
        utf8_percent_encode(&abs, PATH_SEGMENT)
    ))
}

/// Reverses [`avatar_proxy_url`] on the path part of an `avatar://` request.
pub fn decode_avatar_path(path: &str) -> Option<String> {
    let trimmed = path.trim_start_matches('/');
    percent_decode_str(trimmed)
        .decode_utf8()
        .ok()
        .map(std::borrow::Cow::into_owned)
}

/// True for `https://gitlab.com` and any path under it.
pub fn is_gitlab_url(url: &str) -> bool {
    match url::Url::parse(url) {
        Ok(u) => {
            u.scheme() == "https" && u.host_str() == Some(GITLAB_HOSTNAME) && u.port().is_none()
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_and_proxy() {
        assert_eq!(
            absolute_url("/uploads/a.png"),
            "https://gitlab.com/uploads/a.png"
        );
        assert_eq!(absolute_url("https://x.y/a"), "https://x.y/a");
        let p = avatar_proxy_url("/uploads/-/system/user/avatar/1/avatar.png?v=2").unwrap();
        assert!(p.starts_with("avatar://localhost/https%3A%2F%2Fgitlab.com%2Fuploads"));
        let decoded = decode_avatar_path(p.trim_start_matches("avatar://localhost")).unwrap();
        assert_eq!(
            decoded,
            "https://gitlab.com/uploads/-/system/user/avatar/1/avatar.png?v=2"
        );
        assert_eq!(avatar_proxy_url(""), None);
    }

    #[test]
    fn gitlab_url_check() {
        assert!(is_gitlab_url("https://gitlab.com/a/b/-/merge_requests/1"));
        assert!(!is_gitlab_url("http://gitlab.com/a"));
        assert!(!is_gitlab_url("https://gitlab.com.evil.io/a"));
        assert!(!is_gitlab_url("https://evil.io/?https://gitlab.com"));
    }
}
