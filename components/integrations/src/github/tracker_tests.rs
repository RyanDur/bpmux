#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::tracker::log_labels_fetch_failure;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

struct DiagEnvGuard {
    prev: Option<String>,
    _lock: MutexGuard<'static, ()>,
}

impl DiagEnvGuard {
    fn set(path: &std::path::Path) -> Self {
        let lock = env_lock();
        let prev = std::env::var("BPMUX_LOG").ok();
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("BPMUX_LOG", path);
        }
        Self { prev, _lock: lock }
    }

    fn unset() -> Self {
        let lock = env_lock();
        let prev = std::env::var("BPMUX_LOG").ok();
        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("BPMUX_LOG");
        }
        Self { prev, _lock: lock }
    }
}

impl Drop for DiagEnvGuard {
    fn drop(&mut self) {
        #[allow(unsafe_code)]
        unsafe {
            match &self.prev {
                Some(v) => std::env::set_var("BPMUX_LOG", v),
                None => std::env::remove_var("BPMUX_LOG"),
            }
        }
    }
}

#[test]
fn log_labels_fetch_failure_returns_empty_vec() {
    let tmp = TempDir::new().unwrap();
    let _guard = DiagEnvGuard::set(&tmp.path().join("bpmux.log"));
    let out = log_labels_fetch_failure("octocrab", 203, "http 401");
    assert!(out.is_empty(), "failure path must return empty labels");
}

#[test]
fn log_labels_fetch_failure_writes_to_diag_file() {
    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("bpmux.log");
    let _guard = DiagEnvGuard::set(&log_path);

    log_labels_fetch_failure("octocrab", 203, "http 401");

    let contents = std::fs::read_to_string(&log_path).expect("log file written");
    assert!(
        contents.contains("get_story_labels"),
        "diag line missing context prefix: `{contents}`"
    );
    assert!(
        contents.contains("(octocrab)"),
        "diag line missing backend tag: `{contents}`"
    );
    assert!(
        contents.contains("#203"),
        "diag line missing story number: `{contents}`"
    );
    assert!(
        contents.contains("http 401"),
        "diag line missing underlying error: `{contents}`"
    );
}

#[test]
fn log_labels_fetch_failure_is_silent_when_bpmux_log_unset() {
    let tmp = TempDir::new().unwrap();
    let guard_path = tmp.path().join("bpmux.log");
    let _guard = DiagEnvGuard::unset();
    let out = log_labels_fetch_failure("gh", 42, "boom");
    assert!(out.is_empty());
    assert!(
        !guard_path.exists(),
        "no file should be written when BPMUX_LOG is unset"
    );
}

#[test]
fn log_labels_fetch_failure_distinguishes_backends() {
    let tmp = TempDir::new().unwrap();
    let log_path = tmp.path().join("bpmux.log");
    let _guard = DiagEnvGuard::set(&log_path);

    log_labels_fetch_failure("gh", 1, "err-a");
    log_labels_fetch_failure("octocrab", 2, "err-b");
    log_labels_fetch_failure("gh parse", 3, "err-c");

    let contents = std::fs::read_to_string(&log_path).expect("log written");
    assert!(contents.contains("(gh)") && contents.contains("#1"));
    assert!(contents.contains("(octocrab)") && contents.contains("#2"));
    assert!(contents.contains("(gh parse)") && contents.contains("#3"));
}

mod tracker_construction {
    use super::super::tracker::OctocrabStoryTracker;
    use crate::github::connection::GithubConnection;

    #[tokio::test]
    async fn a_hosted_connection_builds_a_tracker() {
        let connection = GithubConnection::Hosted {
            owner: "owner".to_string(),
            repo: "bpmux".to_string(),
            token: "t".to_string(),
        };
        assert!(OctocrabStoryTracker::from_connection(&connection).is_some());
    }

    #[tokio::test]
    async fn a_base_url_connection_builds_a_tracker_without_credentials() {
        let connection = GithubConnection::WithBaseUrl {
            owner: "acme".to_string(),
            repo: "bpmux".to_string(),
            base_url: "http://127.0.0.1:9012/".to_string(),
        };
        assert!(OctocrabStoryTracker::from_connection(&connection).is_some());
    }

    #[test]
    fn an_unparseable_base_url_yields_no_tracker() {
        let connection = GithubConnection::WithBaseUrl {
            owner: "acme".to_string(),
            repo: "bpmux".to_string(),
            base_url: "not a uri".to_string(),
        };
        assert!(OctocrabStoryTracker::from_connection(&connection).is_none());
    }
}

mod crypto_provider {
    use super::super::tracker::ensure_crypto_provider_installed;

    #[test]
    fn installs_provider_so_default_is_some() {
        ensure_crypto_provider_installed();
        assert!(
            rustls::crypto::CryptoProvider::get_default().is_some(),
            "crypto provider must be registered before any TLS handshake"
        );
    }

    #[test]
    fn idempotent_across_calls() {
        ensure_crypto_provider_installed();
        ensure_crypto_provider_installed();
        ensure_crypto_provider_installed();
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }
}
