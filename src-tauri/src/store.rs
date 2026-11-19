//! Atomic JSON persistence in the app data dir (research R12).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;

pub const SETTINGS_FILE: &str = "settings.json";
pub const STATE_FILE: &str = "state.json";
pub const NOTIFICATIONS_FILE: &str = "notifications.json";

/// Result of a load: the value plus a warning when the file was corrupt.
#[derive(Debug)]
pub struct Loaded<T> {
    pub value: T,
    pub warning: Option<String>,
}

/// Loads `path`, falling back to `T::default()` when missing or corrupt.
/// A corrupt file is kept next to the original as `<name>.corrupt` for inspection.
pub fn load_or_default<T: DeserializeOwned + Default>(path: &Path) -> Loaded<T> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Loaded {
                value: T::default(),
                warning: None,
            };
        }
        Err(e) => {
            return Loaded {
                value: T::default(),
                warning: Some(format!("Couldn't read {}: {e}", file_name(path))),
            };
        }
    };
    match serde_json::from_slice::<T>(&bytes) {
        Ok(value) => Loaded {
            value,
            warning: None,
        },
        Err(e) => {
            let _ = fs::rename(path, path.with_extension("json.corrupt"));
            Loaded {
                value: T::default(),
                warning: Some(format!(
                    "{} was corrupt and was reset ({e})",
                    file_name(path)
                )),
            }
        }
    }
}

/// Writes `value` as JSON to `path` atomically (temp file in the same dir, fsync, rename).
pub fn save<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = tmp_path(path);
    let json = serde_json::to_vec(value).map_err(std::io::Error::other)?;
    {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&json)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map_or_default(std::ffi::OsStr::to_os_string);
    name.push(".tmp");
    path.with_file_name(name)
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}
