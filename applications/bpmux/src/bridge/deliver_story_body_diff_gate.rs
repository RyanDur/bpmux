//! #1378 deliver_story body-diff-vs-impl audit.
//!
//! Sister substrate-fix to #1362 (desk-check ATP-gate) + #1377
//! (close_story state-axis) triangulating the "claims drift from
//! shipped" meta-class surfaced in today's iteration-close-audit
//! retro. Where #1362 fires at desk-check time, this gate fires at
//! deliver_story time: diffs body's ATP path(s) against
//! git-changed-paths since story-pickup. Block when body cites an
//! ATP not in changed paths since pickup.
//!
//! Lenient by design:
//! - Body has no source-ATP → pass (sister to #1362).
//! - Git unavailable / git command fails → log + pass (separate
//!   substrate concern; don't make deliver_story depend on
//!   git-environmental-axis).
//! - Body's ATP not yet implemented but story scope says it's
//!   incoming → caught by #1362 desk-check gate instead.

use chrono::{DateTime, Utc};

use crate::board::domain::{Gate, Story};

pub(super) fn pickup_ts(story: &Story) -> Option<DateTime<Utc>> {
    story
        .gate_comments
        .as_ref()?
        .iter()
        .filter(|gc| matches!(gc.gate, Gate::Pickup))
        .map(|gc| gc.ts)
        .max()
}

pub(super) fn changed_paths_since(
    ts: DateTime<Utc>,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
) -> Result<Vec<String>, String> {
    let since = ts.to_rfc3339();
    let arg = format!("--since={since}");
    let raw = git(&["log", arg.as_str(), "--name-only", "--pretty=format:"])?;
    Ok(raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
}

pub(super) fn verify_body_atp_in_changed_paths(
    body: &str,
    changed_paths: &[String],
) -> Result<(), String> {
    let body_paths = super::atp_substantive_gate::extract_atp_paths(body);
    if body_paths.is_empty() {
        return Ok(());
    }
    let mut missing = Vec::new();
    for p in &body_paths {
        if !changed_paths.iter().any(|cp| cp == p) {
            missing.push(p.clone());
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(format!(
        "#1378 deliver_story body-diff-vs-impl gate blocks: body cites ATP path(s) {missing:?} but they are NOT in git-changed paths since story pickup. Either reshape the body to point at the file(s) you actually changed, or grow scope to include the cited path. Sister to #1306/#1335 + #1285 body-vs-impl-drift class."
    ))
}

/// Top-level gate API. Tolerant: returns Ok if git is unavailable or
/// the story has no pickup-ts to anchor against.
pub(super) fn verify_deliver_drift(
    body: &str,
    story: &Story,
    git: &dyn Fn(&[&str]) -> Result<String, String>,
) -> Result<(), String> {
    let Some(ts) = pickup_ts(story) else {
        return Ok(());
    };
    let changed_paths = match changed_paths_since(ts, git) {
        Ok(p) => p,
        Err(e) => {
            crate::session::diag::log_diag(&format!(
                "[#1378] deliver_story body-diff gate: git-changed-paths failed ({e}); skipping (lenient on git-error)"
            ));
            return Ok(());
        }
    };
    verify_body_atp_in_changed_paths(body, &changed_paths)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
#[path = "deliver_story_body_diff_tests.rs"]
mod tests;
