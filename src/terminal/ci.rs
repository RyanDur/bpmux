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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use tokio::sync::mpsc;

use crate::session::diag::log_diag;
use crate::session::message::{DeepDetail, MultiDepthPayload};
use crate::session::{Message, MessageType, Session};

/// Three-state CI indicator shown in the status bar. `Unknown` is the
/// internal startup state before the first poll — it is NOT rendered as
/// a fourth glyph per Phase 1 scope. `render_indicator` returns `None`
/// for `Unknown`, keeping the indicator hidden until we have real data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum CiState {
    #[default]
    Unknown,
    Green,
    Red,
    Running,
}

/// Richer per-poll CI snapshot — state plus the metadata needed for
/// #106's broadcast message format. Fields beyond `state` are
/// optional because `gh run list` may return partial data (e.g., a
/// run without a display title, or an older gh version without some
/// fields).
#[derive(Debug, Clone, Default)]
pub(super) struct CiEvent {
    pub state: CiState,
    pub sha: Option<String>,
    pub title: Option<String>,
    pub run_id: Option<u64>,
}

const POLL_INTERVAL: Duration = Duration::from_secs(60);

/// Spawn a background task that polls CI every 60 seconds and sends state
/// updates to `tx`. The task exits cleanly when the receiver is dropped.
///
/// Only sends an update when the state CHANGES. The startup transition
/// from `Unknown` → first known state always fires.
///
/// When `broadcast_session` is `Some`, this function also posts an
/// activity-type broadcast message (#106) on meaningful state
/// transitions: failures always surface, recoveries surface only
/// when they follow a prior failure. The transition logic is in
/// `compute_broadcast_text`, which is pure and unit-tested.
pub(super) fn spawn_poller(tx: mpsc::Sender<CiState>, broadcast_session: Option<Arc<Session>>) {
    tokio::spawn(async move {
        let mut last_state = CiState::Unknown;
        let mut last_broadcast: Option<CiState> = None;
        let mut interval = tokio::time::interval(POLL_INTERVAL);
        // First tick fires immediately, so we get a state update at startup
        // rather than waiting a full minute for the first paint.
        loop {
            interval.tick().await;
            let event = match poll_once().await {
                Ok(ev) => ev,
                Err(e) => {
                    log_diag(&format!("ci: poll error: {e}"));
                    // Keep the last known state on error — spec says Phase 1
                    // does not render an error glyph. The diag log captures
                    // the failure for later inspection.
                    continue;
                }
            };
            if event.state != last_state {
                last_state = event.state;
                if tx.send(event.state).await.is_err() {
                    break;
                }
            }
            // Broadcast hook (#106 / #188). Pure transition logic in
            // `compute_broadcast_text`; this branch is just IO. The
            // multi-depth payload (#188) comes from the paired pure
            // helper `build_multi_depth_for_transition`, which
            // populates the per-role default depths and the OnDemand
            // deep-detail pointer at `gh run view <run_id>`.
            if let Some(session) = broadcast_session.as_ref()
                && let Some(text) = compute_broadcast_text(last_broadcast, &event)
            {
                let is_recovery = event.state == CiState::Green;
                let multi_depth = build_multi_depth_for_transition(&event, is_recovery);
                let msg = Message {
                    from: "bpmux".to_string(),
                    text,
                    ts: chrono::Utc::now(),
                    message_type: MessageType::Activity,
                    to: None,
                    addressee: None,
                    explicit_privacy: false,
                    multi_depth: Some(multi_depth),
                    topic: None,
                };
                if let Err(e) = session.broadcast(&msg) {
                    log_diag(&format!("ci: broadcast error: {e}"));
                }
                last_broadcast = Some(event.state);
            }
        }
    });
}

