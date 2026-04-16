#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::ci::*;
use crate::session::message::DeepDetail;

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

// ── #215: dev-addressed CI signal ──

#[test]
fn build_dev_ci_signal_addresses_dev_only() {
    let signal = build_dev_ci_signal("CI failed — abc123");
    assert_eq!(
        signal.to.as_ref().unwrap(),
        &vec!["dev".to_string()],
        "signal should be scoped to dev (privacy filter)"
    );
    assert_eq!(
        signal.addressee.as_deref(),
        Some("dev"),
        "signal should call dev's attention"
    );
}

#[test]
fn build_dev_ci_signal_uses_chat_message_type() {
    // Chat (not Activity) so the bridge pushes it to dev's
    // notifications. Activity messages are filtered out of agent
    // chat streams per #152.
    let signal = build_dev_ci_signal("anything");
    assert_eq!(signal.message_type, crate::session::MessageType::Chat);
}

#[test]
fn build_dev_ci_signal_preserves_text() {
    let text = "🔴 CI failed — abc1234 \"feat: thing\" → gh run view 999";
    let signal = build_dev_ci_signal(text);
    assert_eq!(signal.text, text);
}

#[test]
fn build_dev_ci_signal_from_is_bpmux() {
    let signal = build_dev_ci_signal("any");
    assert_eq!(signal.from, "bpmux");
}
