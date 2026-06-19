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
        name: None,
        annotation_message: None,
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
        name: None,
        annotation_message: None,
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
    assert_eq!(
        classify_runs(&[run("completed", None)]).state,
        CiState::Unknown
    );
}

#[test]
fn running_wins_over_stale_red_or_green() {
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

fn event(state: CiState) -> CiEvent {
    CiEvent {
        state,
        sha: None,
        title: None,
        run_id: None,
        workflows: Vec::new(),
    }
}

fn event_with_meta(state: CiState, sha: &str, title: &str, id: u64) -> CiEvent {
    CiEvent {
        state,
        sha: Some(sha.to_string()),
        title: Some(title.to_string()),
        run_id: Some(id),
        workflows: Vec::new(),
    }
}

#[test]
fn compute_broadcast_emits_failure_on_red_state() {
    let text = compute_broadcast_text(&event(CiState::Red));
    assert!(text.is_some(), "Red terminal state should emit");
    assert!(text.unwrap().contains("CI failed"));
}

#[test]
fn compute_broadcast_emits_green_on_green_state() {
    let text = compute_broadcast_text(&event(CiState::Green));
    assert!(text.is_some(), "Green terminal state should emit");
    assert!(text.unwrap().contains("CI green"));
}

#[test]
fn compute_broadcast_emits_paused_on_quota_exhausted_state() {
    let text = compute_broadcast_text(&event(CiState::QuotaExhausted));
    assert!(text.is_some());
    assert!(text.unwrap().contains("CI paused"));
}

#[test]
fn compute_broadcast_is_silent_on_running() {
    assert!(compute_broadcast_text(&event(CiState::Running)).is_none());
}

#[test]
fn compute_broadcast_is_silent_on_unknown() {
    assert!(compute_broadcast_text(&event(CiState::Unknown)).is_none());
}

#[test]
fn failure_message_includes_sha_title_and_run_link_when_metadata_present() {
    let ev = event_with_meta(
        CiState::Red,
        "abcdef1234",
        "fix: prevent silent drop",
        54321,
    );
    let text = compute_broadcast_text(&ev).unwrap();
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
    let text = compute_broadcast_text(&ev).unwrap();
    assert!(text.contains('\u{2026}'), "expected ellipsis in: {text}");
    assert!(
        !text.contains(&long_title),
        "full title should have been truncated"
    );
}

#[test]
fn failure_message_without_metadata_still_has_core_signal() {
    let text = compute_broadcast_text(&event(CiState::Red)).unwrap();
    assert!(text.contains("CI failed"));
    assert!(!text.contains("gh run view"));
}

#[test]
fn green_message_includes_sha_and_run_link() {
    let ev = event_with_meta(CiState::Green, "cafebab", "fix ci", 999);
    let text = compute_broadcast_text(&ev).unwrap();
    assert!(text.contains("CI green"));
    assert!(text.contains("cafebab"));
    assert!(text.contains("gh run view 999"));
}

#[test]
fn build_multi_depth_for_failure_uses_red_glyph_summary() {
    let ev = event_with_meta(CiState::Red, "abcdef1", "fix: x", 42);
    let payload = build_multi_depth_for_transition(&ev);
    assert!(
        payload.summary.contains("CI failed"),
        "summary should describe failure, got: {}",
        payload.summary
    );
    assert!(payload.summary.contains('\u{1F534}'), "expected red glyph");
}

#[test]
fn build_multi_depth_for_green_uses_green_glyph_summary() {
    let ev = event_with_meta(CiState::Green, "abcdef1", "fix: x", 42);
    let payload = build_multi_depth_for_transition(&ev);
    assert!(
        payload.summary.contains("CI green"),
        "summary should describe green completion, got: {}",
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
    let payload = build_multi_depth_for_transition(&ev);
    assert_eq!(payload.role_default.get("pm"), Some(&payload.summary));
    assert_eq!(payload.role_default.get("designer"), Some(&payload.summary));
}

#[test]
fn build_multi_depth_sets_dev_default_to_detail_line_with_sha_and_run_link() {
    let ev = event_with_meta(
        CiState::Red,
        "abcdef1234",
        "fix: prevent silent drop",
        54321,
    );
    let payload = build_multi_depth_for_transition(&ev);
    let dev = payload.role_default.get("dev").expect("dev entry");
    assert!(dev.contains("CI failed"));
    assert!(dev.contains("abcdef1"), "short SHA missing: {dev}");
    assert!(dev.contains("fix: prevent silent drop"), "title missing");
    assert!(dev.contains("gh run view 54321"), "run link missing");
    assert_ne!(dev, &payload.summary);
}

#[test]
fn build_multi_depth_deep_detail_is_on_demand_when_run_id_present() {
    let ev = event_with_meta(CiState::Red, "abcdef1", "fix", 12345);
    let payload = build_multi_depth_for_transition(&ev);
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
    let ev = event(CiState::Red);
    let payload = build_multi_depth_for_transition(&ev);
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
fn build_multi_depth_green_dev_default_describes_green_not_failure() {
    let ev = event_with_meta(CiState::Green, "abcdef1", "fix", 42);
    let payload = build_multi_depth_for_transition(&ev);
    let dev = payload.role_default.get("dev").expect("dev entry");
    assert!(dev.contains("CI green"));
    assert!(!dev.contains("CI failed"));
}

#[test]
fn build_multi_depth_role_default_has_entries_for_all_three_roles() {
    let ev = event_with_meta(CiState::Red, "abcdef1", "fix", 42);
    let payload = build_multi_depth_for_transition(&ev);
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

#[test]
fn resolve_ci_actions_url_falls_back_to_github_base_when_unset() {
    assert_eq!(resolve_ci_actions_url(None, None), GITHUB_BASE_URL);
}

#[test]
fn resolve_ci_actions_url_derives_actions_tab_from_repo() {
    assert_eq!(
        resolve_ci_actions_url(None, Some("acme/bpmux".to_string())),
        "https://github.com/acme/bpmux/actions"
    );
}

#[test]
fn resolve_ci_actions_url_honors_configured_override() {
    assert_eq!(
        resolve_ci_actions_url(
            Some("https://example.test/ci".to_string()),
            Some("acme/bpmux".to_string())
        ),
        "https://example.test/ci"
    );
}

#[test]
fn resolve_ci_actions_url_ignores_blank_override_and_derives_from_repo() {
    assert_eq!(
        resolve_ci_actions_url(Some("   ".to_string()), Some("acme/bpmux".to_string())),
        "https://github.com/acme/bpmux/actions"
    );
}
