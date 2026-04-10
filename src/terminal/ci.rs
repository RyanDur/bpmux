//! Background CI-status polling + indicator state (#151).
//!
//! Phase 1 from `docs/ci-status-ux-spec.md`:
//! - Shell out to `gh run list -L 1 --branch main --json ...` every 60s
//! - Map the result to a three-state enum (Green / Red / Running)
//! - Emit updates over an mpsc channel for the event loop to consume
//! - All polling errors routed through `session::diag::log_diag` so a
//!   network blip or missing `gh` binary never corrupts the TUI frame
//!
//! Explicit non-goals (per the UX spec): no drill-in, no notifications, no
//! broadcast messages, no cross-workspace propagation, no attribution text.
//! The indicator state is *passive* — the only side effect is changing a
//! glyph in the status bar.

use std::time::Duration;

use serde::Deserialize;
use tokio::sync::mpsc;

use crate::session::diag::log_diag;

/// Three-state CI indicator shown in the status bar. `Unknown` is the
/// internal startup state before the first poll — it is NOT rendered as
/// a fourth glyph per Phase 1 scope. `render_indicator` returns `None`
/// for `Unknown`, keeping the indicator hidden until we have real data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CiState {
    Unknown,
    Green,
    Red,
    Running,
}

const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Spawn a background task that polls CI every 60 seconds and sends state
/// updates to `tx`. The task exits cleanly when the receiver is dropped.
///
/// Only sends an update when the state CHANGES. The startup transition
/// from `Unknown` → first known state always fires.
pub(super) fn spawn_poller(tx: mpsc::Sender<CiState>) {
    tokio::spawn(async move {
        let mut last = CiState::Unknown;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        // First tick fires immediately, so we get a state update at startup
        // rather than waiting a full minute for the first paint.
        loop {
            interval.tick().await;
            let next = match poll_once().await {
                Ok(state) => state,
                Err(e) => {
                    log_diag(&format!("ci: poll error: {e}"));
                    // Keep the last known state on error — spec says Phase 1
                    // does not render an error glyph. The diag log captures
                    // the failure for later inspection.
                    continue;
                }
            };
            if next != last {
                last = next;
                if tx.send(next).await.is_err() {
                    break;
                }
            }
        }
    });
}

#[derive(Debug, Deserialize)]
struct RunEntry {
    status: String,
    conclusion: Option<String>,
}

async fn poll_once() -> Result<CiState, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("gh")
        .args([
            "run",
            "list",
            "-L",
            "1",
            "--branch",
            "main",
            "--json",
            "status,conclusion",
        ])
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh exited non-zero: {}", stderr.trim()).into());
    }
    let runs: Vec<RunEntry> = serde_json::from_slice(&output.stdout)?;
    Ok(classify_runs(&runs))
}

fn classify_runs(runs: &[RunEntry]) -> CiState {
    let Some(latest) = runs.first() else {
        return CiState::Unknown;
    };
    // "Running wins over stale success or stale failure — the user cares
    // that a fresh answer is coming." (spec)
    if latest.status == "in_progress" || latest.status == "queued" {
        return CiState::Running;
    }
    match latest.conclusion.as_deref() {
        Some("success") => CiState::Green,
        // All non-success completed states render red — the spec
        // explicitly does not distinguish failed / cancelled / timed_out
        // at the indicator level. Details belong in `gh run view`.
        Some(_) => CiState::Red,
        None => CiState::Unknown,
    }
}

/// Return the (glyph, color-hint) pair for the status bar, or None if the
/// indicator should not be shown (e.g. Unknown / startup).
pub(super) fn render_indicator(state: CiState) -> Option<(&'static str, CiGlyphColor)> {
    match state {
        CiState::Green => Some(("CI \u{2713}", CiGlyphColor::Green)), // ✓
        CiState::Red => Some(("CI \u{2717}", CiGlyphColor::Red)),     // ✗
        CiState::Running => Some(("CI \u{25CB}", CiGlyphColor::Yellow)), // ◌
        CiState::Unknown => None,
    }
}

/// Color-hint returned by `render_indicator`. The caller maps this to its
/// own theme-specific color; keeping it as an enum avoids pulling the
/// theme module into `ci.rs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CiGlyphColor {
    Green,
    Red,
    Yellow,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(status: &str, conclusion: Option<&str>) -> RunEntry {
        RunEntry {
            status: status.to_string(),
            conclusion: conclusion.map(String::from),
        }
    }

    #[test]
    fn empty_runs_is_unknown() {
        assert_eq!(classify_runs(&[]), CiState::Unknown);
    }

    #[test]
    fn in_progress_is_running() {
        assert_eq!(classify_runs(&[run("in_progress", None)]), CiState::Running);
    }

    #[test]
    fn queued_is_running() {
        assert_eq!(classify_runs(&[run("queued", None)]), CiState::Running);
    }

    #[test]
    fn completed_success_is_green() {
        assert_eq!(
            classify_runs(&[run("completed", Some("success"))]),
            CiState::Green
        );
    }

    #[test]
    fn completed_failure_is_red() {
        assert_eq!(
            classify_runs(&[run("completed", Some("failure"))]),
            CiState::Red
        );
    }

    #[test]
    fn completed_cancelled_is_red() {
        assert_eq!(
            classify_runs(&[run("completed", Some("cancelled"))]),
            CiState::Red
        );
    }

    #[test]
    fn completed_timed_out_is_red() {
        // Per spec: all non-success conclusions render red.
        assert_eq!(
            classify_runs(&[run("completed", Some("timed_out"))]),
            CiState::Red
        );
    }

    #[test]
    fn completed_action_required_is_red() {
        assert_eq!(
            classify_runs(&[run("completed", Some("action_required"))]),
            CiState::Red
        );
    }

    #[test]
    fn completed_without_conclusion_is_unknown() {
        // Degenerate case: gh shouldn't return this, but defend anyway.
        assert_eq!(classify_runs(&[run("completed", None)]), CiState::Unknown);
    }

    #[test]
    fn running_wins_over_stale_red_or_green() {
        // When gh gives us only one run, and that run is in_progress, we
        // render Running even if a previous run was red. The spec
        // explicitly wants "running wins."
        assert_eq!(classify_runs(&[run("in_progress", None)]), CiState::Running);
    }

    #[test]
    fn unknown_has_no_indicator() {
        assert!(render_indicator(CiState::Unknown).is_none());
    }

    #[test]
    fn green_renders_check_glyph() {
        let (label, color) = render_indicator(CiState::Green).unwrap();
        assert_eq!(label, "CI \u{2713}");
        assert_eq!(color, CiGlyphColor::Green);
    }

    #[test]
    fn red_renders_cross_glyph() {
        let (label, color) = render_indicator(CiState::Red).unwrap();
        assert_eq!(label, "CI \u{2717}");
        assert_eq!(color, CiGlyphColor::Red);
    }

    #[test]
    fn running_renders_circle_glyph() {
        let (label, color) = render_indicator(CiState::Running).unwrap();
        assert_eq!(label, "CI \u{25CB}");
        assert_eq!(color, CiGlyphColor::Yellow);
    }

    #[test]
    fn glyphs_are_distinct_per_state() {
        // Accessibility: color alone is insufficient, the spec requires
        // glyph differences as redundant meaning.
        let green = render_indicator(CiState::Green).unwrap().0;
        let red = render_indicator(CiState::Red).unwrap().0;
        let running = render_indicator(CiState::Running).unwrap().0;
        assert_ne!(green, red);
        assert_ne!(green, running);
        assert_ne!(red, running);
    }
}