/// Decide whether a state change warrants a broadcast message, and
/// return the formatted text if so. Pure function, no IO — tested
/// exhaustively over the transition table below.
///
/// Rules (per PM shape for #106):
/// - Emit "CI failed" whenever `next.state` is Red AND
///   `last_broadcast` was not Red. Covers unknown→red, green→red,
///   running→red. The first known state being red fires on startup.
/// - Emit "CI recovered" whenever `next.state` is Green AND
///   `last_broadcast` was Red. The "recoveries after failures only"
///   rule — a healthy pipeline never broadcasts green.
/// - Everything else: silent. Running/Unknown are ignored.
pub(super) fn compute_broadcast_text(
    last_broadcast: Option<CiState>,
    next: &CiEvent,
) -> Option<String> {
    match next.state {
        CiState::Red if last_broadcast != Some(CiState::Red) => Some(format_failure_line(next)),
        CiState::Green if last_broadcast == Some(CiState::Red) => Some(format_recovery_line(next)),
        _ => None,
    }
}

/// Build the `MultiDepthPayload` for a CI transition broadcast
/// (#188 / #184 child 5). Pure function, no IO — called from
/// `spawn_poller` alongside `compute_broadcast_text` so the payload
/// and the text stay consistent by construction.
///
/// **Rule table** (designed to match #184's progressive disclosure
/// pattern while keeping the schema's per-role default_depth map
/// honest):
/// - `summary` is the shallowest form — `🔴 CI failed` for a failure,
///   `🟢 CI recovered` for a recovery. This is what renders if no
///   role-specific depth applies.
/// - `role_default`:
///   - `pm` → summary. PM's decisions need the binary state, not
///     stage-level detail. Don't crowd the shaping queue view.
///   - `designer` → summary. Designer acts on "is shipping blocked?",
///     not on stage-level debug.
///   - `dev` → detail form (full text including commit sha, title,
///     and run link). Dev's decisions depend on WHY a build failed
///     — stage info + commit context is load-bearing.
/// - `deep`:
///   - When we have a `run_id` from the gh poll, the deep form is
///     an `OnDemand` pointer at `gh run view <run_id>`. Token
///     economy: only the drilling agent pays the cost of fetching
///     the full log.
///   - When we don't have a run_id (degenerate gh output), the
///     deep form is a `Preloaded` placeholder so the payload is
///     still valid. Rare in practice.
///
/// The per-role defaults assume the renderer consumer (#184 child 3)
/// will look up the viewing role's entry in `role_default` and fall
/// back to `summary` if missing. Roles not in the map (e.g., `ryan`
/// itself) fall through to `summary`, which is correct — Ryan's
/// default CI view should be the binary state, same as pm/designer.
pub(super) fn build_multi_depth_for_transition(
    event: &CiEvent,
    is_recovery: bool,
) -> MultiDepthPayload {
    let summary = if is_recovery {
        "\u{1F7E2} CI recovered".to_string()
    } else {
        "\u{1F534} CI failed".to_string()
    };
    let dev_detail = if is_recovery {
        format_recovery_line(event)
    } else {
        format_failure_line(event)
    };

    let mut role_default: HashMap<String, String> = HashMap::new();
    role_default.insert("pm".to_string(), summary.clone());
    role_default.insert("designer".to_string(), summary.clone());
    role_default.insert("dev".to_string(), dev_detail);

    let deep = match event.run_id {
        Some(id) => DeepDetail::OnDemand {
            command: format!("gh run view {id}"),
            description: "full CI run output including all stages and logs".to_string(),
        },
        None => DeepDetail::Preloaded {
            text: "(no run id available)".to_string(),
        },
    };

    MultiDepthPayload {
        summary,
        role_default,
        deep,
    }
}

fn format_failure_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{1F534} CI failed");
    append_meta(&mut line, event);
    line
}

fn format_recovery_line(event: &CiEvent) -> String {
    let mut line = String::from("\u{1F7E2} CI recovered");
    append_meta(&mut line, event);
    line
}

