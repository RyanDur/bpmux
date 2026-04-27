//! Tests for the tracker's fail-loud observability (#236).
//!
//! Live `gh` and `octocrab` aren't available in CI, so we test the
//! extracted helper `log_labels_fetch_failure` directly — it owns
//! the fail-soft + log_diag semantics tracker callers delegate to.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::tracker::log_labels_fetch_failure;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

/// Serialize access to the BPMUX_LOG env var across these tests.
/// cargo test runs tests in parallel by default; without this mutex
/// a concurrent `remove_var` races with a `set_var` + read pattern
/// and the "writes" test flaps.
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
    /// Set `BPMUX_LOG` to `path` for the life of the guard, holding
    /// a process-wide mutex while the var is mutated. Restores the
    /// previous value (or removes it) on drop.
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
    // Mirrors session::diag::log_diag's no-op branch: when BPMUX_LOG
    // isn't set, the helper must not write anywhere and must still
    // return an empty Vec. This is the pre-#236 silent behavior
    // preserved for users who haven't opted into diagnostics.
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

// #298 tracker-selection tests — exercise env-var permutations via
// closure-injected env reader (no process env mutation, no env_lock
// serialization needed). Catches the silent Octocrab→Gh fallback
// class by asserting the exact FallbackReason variant for each
// degenerate env state.

mod tracker_selection {
    use super::super::tracker::{FallbackReason, TrackerSelection, select_from_env};
    use std::collections::HashMap;

    /// Build a reader closure over an owned env map. Tests construct
    /// a fresh map per case so no cross-test state leaks. Closure
    /// matches `Fn(&str) -> Option<String>` so `select_from_env`
    /// accepts it directly.
    fn reader(env: HashMap<&str, &str>) -> impl Fn(&str) -> Option<String> {
        let owned: HashMap<String, String> = env
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key: &str| owned.get(key).cloned()
    }

    #[test]
    fn returns_octocrab_when_repo_and_gh_token_set() {
        let r = reader(HashMap::from([
            ("BPMUX_REPO", "ryan/bpmux"),
            ("GH_TOKEN", "t"),
        ]));
        assert_eq!(
            select_from_env(r),
            TrackerSelection::Octocrab {
                owner: "ryan".to_string(),
                repo: "bpmux".to_string(),
                token: "t".to_string(),
            }
        );
    }

    #[test]
    fn returns_octocrab_when_repo_and_github_token_set() {
        // GITHUB_TOKEN is the documented Actions-runtime variant;
        // bridge treats it as an alias for GH_TOKEN so repo owners
        // with either convention work without re-configuring.
        let r = reader(HashMap::from([
            ("BPMUX_REPO", "ryan/bpmux"),
            ("GITHUB_TOKEN", "gh-actions-token"),
        ]));
        assert!(matches!(
            select_from_env(r),
            TrackerSelection::Octocrab { .. }
        ));
    }

    #[test]
    fn gh_token_wins_when_both_tokens_set() {
        // Preference order: GH_TOKEN first, GITHUB_TOKEN fallback.
        // Matches gh CLI's precedence; avoids surprise when someone
        // sets both expecting the user-convention one to win.
        let r = reader(HashMap::from([
            ("BPMUX_REPO", "o/r"),
            ("GH_TOKEN", "primary"),
            ("GITHUB_TOKEN", "secondary"),
        ]));
        match select_from_env(r) {
            TrackerSelection::Octocrab { token, .. } => assert_eq!(token, "primary"),
            other => panic!("expected Octocrab, got {other:?}"),
        }
    }

    #[test]
    fn fallback_no_repo_when_bpmux_repo_missing() {
        // Most common failure mode: user sets GH_TOKEN thinking it's
        // enough but hasn't set BPMUX_REPO. Caller logs this reason;
        // user gets a diag hit telling them what's missing.
        let r = reader(HashMap::from([("GH_TOKEN", "t")]));
        assert_eq!(
            select_from_env(r),
            TrackerSelection::Fallback(FallbackReason::NoRepo)
        );
    }

    #[test]
    fn fallback_malformed_repo_when_no_slash() {
        // BPMUX_REPO must be "owner/name"; a bare token (no slash)
        // is malformed. Distinguished from NoRepo so the diag line
        // points the user at the format issue, not a missing var.
        let r = reader(HashMap::from([
            ("BPMUX_REPO", "justname"),
            ("GH_TOKEN", "t"),
        ]));
        assert_eq!(
            select_from_env(r),
            TrackerSelection::Fallback(FallbackReason::MalformedRepo)
        );
    }

    #[test]
    fn fallback_malformed_repo_when_owner_empty() {
        let r = reader(HashMap::from([("BPMUX_REPO", "/bpmux"), ("GH_TOKEN", "t")]));
        assert_eq!(
            select_from_env(r),
            TrackerSelection::Fallback(FallbackReason::MalformedRepo)
        );
    }

    #[test]
    fn fallback_malformed_repo_when_name_empty() {
        let r = reader(HashMap::from([("BPMUX_REPO", "ryan/"), ("GH_TOKEN", "t")]));
        assert_eq!(
            select_from_env(r),
            TrackerSelection::Fallback(FallbackReason::MalformedRepo)
        );
    }

    #[test]
    fn fallback_no_token_when_neither_token_set() {
        let r = reader(HashMap::from([("BPMUX_REPO", "o/r")]));
        assert_eq!(
            select_from_env(r),
            TrackerSelection::Fallback(FallbackReason::NoToken)
        );
    }

    #[test]
    fn fallback_when_env_is_completely_empty() {
        // NoRepo wins (checked first in the resolution order).
        // Deterministic ordering matters because a diag reader
        // that sees NoRepo knows it's not a token issue; if
        // ordering flipped they'd chase the wrong fix.
        let r = reader(HashMap::new());
        assert_eq!(
            select_from_env(r),
            TrackerSelection::Fallback(FallbackReason::NoRepo)
        );
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
