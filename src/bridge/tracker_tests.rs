//! Tests for the tracker's fail-loud observability (#236).
//!
//! Live `gh` and `octocrab` aren't available in CI, so we test the
//! extracted helper `log_labels_fetch_failure` directly — it owns
//! the fail-soft + log_diag semantics both tracker impls delegate to.
//! Success-path coverage flows through `addressee_for_pass_desk_check`
//! and the existing `FakeTracker` (empty-labels → default-to-ryan).

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
