pub mod client;
pub mod rate_limit;

pub use client::{ensure_crypto_provider_installed, octocrab_client};
pub use rate_limit::{
    BackoffDecision, BackoffSignal, CLOCK_SKEW_GRACE_SECS, ObservedWakes, RateLimitObserver,
    classify_and_decide, decide_backoff, is_rate_limit_error, signal_label,
};

use octocrab::Error;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoardFetchError {
    AuthInvalid,
    RateLimited { retry_after_secs: Option<u64> },
    NetworkUnreachable,
    Other(String),
}

impl BoardFetchError {
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

pub fn classify_error(err: &Error) -> BoardFetchError {
    match err {
        Error::GitHub { source, .. } => {
            let code = source.status_code.as_u16();
            if code == 401 || code == 403 {
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
