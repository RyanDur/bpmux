//! GitHub API error taxonomy shared across layers.
//!
//! The bridge's StoryTracker impl and the terminal's board poller
//! both issue GitHub API calls and need to surface the same error
//! classes (auth, rate limit, network, other) to users. Keeping the
//! shared vocabulary here — below both `bridge` and `terminal` — lets
//! each depend only on this module instead of importing across
//! sibling layers.

use octocrab::Error;

/// Distinguishable error classes for specific UX surfaces. Each
/// variant maps to a human-readable message via `user_message`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardFetchError {
    /// GH_TOKEN env var missing or rejected by GitHub (401/403).
    AuthInvalid,
    /// GitHub rate limit hit. `retry_after_secs` is optional — some
    /// error responses don't include a retry-after header.
    RateLimited { retry_after_secs: Option<u64> },
    /// Network unreachable, DNS failure, timeout.
    NetworkUnreachable,
    /// Something else — unparseable response, 5xx, etc.
    Other(String),
}

impl BoardFetchError {
    /// Human-readable error surface for the board view. Kept as a
    /// short line — the board has a few rows of real estate, not
    /// paragraphs.
    pub fn user_message(&self) -> String {
        match self {
            Self::AuthInvalid => {
                "GITHUB_TOKEN missing or invalid — set it and restart bpmux".to_string()
            }
            Self::RateLimited {
                retry_after_secs: Some(s),
            } => format!("rate limited — retrying in {s}s"),
            Self::RateLimited {
                retry_after_secs: None,
            } => "rate limited — retrying".to_string(),
            Self::NetworkUnreachable => "offline".to_string(),
            Self::Other(e) => format!("fetch failed: {e}"),
        }
    }
}

/// Map octocrab errors into a `BoardFetchError`. Pure function —
/// called from both the board poller and the bridge tracker so
/// error language is identical across code paths.
pub fn classify_error(err: &Error) -> BoardFetchError {
    match err {
        Error::GitHub { source, .. } => {
            let code = source.status_code.as_u16();
            if code == 401 || code == 403 {
                // 403 with a rate-limit note lands here too. Check
                // the message for a rate-limit hint; otherwise treat
                // as auth.
                let msg_lc = source.message.to_lowercase();
                if msg_lc.contains("rate limit") || msg_lc.contains("api rate") {
                    BoardFetchError::RateLimited {
                        retry_after_secs: None,
                    }
                } else {
                    BoardFetchError::AuthInvalid
                }
            } else if code == 429 {
                BoardFetchError::RateLimited {
                    retry_after_secs: None,
                }
            } else {
                BoardFetchError::Other(format!("{code}: {}", source.message))
            }
        }
        Error::Http { .. } | Error::Hyper { .. } => BoardFetchError::NetworkUnreachable,
        other => BoardFetchError::Other(other.to_string()),
    }
}