/// Append the optional metadata tail (short SHA + title + run link)
/// to a message line. Each field is conditional so a partial poll
/// still produces a useful message.
fn append_meta(line: &mut String, event: &CiEvent) {
    if let Some(sha) = event.sha.as_deref() {
        let short: String = sha.chars().take(7).collect();
        line.push_str(" \u{2014} ");
        line.push_str(&short);
        if let Some(title) = event.title.as_deref() {
            line.push_str(" \"");
            // Truncate long titles so the one-line format stays scannable.
            const MAX_TITLE_LEN: usize = 60;
            if title.chars().count() > MAX_TITLE_LEN {
                let truncated: String = title.chars().take(MAX_TITLE_LEN - 1).collect();
                line.push_str(&truncated);
                line.push('\u{2026}');
            } else {
                line.push_str(title);
            }
            line.push('"');
        }
    }
    if let Some(run_id) = event.run_id {
        line.push_str(" \u{2192} gh run view ");
        line.push_str(&run_id.to_string());
    }
}

#[derive(Debug, Deserialize, Default)]
struct RunEntry {
    status: String,
    conclusion: Option<String>,
    #[serde(rename = "headSha", default)]
    head_sha: Option<String>,
    #[serde(rename = "displayTitle", default)]
    display_title: Option<String>,
    #[serde(rename = "databaseId", default)]
    database_id: Option<u64>,
}

