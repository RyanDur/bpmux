//! Diagnostic logging sink for background task errors.
//!
//! Code that runs concurrently with the TUI must never write to stdout or
//! stderr — ratatui owns stdout via the alt-screen, and stderr still flows
//! to the same tty, so any write corrupts the rendered frame. That's the
//! bug class behind the "unusable" stderr-bleed incident in InstrumentedStorage.
//!
//! `log_diag(line)` appends a timestamped line to a diagnostic file. Path
//! resolution (post-#512) walks three precedence levels:
//!
//! 1. `BPMUX_LOG` env override — wins always, in any mode.
//! 2. test-mode → `${TMPDIR}/bpmux-test-diag.log`. Detected via either
//!    `BPMUX_TEST_MODE=1` (Playwright suites + explicit invocations) or
//!    `current_exe()` containing a `deps` path component (every cargo
//!    test binary lives under `target/.../deps/<hash>`; the prod
//!    binary at `target/.../bpmux` does not).
//! 3. Default → `~/.bpmux/diag.log` (production-bridge per #508 slice 1).
//!
//! Closes the test-pollution sub-bug from #508 slice 1's incomplete
//! substrate-fix: cargo runs were writing fixture-noise into the prod
//! diag log, drowning real evidence during diagnosis.

use std::io::Write;

const TEST_DIAG_FILENAME: &str = "bpmux-test-diag.log";

/// Resolve the diagnostic log path. See module docs for precedence.
fn diag_path() -> Option<std::path::PathBuf> {
    let bpmux_log = std::env::var("BPMUX_LOG").ok();
    let bpmux_test_mode = std::env::var("BPMUX_TEST_MODE").ok();
    let exe_is_cargo_test = std::env::current_exe()
        .ok()
        .map(path_has_deps_segment)
        .unwrap_or(false);
    let home = std::env::var_os("HOME").map(std::path::PathBuf::from);
    let tmpdir = std::env::temp_dir();
    resolve_diag_path(
        bpmux_log.as_deref(),
        bpmux_test_mode.as_deref(),
        exe_is_cargo_test,
        home.as_deref(),
        &tmpdir,
    )
}

/// Pure routing function — separated from env/fs reads so unit tests can
/// drive every precedence branch without touching the host environment.
pub(crate) fn resolve_diag_path(
    bpmux_log: Option<&str>,
    bpmux_test_mode: Option<&str>,
    exe_is_cargo_test: bool,
    home: Option<&std::path::Path>,
    tmpdir: &std::path::Path,
) -> Option<std::path::PathBuf> {
    if let Some(p) = bpmux_log.filter(|s| !s.is_empty()) {
        return Some(std::path::PathBuf::from(p));
    }
    let in_test_mode = bpmux_test_mode.is_some_and(|s| !s.is_empty()) || exe_is_cargo_test;
    if in_test_mode {
        return Some(tmpdir.join(TEST_DIAG_FILENAME));
    }
    home.map(|h| h.join(".bpmux").join("diag.log"))
}

/// Cargo builds every test binary under `target/{debug,release}/deps/<hash>`;
/// the production `bpmux` binary lives one level up. A `deps` path component
/// reliably distinguishes the two without false-positives on prod runs.
pub(crate) fn path_has_deps_segment(exe: std::path::PathBuf) -> bool {
    exe.components().any(|c| c.as_os_str() == "deps")
}

/// Append a timestamped diagnostic line to the resolved path.
/// Swallows all IO errors — the caller's failure mode is "some task
/// logged something" and we cannot escalate further without potentially
/// writing to a tty.
pub fn log_diag(line: &str) {
    let Some(path) = diag_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    else {
        return;
    };
    let ts = chrono::Utc::now().to_rfc3339();
    let _ = writeln!(file, "{ts} {line}");
}
