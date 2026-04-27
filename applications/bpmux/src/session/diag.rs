//! Diagnostic logging sink for background task errors.
//!
//! Code that runs concurrently with the TUI must never write to stdout or
//! stderr — ratatui owns stdout via the alt-screen, and stderr still flows
//! to the same tty, so any write corrupts the rendered frame. That's the
//! bug class behind the "unusable" stderr-bleed incident in InstrumentedStorage.
//!
//! `log_diag(line)` appends a timestamped line to a diagnostic file. The
//! path is `BPMUX_LOG` (env override) or `~/.bpmux/diag.log` (default).
//! Pre-#508 the default was a silent no-op when `BPMUX_LOG` was unset,
//! which made today's 2026-04-27 substrate-failure-class invisible in
//! production: observability that requires opt-in env-var fails-open
//! when no one remembers to set it. The default-to-file path keeps
//! diagnostics on by default while preserving the env override for
//! redirection.

use std::io::Write;

/// Resolve the diagnostic log path: `BPMUX_LOG` env override, else
/// `~/.bpmux/diag.log` default. Returns `None` when home dir is
/// undiscoverable AND `BPMUX_LOG` is unset/empty.
fn diag_path() -> Option<std::path::PathBuf> {
    if let Ok(path) = std::env::var("BPMUX_LOG")
        && !path.is_empty()
    {
        return Some(std::path::PathBuf::from(path));
    }
    let home = std::env::var_os("HOME")?;
    let mut path = std::path::PathBuf::from(home);
    path.push(".bpmux");
    path.push("diag.log");
    Some(path)
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