async fn poll_once() -> Result<CiEvent, Box<dyn std::error::Error + Send + Sync>> {
    let output = tokio::process::Command::new("gh")
        .args([
            "run",
            "list",
            "-L",
            "1",
            "--branch",
            "main",
            "--json",
            "status,conclusion,headSha,displayTitle,databaseId",
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

fn classify_runs(runs: &[RunEntry]) -> CiEvent {
    let Some(latest) = runs.first() else {
        return CiEvent::default();
    };
    // "Running wins over stale success or stale failure — the user cares
    // that a fresh answer is coming." (spec)
    let state = if latest.status == "in_progress" || latest.status == "queued" {
        CiState::Running
    } else {
        match latest.conclusion.as_deref() {
            Some("success") => CiState::Green,
            // All non-success completed states render red — the spec
            // explicitly does not distinguish failed / cancelled / timed_out
            // at the indicator level. Details belong in `gh run view`.
            Some(_) => CiState::Red,
            None => CiState::Unknown,
        }
    };
    CiEvent {
        state,
        sha: latest.head_sha.clone(),
        title: latest.display_title.clone(),
        run_id: latest.database_id,
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
            head_sha: None,
            display_title: None,
            database_id: None,
        }
    }

    fn run_with_meta(
        status: &str,
        conclusion: Option<&str>,
        sha: &str,
        title: &str,
        id: u64,
    ) -> RunEntry {
        RunEntry {
            status: status.to_string(),
            conclusion: conclusion.map(String::from),
            head_sha: Some(sha.to_string()),
            display_title: Some(title.to_string()),
            database_id: Some(id),
        }
    }

    #[test]
    fn empty_runs_is_unknown() {
        assert_eq!(classify_runs(&[]).state, CiState::Unknown);
    }

    #[test]
    fn in_progress_is_running() {
        assert_eq!(
            classify_runs(&[run("in_progress", None)]).state,
            CiState::Running
        );
    }

    #[test]
    fn queued_is_running() {
        assert_eq!(
            classify_runs(&[run("queued", None)]).state,
            CiState::Running
        );
    }

    #[test]
    fn completed_success_is_green() {
        assert_eq!(
            classify_runs(&[run("completed", Some("success"))]).state,
            CiState::Green
        );
    }

    #[test]
    fn completed_failure_is_red() {
        assert_eq!(
            classify_runs(&[run("completed", Some("failure"))]).state,
            CiState::Red
        );
    }

    #[test]
    fn completed_cancelled_is_red() {
        assert_eq!(
            classify_runs(&[run("completed", Some("cancelled"))]).state,
            CiState::Red
        );
    }

    #[test]
    fn completed_timed_out_is_red() {
        // Per spec: all non-success conclusions render red.
        assert_eq!(
            classify_runs(&[run("completed", Some("timed_out"))]).state,
            CiState::Red
        );
    }

    #[test]
    fn completed_action_required_is_red() {
        assert_eq!(
            classify_runs(&[run("completed", Some("action_required"))]).state,
            CiState::Red
        );
    }

    #[test]
    fn completed_without_conclusion_is_unknown() {
        // Degenerate case: gh shouldn't return this, but defend anyway.
        assert_eq!(
            classify_runs(&[run("completed", None)]).state,
            CiState::Unknown
        );
    }

    #[test]
    fn running_wins_over_stale_red_or_green() {
        // When gh gives us only one run, and that run is in_progress, we
        // render Running even if a previous run was red. The spec
        // explicitly wants "running wins."
        assert_eq!(
            classify_runs(&[run("in_progress", None)]).state,
            CiState::Running
        );
    }

    #[test]
    fn classify_runs_preserves_metadata_fields_into_ci_event() {
        let event = classify_runs(&[run_with_meta(
            "completed",
            Some("failure"),
            "abc1234567",
            "fix: prevent silent drop",
            12345,
        )]);
        assert_eq!(event.state, CiState::Red);
        assert_eq!(event.sha.as_deref(), Some("abc1234567"));
        assert_eq!(event.title.as_deref(), Some("fix: prevent silent drop"));
        assert_eq!(event.run_id, Some(12345));
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

    // ─────────────────────────────────────────────────────────────────
    // #106 broadcast transition tests — pure function over the
    // state-transition table described in PM's shape comment. Unit
    // names because these test what the function returns, not what a
    // system does.
    // ─────────────────────────────────────────────────────────────────

    fn event(state: CiState) -> CiEvent {
        CiEvent {
            state,
            sha: None,
            title: None,
            run_id: None,
        }
    }

    fn event_with_meta(state: CiState, sha: &str, title: &str, id: u64) -> CiEvent {
        CiEvent {
            state,
            sha: Some(sha.to_string()),
            title: Some(title.to_string()),
            run_id: Some(id),
        }
    }

    #[test]
    fn compute_broadcast_emits_failure_on_first_red_from_startup() {
        // last_broadcast = None (startup), next = Red → should emit.
        // Failures always surface, even on the first known state.
        let text = compute_broadcast_text(None, &event(CiState::Red));
        assert!(text.is_some(), "startup-into-red should emit");
        let text = text.unwrap();
        assert!(text.contains("CI failed"), "got: {text}");
    }

    #[test]
    fn compute_broadcast_is_silent_on_first_green_from_startup() {
        // last_broadcast = None (startup), next = Green → silent.
        // "Successes only after failures" — a healthy pipeline is
        // quiet, not constantly announcing green.
        let text = compute_broadcast_text(None, &event(CiState::Green));
        assert!(text.is_none(), "startup-into-green should be silent");
    }

    #[test]
    fn compute_broadcast_emits_failure_on_green_to_red_transition() {
        let text = compute_broadcast_text(Some(CiState::Green), &event(CiState::Red));
        assert!(text.is_some());
        assert!(text.unwrap().contains("CI failed"));
    }

    #[test]
    fn compute_broadcast_emits_recovery_on_red_to_green_transition() {
        let text = compute_broadcast_text(Some(CiState::Red), &event(CiState::Green));
        assert!(text.is_some());
        let text = text.unwrap();
        assert!(text.contains("CI recovered"), "got: {text}");
    }

    #[test]
    fn compute_broadcast_is_silent_on_red_to_red() {
        // Already-broadcast red stays silent on the next red poll.
        // Prevents flapping between two red states from double-broadcasting.
        let text = compute_broadcast_text(Some(CiState::Red), &event(CiState::Red));
        assert!(text.is_none());
    }

    #[test]
    fn compute_broadcast_is_silent_on_green_to_green() {
        let text = compute_broadcast_text(Some(CiState::Green), &event(CiState::Green));
        assert!(text.is_none());
    }

    #[test]
    fn compute_broadcast_is_silent_on_transition_to_running() {
        // Running is an intermediate state, not an event. No broadcast.
        assert!(compute_broadcast_text(None, &event(CiState::Running)).is_none());
        assert!(compute_broadcast_text(Some(CiState::Green), &event(CiState::Running)).is_none());
        assert!(compute_broadcast_text(Some(CiState::Red), &event(CiState::Running)).is_none());
    }

    #[test]
    fn compute_broadcast_is_silent_on_transition_from_running_to_green() {
        // A passing build after running — same green as before the build.
        // Only broadcast if last_broadcast was Red. Running doesn't count.
        let text = compute_broadcast_text(Some(CiState::Running), &event(CiState::Green));
        assert!(text.is_none());
    }

    #[test]
    fn failure_message_includes_sha_title_and_run_link_when_metadata_present() {
        let ev = event_with_meta(
            CiState::Red,
            "abcdef1234",
            "fix: prevent silent drop",
            54321,
        );
        let text = compute_broadcast_text(None, &ev).unwrap();
        assert!(text.contains("CI failed"));
        assert!(text.contains("abcdef1"), "short SHA missing: {text}");
        assert!(
            text.contains("fix: prevent silent drop"),
            "title missing: {text}"
        );
        assert!(
            text.contains("gh run view 54321"),
            "run link missing: {text}"
        );
    }

    #[test]
    fn failure_message_truncates_long_titles() {
        let long_title = "a".repeat(100);
        let ev = event_with_meta(CiState::Red, "abcdef1234", &long_title, 999);
        let text = compute_broadcast_text(None, &ev).unwrap();
        // Should contain ellipsis, not the full 100-char title.
        assert!(text.contains('\u{2026}'), "expected ellipsis in: {text}");
        assert!(
            !text.contains(&long_title),
            "full title should have been truncated"
        );
    }

    #[test]
    fn failure_message_without_metadata_still_has_core_signal() {
        let text = compute_broadcast_text(None, &event(CiState::Red)).unwrap();
        assert!(text.contains("CI failed"));
        // No sha / title / run link, but the core signal remains.
        assert!(!text.contains("gh run view"));
    }

    #[test]
    fn recovery_message_includes_sha_and_run_link() {
        let ev = event_with_meta(CiState::Green, "cafebab", "fix ci", 999);
        let text = compute_broadcast_text(Some(CiState::Red), &ev).unwrap();
        assert!(text.contains("CI recovered"));
        assert!(text.contains("cafebab"));
        assert!(text.contains("gh run view 999"));
    }

    // ─────────────────────────────────────────────────────────────────
    // #188 multi-depth payload tests — pure function over the
    // `build_multi_depth_for_transition` helper. Unit-named per the
    // test-name discipline: describe what the function returns.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn build_multi_depth_for_failure_uses_red_glyph_summary() {
        let ev = event_with_meta(CiState::Red, "abcdef1", "fix: x", 42);
        let payload = build_multi_depth_for_transition(&ev, false);
        assert!(
            payload.summary.contains("CI failed"),
            "summary should describe failure, got: {}",
            payload.summary
        );
        assert!(payload.summary.contains('\u{1F534}'), "expected red glyph");
    }

    #[test]
    fn build_multi_depth_for_recovery_uses_green_glyph_summary() {
        let ev = event_with_meta(CiState::Green, "abcdef1", "fix: x", 42);
        let payload = build_multi_depth_for_transition(&ev, true);
        assert!(
            payload.summary.contains("CI recovered"),
            "summary should describe recovery, got: {}",
            payload.summary
        );
        assert!(
            payload.summary.contains('\u{1F7E2}'),
            "expected green glyph"
        );
    }

    #[test]
    fn build_multi_depth_sets_pm_and_designer_defaults_to_summary() {
        let ev = event_with_meta(CiState::Red, "abcdef1", "fix: x", 42);
        let payload = build_multi_depth_for_transition(&ev, false);
        // pm and designer default to the summary — binary-state
        // is all they need to make routing decisions.
        assert_eq!(payload.role_default.get("pm"), Some(&payload.summary));
        assert_eq!(payload.role_default.get("designer"), Some(&payload.summary));
    }

    #[test]
    fn build_multi_depth_sets_dev_default_to_detail_line_with_sha_and_run_link() {
        // Dev needs stage/commit context to diagnose a failure —
        // defaults to the rich form, not the summary.
        let ev = event_with_meta(
            CiState::Red,
            "abcdef1234",
            "fix: prevent silent drop",
            54321,
        );
        let payload = build_multi_depth_for_transition(&ev, false);
        let dev = payload.role_default.get("dev").expect("dev entry");
        assert!(dev.contains("CI failed"));
        assert!(dev.contains("abcdef1"), "short SHA missing: {dev}");
        assert!(dev.contains("fix: prevent silent drop"), "title missing");
        assert!(dev.contains("gh run view 54321"), "run link missing");
        // Dev's default is strictly richer than the summary — not equal.
        assert_ne!(dev, &payload.summary);
    }

    #[test]
    fn build_multi_depth_deep_detail_is_on_demand_when_run_id_present() {
        let ev = event_with_meta(CiState::Red, "abcdef1", "fix", 12345);
        let payload = build_multi_depth_for_transition(&ev, false);
        match payload.deep {
            DeepDetail::OnDemand {
                command,
                description,
            } => {
                assert!(
                    command.contains("gh run view 12345"),
                    "command should embed run id: {command}"
                );
                assert!(
                    !description.is_empty(),
                    "description should be a non-empty label"
                );
            }
            DeepDetail::Preloaded { .. } => {
                panic!("expected OnDemand when run_id is present")
            }
        }
    }

    #[test]
    fn build_multi_depth_deep_detail_is_preloaded_fallback_when_run_id_missing() {
        // Degenerate gh output case — no run_id available. Should
        // still produce a valid payload, falling back to a
        // Preloaded placeholder rather than panicking.
        let ev = event(CiState::Red); // defaults: all metadata None
        let payload = build_multi_depth_for_transition(&ev, false);
        match payload.deep {
            DeepDetail::Preloaded { text } => {
                assert!(!text.is_empty());
            }
            DeepDetail::OnDemand { .. } => {
                panic!("expected Preloaded fallback when run_id is missing")
            }
        }
    }

    #[test]
    fn build_multi_depth_recovery_dev_default_describes_recovery_not_failure() {
        // Guard against a copy-paste bug where recovery rendering
        // leaks the failure template. Dev's recovery default should
        // say "CI recovered", not "CI failed".
        let ev = event_with_meta(CiState::Green, "abcdef1", "fix", 42);
        let payload = build_multi_depth_for_transition(&ev, true);
        let dev = payload.role_default.get("dev").expect("dev entry");
        assert!(dev.contains("CI recovered"));
        assert!(!dev.contains("CI failed"));
    }

    #[test]
    fn build_multi_depth_role_default_has_entries_for_all_three_roles() {
        // Regression guard: if a future edit drops a role by
        // accident, this test fires. Three agent roles + optional
        // ryan (absent, falls back to summary).
        let ev = event_with_meta(CiState::Red, "abcdef1", "fix", 42);
        let payload = build_multi_depth_for_transition(&ev, false);
        assert_eq!(
            payload.role_default.len(),
            3,
            "expected entries for pm, designer, dev; got {:?}",
            payload.role_default.keys().collect::<Vec<_>>()
        );
        assert!(payload.role_default.contains_key("pm"));
        assert!(payload.role_default.contains_key("designer"));
        assert!(payload.role_default.contains_key("dev"));
    }
}
