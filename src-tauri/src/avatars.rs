//! `avatar://localhost/<percent-encoded URL>`: avatars fetched by Rust and cached, so the
//! webview never contacts GitLab (research R16, constitution I). Only gitlab.com is fetched.

use std::fmt::Write as _;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use lru::LruCache;
use sha2::{Digest, Sha256};
use tauri::http::{Response, StatusCode, header};

use crate::config::{decode_avatar_path, is_gitlab_url};

const MEMORY_ENTRIES: usize = 300;
const DISK_MAX_AGE: Duration = Duration::from_hours(168);
const MAX_BYTES: usize = 2 * 1024 * 1024;

pub struct AvatarCache {
    memory: Mutex<LruCache<String, (Vec<u8>, &'static str)>>,
    dir: PathBuf,
    http: reqwest::Client,
}

impl AvatarCache {
    pub fn new(dir: PathBuf, http: reqwest::Client) -> Self {
        AvatarCache {
            memory: Mutex::new(LruCache::new(
                NonZeroUsize::new(MEMORY_ENTRIES).expect("non-zero"),
            )),
            dir,
            http,
        }
    }

    /// Serves one request path (`/<percent-encoded URL>`).
    pub async fn serve(&self, path: &str) -> Response<Vec<u8>> {
        let Some(url) = decode_avatar_path(path).filter(|u| is_gitlab_url(u)) else {
            return not_found();
        };
        if let Some((bytes, ct)) = self
            .memory
            .lock()
            .ok()
            .and_then(|mut m| m.get(&url).cloned())
        {
            return ok(bytes, ct);
        }
        let file = self.dir.join(hex_sha256(&url));
        if let Some(bytes) = read_fresh(&file) {
            let ct = sniff(&bytes);
            self.remember(&url, &bytes, ct);
            return ok(bytes, ct);
        }
        match self.fetch(&url).await {
            Some(bytes) => {
                let ct = sniff(&bytes);
                let _ = std::fs::create_dir_all(&self.dir);
                let _ = std::fs::write(&file, &bytes);
                self.remember(&url, &bytes, ct);
                ok(bytes, ct)
            }
            None => not_found(),
        }
    }

    fn remember(&self, url: &str, bytes: &[u8], ct: &'static str) {
        if let Ok(mut m) = self.memory.lock() {
            m.put(url.to_string(), (bytes.to_vec(), ct));
        }
    }

    /// No token is sent: avatar uploads on gitlab.com are public.
    async fn fetch(&self, url: &str) -> Option<Vec<u8>> {
        let resp = self.http.get(url).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        // A redirect could leave gitlab.com; refuse the result then.
        if !is_gitlab_url(resp.url().as_str()) {
            return None;
        }
        let bytes = resp.bytes().await.ok()?;
        if bytes.len() > MAX_BYTES || sniff(&bytes) == "application/octet-stream" {
            return None;
        }
        Some(bytes.to_vec())
    }
}

fn read_fresh(file: &std::path::Path) -> Option<Vec<u8>> {
    let meta = std::fs::metadata(file).ok()?;
    let age = SystemTime::now()
        .duration_since(meta.modified().ok()?)
        .unwrap_or_default();
    if age > DISK_MAX_AGE {
        let _ = std::fs::remove_file(file);
        return None;
    }
    std::fs::read(file).ok()
}

fn hex_sha256(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Content type from magic bytes; non-images are rejected.
pub fn sniff(b: &[u8]) -> &'static str {
    if b.starts_with(&[0x89, b'P', b'N', b'G']) {
        "image/png"
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if b.starts_with(b"GIF8") {
        "image/gif"
    } else if b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "application/octet-stream"
    }
}

fn ok(bytes: Vec<u8>, ct: &'static str) -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, ct)
        .header(header::CACHE_CONTROL, "max-age=86400")
        .body(bytes)
        .unwrap_or_else(|_| not_found())
}

fn not_found() -> Response<Vec<u8>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Vec::new())
        .expect("static response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_images() {
        assert_eq!(sniff(&[0x89, b'P', b'N', b'G', 0, 0]), "image/png");
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(sniff(b"<html>"), "application/octet-stream");
    }

    #[tokio::test]
    async fn refuses_other_hosts_without_fetching() {
        let dir = tempfile::tempdir().unwrap();
        let cache = AvatarCache::new(dir.path().to_path_buf(), reqwest::Client::new());
        let gravatar =
            crate::config::avatar_proxy_url("https://secure.gravatar.com/avatar/x").unwrap();
        let path = gravatar.trim_start_matches("avatar://localhost");
        assert_eq!(cache.serve(path).await.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            cache.serve("/not%20a%20url").await.status(),
            StatusCode::NOT_FOUND
        );
    }
}
