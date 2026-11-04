//! The GitLab token lives only in the macOS Keychain (constitution II, research R10).

use crate::error::{AppError, AppResult};

pub const SERVICE: &str = "com.duka88.gitlab-tracker";
pub const ACCOUNT: &str = "gitlab-token:gitlab.com";

fn entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(SERVICE, ACCOUNT).map_err(|e| AppError::Keychain(e.to_string()))
}

/// The stored token, or `None` when there isn't one.
pub fn get_token() -> AppResult<Option<String>> {
    match entry()?.get_password() {
        Ok(t) if !t.trim().is_empty() => Ok(Some(t)),
        Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}

pub fn set_token(token: &str) -> AppResult<()> {
    entry()?
        .set_password(token)
        .map_err(|e| AppError::Keychain(e.to_string()))
}

pub fn delete_token() -> AppResult<()> {
    match entry()?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(AppError::Keychain(e.to_string())),
    }
}
