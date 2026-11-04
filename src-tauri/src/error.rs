//! Error type shared by the GitLab client, the poller and the IPC commands.
//! Serialized to the UI as `{ kind, message }` (contracts/tauri-ipc.md).
//! Messages never contain the token (constitution II).

use serde::{Deserialize, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppError {
    #[error("No GitLab token — add one in Settings")]
    NoToken,
    #[error("Token rejected — update it in Settings")]
    TokenRejected,
    #[error("No permission to read this list")]
    NoPermission,
    /// Carries the full message, e.g. "Group not found".
    #[error("{0}")]
    NotFound(String),
    #[error("GitLab rate limit — retrying next check")]
    RateLimited,
    #[error("Can't reach GitLab — showing last data")]
    Unreachable,
    /// First GraphQL error message, or an unexpected HTTP status.
    #[error("{0}")]
    Query(String),
    #[error("{0}")]
    Invalid(String),
    #[error("Keychain error: {0}")]
    Keychain(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    pub fn kind(&self) -> &'static str {
        match self {
            AppError::NoToken => "noToken",
            AppError::TokenRejected => "tokenRejected",
            AppError::NoPermission => "noPermission",
            AppError::NotFound(_) => "notFound",
            AppError::RateLimited => "rateLimited",
            AppError::Unreachable => "unreachable",
            AppError::Query(_) => "query",
            AppError::Invalid(_) => "invalid",
            AppError::Keychain(_) => "keychain",
            AppError::Internal(_) => "internal",
        }
    }

    /// Errors shown once for the whole app rather than per tab.
    pub fn is_global(&self) -> bool {
        matches!(
            self,
            AppError::NoToken | AppError::TokenRejected | AppError::Unreachable
        )
    }

    /// Rate limiting is a warning (orange), everything else is an error (red).
    pub fn is_warning(&self) -> bool {
        matches!(self, AppError::RateLimited)
    }

    pub fn to_issue(&self) -> Issue {
        Issue {
            kind: self.kind().to_string(),
            message: self.to_string(),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.to_issue().serialize(s)
    }
}

/// `{ kind, message }` for a red/orange bar (FR-031). Also the wire form of [`AppError`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    pub kind: String,
    pub message: String,
}

impl Issue {
    pub fn warning(kind: &str, message: impl Into<String>) -> Self {
        Issue {
            kind: kind.to_string(),
            message: message.into(),
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
