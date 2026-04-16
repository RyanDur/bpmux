#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::tools_stories::*;
use super::tracker::StoryTracker;
use crate::transport::protocol::ClientMessage;

#[test]
fn role_from_name_extracts_short_role() {
    assert_eq!(role_from_name("dev:claude-code"), "dev");
    assert_eq!(role_from_name("pm"), "pm");
    assert_eq!(role_from_name("designer:cursor"), "designer");
    assert_eq!(role_from_name("ryan:human"), "ryan");
}

#[test]
fn require_role_allows_matching_role() {
    assert!(require_role("dev:claude-code", &["dev"], "start").is_ok());
    assert!(require_role("pm", &["pm"], "label").is_ok());
    assert!(require_role("ryan:human", &["ryan"], "accept").is_ok());
}

#[test]
fn require_role_rejects_non_matching_role() {
    let result = require_role("designer:claude-code", &["dev"], "start");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to dev"));
}

#[test]
fn require_role_rejects_pm_from_dev_operations() {
    let result = require_role("pm:claude-code", &["dev"], "deliver");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to dev"));
}

#[test]
fn require_role_rejects_dev_from_pm_operations() {
    let result = require_role("dev:claude-code", &["pm"], "label");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn require_role_rejects_agents_from_owner_only_operations() {
    let result = require_role("dev:claude-code", &["ryan"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));

    let result = require_role("pm:claude-code", &["ryan"], "reject");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to ryan"));
}

#[test]
fn require_role_allows_designer_for_desk_check_operations() {
    assert!(require_role("designer:claude-code", &["designer"], "pass desk-check").is_ok());
    assert!(require_role("designer", &["designer"], "fail desk-check").is_ok());
}

#[test]
fn require_role_rejects_non_designer_from_desk_check_operations() {
    for caller in ["dev:claude-code", "pm:claude-code", "ryan:human"] {
        let result = require_role(caller, &["designer"], "pass desk-check");
        assert!(
            result.is_err(),
            "{caller} should not pass desk-check; got: {result:?}"
        );
        assert!(result.unwrap_err().contains("restricted to designer"));
    }
}

// ── Workflow signal tests (#197) ──

#[test]
fn delivery_signals_target_designer_and_pm() {
    let signals = build_delivery_signals(191, "topic channels");
    assert_eq!(signals.len(), 2, "deliver should produce 2 signals");

    if let ClientMessage::Send { to, addressee, .. } = &signals[0] {
        assert_eq!(to.as_ref().unwrap(), &["designer"]);
        assert_eq!(addressee.as_deref(), Some("designer"));
    } else {
        panic!("expected Send variant");
    }

    if let ClientMessage::Send { to, addressee, .. } = &signals[1] {
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
        assert_eq!(addressee.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn delivery_signals_do_not_target_dev() {
    let signals = build_delivery_signals(191, "topic channels");
    for signal in &signals {
        if let ClientMessage::Send { to, .. } = signal {
            let recipients = to.as_ref().unwrap();
            assert!(
                !recipients.contains(&"dev".to_string()),
                "delivery signal should NOT target dev, got: {recipients:?}"
            );
        }
    }
}

#[test]
fn start_signal_targets_pm_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_start_signal(100) {
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
        assert_eq!(addressee.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn accept_signal_broadcasts_with_role_attribution() {
    if let ClientMessage::Send {
        to,
        addressee,
        text,
        ..
    } = build_accept_signal(100, "pm")
    {
        assert!(
            to.is_none(),
            "accept signal should broadcast (no privacy filter)"
        );
        assert_eq!(addressee.as_deref(), Some("dev"));
        assert!(
            text.contains("accepted by pm"),
            "signal should include who accepted. got: {text}"
        );
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn reject_signal_targets_dev_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_reject_signal(100, "needs fix") {
        assert_eq!(to.as_ref().unwrap(), &["dev"]);
        assert_eq!(addressee.as_deref(), Some("dev"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn reject_signal_includes_reason() {
    if let ClientMessage::Send { text, .. } = build_reject_signal(42, "missing AC") {
        assert!(
            text.contains("missing AC"),
            "reason should be in text: {text}"
        );
        assert!(
            text.contains("#42"),
            "story number should be in text: {text}"
        );
    }
}

// ── #203 desk-check signal tests ──

#[test]
fn pass_desk_check_signal_addresses_ryan_for_feature() {
    // Addressee is parameterized from addressee_for_pass_desk_check.
    let addressee = addressee_for_pass_desk_check(&["vertical".to_string()]);
    if let ClientMessage::Send {
        to, addressee: a, ..
    } = build_pass_desk_check_signal(203, addressee)
    {
        assert_eq!(to.as_ref().unwrap(), &["ryan"]);
        assert_eq!(a.as_deref(), Some("ryan"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signal_addresses_pm_for_honest_plumbing() {
    // Infrastructure stories route to PM (team-accept lane).
    let addressee = addressee_for_pass_desk_check(&["honest-plumbing".to_string()]);
    if let ClientMessage::Send {
        to, addressee: a, ..
    } = build_pass_desk_check_signal(203, addressee)
    {
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
        assert_eq!(a.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signal_addresses_pm_for_half_slice() {
    let addressee = addressee_for_pass_desk_check(&["half-slice".to_string()]);
    if let ClientMessage::Send { addressee: a, .. } = build_pass_desk_check_signal(203, addressee) {
        assert_eq!(a.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signal_defaults_ryan_on_ambiguous_labels() {
    // No recognizable verdict label → default up to Ryan.
    let addressee = addressee_for_pass_desk_check(&[]);
    assert_eq!(addressee, "ryan");
    let addressee = addressee_for_pass_desk_check(&["ready".to_string(), "bug".to_string()]);
    assert_eq!(addressee, "ryan");
}

#[test]
fn pass_desk_check_signal_does_not_target_dev() {
    let addressee = addressee_for_pass_desk_check(&[]);
    if let ClientMessage::Send { to, .. } = build_pass_desk_check_signal(203, addressee) {
        let recipients = to.as_ref().unwrap();
        assert!(
            !recipients.contains(&"dev".to_string()),
            "pass signal should not go to dev, got: {recipients:?}"
        );
    }
}

#[test]
fn pass_desk_check_signal_announces_ready_for_acceptance() {
    if let ClientMessage::Send { text, .. } = build_pass_desk_check_signal(203, "ryan") {
        assert!(
            text.contains("#203"),
            "story number should be in text: {text}"
        );
        assert!(
            text.contains("ready for acceptance"),
            "pass signal should say 'ready for acceptance': {text}"
        );
    }
}

#[test]
fn pass_desk_check_signals_emits_two_for_feature() {
    // Feature PASS emits two signals so PM gets the presentation cue
    // directly — without this, PM depends on scanning chat for the
    // designer's PASS message.
    let signals = build_pass_desk_check_signals(210, &["vertical".to_string()]);
    assert_eq!(signals.len(), 2, "feature PASS should emit two signals");
    let addressees: Vec<Option<String>> = signals
        .iter()
        .map(|m| match m {
            ClientMessage::Send { addressee, .. } => addressee.clone(),
            _ => None,
        })
        .collect();
    assert!(
        addressees.contains(&Some("ryan".to_string())),
        "should address ryan: {addressees:?}"
    );
    assert!(
        addressees.contains(&Some("pm".to_string())),
        "should address pm: {addressees:?}"
    );
}

#[test]
fn pass_desk_check_signals_emits_one_for_infra() {
    // Infra (honest-plumbing / half-slice) continues to route to PM
    // alone — PM is both presenter and acceptor for the team-accept
    // lane, so no companion signal is needed.
    let signals = build_pass_desk_check_signals(253, &["honest-plumbing".to_string()]);
    assert_eq!(signals.len(), 1, "infra PASS should emit one signal");
    if let ClientMessage::Send { addressee, .. } = &signals[0] {
        assert_eq!(addressee.as_deref(), Some("pm"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn pass_desk_check_signals_ambiguous_routes_to_ryan_and_pm() {
    // An unlabeled / ambiguously-labeled story defaults to Ryan's
    // lane (see addressee_for_pass_desk_check). That means the
    // feature-coroutine also fires — PM gets a presenter signal.
    let signals = build_pass_desk_check_signals(999, &[]);
    assert_eq!(signals.len(), 2);
}

#[test]
fn pass_desk_check_pm_present_signal_cues_presentation() {
    // Text distinguishes the PM-facing signal from the Ryan-facing
    // "ready for acceptance" — each role gets the call-to-action
    // appropriate to their role in the accept loop.
    if let ClientMessage::Send { text, to, .. } = build_pass_desk_check_pm_present_signal(210) {
        assert!(text.contains("#210"), "story number in text: {text}");
        assert!(
            text.contains("present to ryan"),
            "PM signal should cue presentation, got: {text}"
        );
        assert_eq!(to.as_ref().unwrap(), &["pm"]);
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn fail_desk_check_signal_targets_dev_only() {
    if let ClientMessage::Send { to, addressee, .. } = build_fail_desk_check_signal(203, "reason") {
        assert_eq!(to.as_ref().unwrap(), &["dev"]);
        assert_eq!(addressee.as_deref(), Some("dev"));
    } else {
        panic!("expected Send variant");
    }
}

#[test]
fn fail_desk_check_signal_does_not_target_ryan() {
    if let ClientMessage::Send { to, .. } = build_fail_desk_check_signal(203, "reason") {
        let recipients = to.as_ref().unwrap();
        assert!(
            !recipients.contains(&"ryan".to_string()),
            "fail signal should not go to ryan, got: {recipients:?}"
        );
    }
}

#[test]
fn fail_desk_check_signal_includes_reason() {
    if let ClientMessage::Send { text, .. } =
        build_fail_desk_check_signal(203, "missing negative case")
    {
        assert!(
            text.contains("missing negative case"),
            "reason should be in text: {text}"
        );
        assert!(
            text.contains("#203"),
            "story number should be in text: {text}"
        );
        assert!(
            text.contains("needs fixes"),
            "fail signal should say 'needs fixes': {text}"
        );
    }
}

// ── Handler unit tests with configurable fakes (#213) ──

#[derive(Clone)]
struct AlreadyClosedTracker;

impl StoryTracker for AlreadyClosedTracker {
    async fn close_story(&self, _number: u64) -> Result<String, String> {
        Err("issue is already closed".to_string())
    }
    async fn add_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("#{number} labeled '{label}'"))
    }
    async fn remove_label(&self, number: u64, label: &str) -> Result<String, String> {
        Ok(format!("#{number} unlabeled '{label}'"))
    }
    async fn create_story(&self, title: &str, _body: &str) -> Result<String, String> {
        Ok(format!("created: {title}"))
    }
    async fn list_stories(&self) -> Result<String, String> {
        Ok("[]".to_string())
    }
    async fn comment_on_story(&self, _number: u64, _body: &str) -> Result<String, String> {
        Ok("commented".to_string())
    }
    async fn get_story_title(&self, number: u64) -> Option<String> {
        Some(format!("story #{number}"))
    }
    async fn get_story_labels(&self, _number: u64) -> Vec<String> {
        Vec::new()
    }
}

#[tokio::test]
async fn accept_returns_noop_when_tracker_reports_already_closed() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({"number": 213});
    let result = handle_accept_story(
        args.as_object().unwrap(),
        "pm:claude-code",
        &ws_tx,
        &AlreadyClosedTracker,
    )
    .await;
    assert!(
        result.is_ok(),
        "already-closed should be Ok, got: {result:?}"
    );
    let msg = result.unwrap();
    assert!(
        msg.contains("already accepted"),
        "should say already accepted. got: {msg}"
    );
}

#[tokio::test]
async fn accept_rejects_unknown_role_via_handler() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({"number": 1});
    let result = handle_accept_story(
        args.as_object().unwrap(),
        "intern:claude-code",
        &ws_tx,
        &AlreadyClosedTracker,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to"));
}

// ── Contract tests for GhTracker (#213) ──
// These verify the real adapter works against the live gh CLI.
// Marked #[ignore] because they require gh authentication — run
// locally with `cargo test -- --ignored` when gh is configured.

#[tokio::test]
#[ignore]
async fn gh_tracker_list_stories_returns_parseable_json() {
    use super::tracker::GhTracker;
    let tracker = GhTracker {
        board_fixture: None,
    };
    let result = tracker.list_stories().await;
    assert!(
        result.is_ok(),
        "list_stories should succeed with gh auth. got: {result:?}"
    );
    let json_str = result.unwrap();
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).expect("list_stories should return valid JSON array");
    if let Some(first) = parsed.first() {
        assert!(
            first.get("number").is_some(),
            "each story should have a 'number' field. got: {first}"
        );
        assert!(
            first.get("title").is_some(),
            "each story should have a 'title' field. got: {first}"
        );
    }
}

#[tokio::test]
#[ignore]
async fn gh_tracker_get_story_title_returns_known_story() {
    use super::tracker::GhTracker;
    let tracker = GhTracker {
        board_fixture: None,
    };
    let title = tracker.get_story_title(213).await;
    assert!(title.is_some(), "story #213 should exist and have a title");
    let t = title.unwrap();
    assert!(!t.is_empty(), "title should not be empty. got: '{t}'");
}

// ── Contract tests for OctocrabStoryTracker (#221) ──
// Same pattern as GhTracker contract tests: #[ignore] + require auth.
// Run with `cargo test -- --ignored` locally when GH_TOKEN + BPMUX_REPO
// are set. CI skips them because CI has neither configured.

#[tokio::test]
#[ignore]
async fn octocrab_tracker_from_env_builds_when_auth_configured() {
    use super::tracker::OctocrabStoryTracker;
    let tracker = OctocrabStoryTracker::from_env();
    assert!(
        tracker.is_some(),
        "OctocrabStoryTracker should build with GH_TOKEN + BPMUX_REPO set"
    );
}

#[tokio::test]
#[ignore]
async fn octocrab_tracker_list_stories_returns_parseable_json() {
    use super::tracker::OctocrabStoryTracker;
    let tracker = OctocrabStoryTracker::from_env().expect("auth configured");
    let result = tracker.list_stories().await;
    assert!(
        result.is_ok(),
        "list_stories should succeed with auth. got: {result:?}"
    );
    let json_str = result.unwrap();
    let parsed: Vec<serde_json::Value> =
        serde_json::from_str(&json_str).expect("list_stories should return valid JSON array");
    if let Some(first) = parsed.first() {
        assert!(first.get("number").is_some(), "need 'number' field");
        assert!(first.get("title").is_some(), "need 'title' field");
        assert!(first.get("state").is_some(), "need 'state' field");
        assert!(first.get("labels").is_some(), "need 'labels' field");
    }
}

#[tokio::test]
#[ignore]
async fn octocrab_tracker_get_story_title_returns_known_story() {
    use super::tracker::OctocrabStoryTracker;
    let tracker = OctocrabStoryTracker::from_env().expect("auth configured");
    let title = tracker.get_story_title(213).await;
    assert!(title.is_some(), "story #213 should exist");
    assert!(!title.unwrap().is_empty(), "title non-empty");
}
