//! Diagnostic logging sink for background task errors.
//!
//! Code that runs concurrently with the TUI must never write to stdout or
//! stderr — ratatui owns stdout via the alt-screen, and stderr still flows
//! to the same tty, so any write corrupts the rendered frame. That's the
//! bug class behind the "unusable" stderr-bleed incident in InstrumentedStorage.
//!
//! This module provides `log_diag(line)` which appends a timestamped line to
//! the file named by the `BPMUX_LOG` environment variable. When the env var
//! is unset, logging is a no-op — important because forcing a log file as a
//! side effect of startup would surprise users. Opt-in observability.

use std::io::Write;

/// Append a diagnostic line to the file named by `BPMUX_LOG`, if set.
/// No-op otherwise. Swallows all IO errors because the caller's failure
/// mode is "some task logged something" and we cannot escalate further
/// without potentially writing to a tty.
pub fn log_diag(line: &str) {
    let Ok(path) = std::env::var("BPMUX_LOG") else {
        return;
    };
    if path.is_empty() {
        return;
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
