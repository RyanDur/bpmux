#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[cfg(feature = "integration_tests")]
use super::tracker::StoryTracker;

use super::tools_stories::*;
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
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
fn require_role_rejects_dev_from_close_iteration() {
    let result = require_role("dev:claude-code", &["pm"], "close iteration");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn require_role_rejects_designer_from_close_iteration() {
    let result = require_role("designer:claude-code", &["pm"], "close iteration");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn require_role_rejects_ryan_from_close_iteration() {
    let result = require_role("ryan:human", &["pm"], "close iteration");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[test]
fn build_iteration_close_signal_broadcasts_to_all() {
    let signal = build_iteration_close_signal();
    let ClientMessage::Send {
        to,
        addressee,
        text,
        message_type,
    } = signal
    else {
        panic!("expected Send variant");
    };
    assert!(to.is_none(), "to should be None for full broadcast");
    assert_eq!(
        addressee.as_deref(),
        Some("all"),
        "addressee must be 'all' so every agent treats it as their turn"
    );
    assert!(
        text.contains("[iteration-close]"),
        "text must carry the [iteration-close] marker so agents can recognize the signal: got {text}"
    );
    assert!(
        text.contains("flush unsaved findings"),
        "text must name the externalize action: got {text}"
    );
    assert!(message_type.is_none());
}

#[test]
fn build_board_invalidate_signal_is_system_typed_broadcast() {
    let signal = build_board_invalidate_signal();
    let ClientMessage::Send {
        to,
        addressee,
        text,
        message_type,
    } = signal
    else {
        panic!("expected Send variant");
    };
    assert!(
        to.is_none(),
        "to must be None so every connected TUI refreshes, not just one"
    );
    assert!(
        addressee.is_none(),
        "addressee must be None — not an attention signal, just a state-change nudge"
    );
    assert_eq!(
        text, BOARD_INVALIDATE_MARKER,
        "text must be the exact marker constant so event.rs can structurally match it"
    );
    assert_eq!(
        message_type.as_deref(),
        Some("system"),
        "message_type must be 'system' so the TUI suppresses chat rendering"
    );
}

// #316 close_story MCP tool — PM-only. Permission-gate tests here;
// behavior is exercised via the migrated RefreshDriver path tests.

#[test]
fn close_story_is_pm_only_gate() {
    for caller in ["dev:claude-code", "designer:claude-code", "ryan:human"] {
        let result = require_role(caller, &["pm"], "close");
        assert!(
            result.is_err(),
            "{caller} should not pass close_story permission gate"
        );
        assert!(result.unwrap_err().contains("restricted to pm"));
    }
}

#[test]
fn close_story_allows_pm() {
    assert!(require_role("pm:claude-code", &["pm"], "close").is_ok());
}

// #317 comment_on_story — any-agent; handler rejects empty-body so
// issue comments can't be silent. ws_tx Mutex<None> exercises the
// fail-fast-before-broadcast path (no ws connection needed).

#[tokio::test]
async fn comment_on_story_rejects_empty_body() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(42));
    args.insert("body".to_string(), serde_json::json!(""));
    let result = handle_comment_on_story(&args, "dev:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "empty body must be rejected: got {result:?}"
    );
    assert!(result.unwrap_err().contains("must not be empty"));
}

#[tokio::test]
async fn comment_on_story_rejects_whitespace_only_body() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".to_string(), serde_json::json!(42));
    args.insert("body".to_string(), serde_json::json!("   \n  "));
    let result = handle_comment_on_story(&args, "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err(), "whitespace-only body must be rejected");
}

#[tokio::test]
async fn comment_on_story_rejects_when_refresh_absent() {
    let ws_tx = tokio::sync::Mutex::new(None);
    for caller in ["dev:claude-code", "pm:claude-code", "designer:claude-code"] {
        let mut args = serde_json::Map::new();
        args.insert("number".to_string(), serde_json::json!(42));
        args.insert("body".to_string(), serde_json::json!("a real comment"));
        let result = handle_comment_on_story(&args, caller, &ws_tx, None).await;
        assert!(
            result.is_err(),
            "caller={caller} expected Err, got {result:?}"
        );
        assert!(
            result.unwrap_err().contains("requires refresh"),
            "expected refresh-required error for caller={caller}"
        );
    }
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
fn pass_desk_check_signal_addresses_ryan_for_feature_label() {
    let addressee = addressee_for_pass_desk_check(StoryType::Feature);
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
fn pass_desk_check_signal_addresses_pm_for_chore_label() {
    let addressee = addressee_for_pass_desk_check(StoryType::Chore);
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
fn pass_desk_check_signal_addresses_pm_for_bug_label() {
    // Bugs are team-accepted by default in the 3-type vocabulary.
    let addressee = addressee_for_pass_desk_check(StoryType::Bug);
    assert_eq!(addressee, "pm");
}

#[test]
fn pass_desk_check_signal_falls_back_to_title_prefix() {
    // No labels yet — classifier reads the title prefix so the
    // backfill-lag doesn't misroute Chore stories to Ryan.
    let addressee = addressee_for_pass_desk_check(classify_story(&[], "[Chore] split app_state"));
    assert_eq!(addressee, "pm");
    let addressee = addressee_for_pass_desk_check(classify_story(&[], "[Feature] new view"));
    assert_eq!(addressee, "ryan");
}

#[test]
fn pass_desk_check_signal_defaults_ryan_on_empty() {
    // Default-to-Feature: un-tagged stories route to Ryan.
    let addressee = addressee_for_pass_desk_check(classify_story(&[], ""));
    assert_eq!(addressee, "ryan");
    let addressee =
        addressee_for_pass_desk_check(classify_story(&["ready".to_string()], "unlabeled story"));
    assert_eq!(addressee, "ryan");
}

#[test]
fn pass_desk_check_signal_does_not_target_dev() {
    let addressee = addressee_for_pass_desk_check(classify_story(&[], ""));
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
    let signals = build_pass_desk_check_signals(210, StoryType::Feature);
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
    let signals = build_pass_desk_check_signals(253, StoryType::Chore);
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
    let signals = build_pass_desk_check_signals(999, classify_story(&[], ""));
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

#[tokio::test]
async fn accept_requires_refresh_after_5n_fallback_retirement() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({"number": 213});
    let result =
        handle_accept_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "expected refresh-required Err, got {result:?}"
    );
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn accept_rejects_unknown_role_via_handler() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({"number": 1});
    let result = handle_accept_story(
        args.as_object().unwrap(),
        "intern:claude-code",
        &ws_tx,
        None,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to"));
}

// ── #265: reject_story opened to team roles ──────────────────
//
// Parallel to #213's accept_story role-gate opening. PM/designer/dev
// fire rejections on Ryan's behalf when he says "reject," so the tool
// has a callable path. Unknown roles still error.

#[tokio::test]
async fn reject_allows_pm_designer_dev_owner() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    for caller in [
        "ryan:human",
        "pm:claude-code",
        "designer:claude-code",
        "dev:claude-code",
    ] {
        let args = serde_json::json!({"number": 1, "reason": "needs rework"});
        let result = handle_reject_story(args.as_object().unwrap(), caller, &ws_tx, None).await;
        assert!(
            result.is_ok() || !result.as_ref().unwrap_err().contains("restricted to"),
            "reject must allow {caller}; got: {result:?}"
        );
    }
}

#[tokio::test]
async fn reject_rejects_unknown_role_via_handler() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({"number": 1, "reason": "test"});
    let result = handle_reject_story(
        args.as_object().unwrap(),
        "intern:claude-code",
        &ws_tx,
        None,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to"));
}

// Awareness signal for feature-gated integration tests (#299 rework
// feedback from desk-check). `#[cfg(feature = ...)]` removes the tests
// from default `cargo test` output entirely — AC3 originally asked for
// a "skipped X real-substrate tests" message so devs know they exist.
// This always-compiled test carries the signal in its NAME so the
// default cargo-test listing surfaces it. Passes trivially; the
// signal is the name appearing in the output.
#[test]
fn integration_tests_are_feature_gated_enable_with_features_integration_tests() {
    // See `Cargo.toml` [features] + the `#[cfg(feature = "integration_tests")]`
    // gates in this file. Run with:
    //   cargo test --features integration_tests
    // Requires `GH_TOKEN` + `BPMUX_REPO` env vars.
    // 5 real-substrate contract tests: 2 against gh CLI + 3 against
    // octocrab REST.
}

// GhTracker contract tests retired in #320 (gh CLI fallback deleted).
// Octocrab is the sole production path; tests below exercise it.

// ── Contract tests for OctocrabStoryTracker (#221) ──
// Same pattern as GhTracker contract tests: feature-gated (#299 rework)
// instead of `#[ignore]` + require auth. Run with
// `cargo test --features integration_tests` when GH_TOKEN + BPMUX_REPO
// are set. Default CI skips because the feature isn't enabled there.

#[cfg(feature = "integration_tests")]
#[tokio::test]
async fn octocrab_tracker_from_env_builds_when_auth_configured() {
    use super::tracker::OctocrabStoryTracker;
    let tracker = OctocrabStoryTracker::from_env();
    assert!(
        tracker.is_some(),
        "OctocrabStoryTracker should build with GH_TOKEN + BPMUX_REPO set"
    );
}

#[cfg(feature = "integration_tests")]
#[tokio::test]
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

#[cfg(feature = "integration_tests")]
#[tokio::test]
async fn octocrab_tracker_get_story_title_returns_known_story() {
    use super::tracker::OctocrabStoryTracker;
    let tracker = OctocrabStoryTracker::from_env().expect("auth configured");
    let title = tracker.get_story_title(213).await;
    assert!(title.is_some(), "story #213 should exist");
    assert!(!title.unwrap().is_empty(), "title non-empty");
}

#[cfg(feature = "integration_tests")]
#[tokio::test]
async fn spike_451_ingest_latency_benchmark() {
    use super::tracker::OctocrabStoryTracker;
    use std::time::Instant;
    let tracker = OctocrabStoryTracker::from_env().expect("auth configured");
    eprintln!("\n=== Spike 1 (#451): ingest-all latency ===");
    let t0 = Instant::now();
    let stories = tracker.fetch_board_stories().await.expect("board fetch");
    let fetch = t0.elapsed();
    let n = stories.len();
    let bytes: usize = stories.iter().map(|s| s.to_string().len()).sum();
    eprintln!(
        "fetch_board_stories: {n} stories in {fetch:?}  —  {} KB serialized",
        bytes / 1024
    );
    let active: Vec<u64> = stories
        .iter()
        .filter_map(|s| {
            let state = s.get("state").and_then(|v| v.as_str())?;
            if matches!(state, "ready" | "in-progress" | "delivered" | "accepted") {
                s.get("number").and_then(|v| v.as_u64())
            } else {
                None
            }
        })
        .collect();
    let sample = active.len().min(30);
    let t1 = Instant::now();
    let mut total_c = 0usize;
    let mut total_cb = 0usize;
    for n in active.iter().take(sample) {
        if let Ok(cs) = tracker.list_story_comments(*n).await {
            total_c += cs.len();
            total_cb += cs.iter().map(|c| c.to_string().len()).sum::<usize>();
        }
    }
    let comments_elapsed = t1.elapsed();
    eprintln!(
        "list_story_comments sample: {} comments across {} stories in {comments_elapsed:?}  —  {} KB",
        total_c,
        sample,
        total_cb / 1024
    );
    let projected = if sample > 0 {
        let per_story = comments_elapsed.as_millis() / sample as u128;
        per_story * active.len() as u128
    } else {
        0
    };
    eprintln!(
        "projected comments-for-all-active: ~{}ms wall-clock, {} active stories",
        projected,
        active.len()
    );
    eprintln!(
        "=== total eager-ingest estimate: ~{}ms, ~{} KB memory (JSON) ===\n",
        fetch.as_millis() + projected,
        (bytes + total_cb * active.len() / sample.max(1)) / 1024
    );
}

// ── #391: observation-falsifier gate tests ──

#[tokio::test]
async fn create_story_rejects_non_pm_role() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "dev-filed story",
        "user_job": "as dev, I want X",
        "observation_falsifier": "reject if X"
    });
    for role in ["dev", "designer", "ryan", "unknown"].iter() {
        let result = handle_create_story(args.as_object().unwrap(), role, &ws_tx, None).await;
        assert!(
            result.is_err(),
            "create_story should reject {role} calls; got: {result:?}"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("pm-lane"),
            "error should name pm-lane. role={role}, err={err}"
        );
        assert!(
            err.contains("broadcast"),
            "error should redirect to broadcast. role={role}, err={err}"
        );
    }
}

#[tokio::test]
async fn create_story_pm_role_passes_role_gate_then_hits_refresh_required() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "pm-filed story",
        "user_job": "as pm, I want X",
        "observation_falsifier": "reject if X"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn create_story_pm_with_tool_suffix_role_gate_passes_then_refresh_required() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "pm-filed story",
        "user_job": "as pm, I want X",
        "observation_falsifier": "reject if X"
    });
    let result =
        handle_create_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn create_story_rejects_missing_user_job() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "body": "body text",
        "observation_falsifier": "I would reject if X"
        // user_job missing
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err(), "missing user_job should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("user_job"),
        "error should name the field. got: {err}"
    );
    assert!(
        err.contains("retro") || err.contains("user-job"),
        "error should cite the retro framing. got: {err}"
    );
}

#[tokio::test]
async fn create_story_rejects_empty_user_job() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "user_job": "   ",
        "observation_falsifier": "I would reject if X"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err(), "whitespace-only user_job should fail");
    assert!(result.unwrap_err().contains("user_job"));
}

#[tokio::test]
async fn create_story_rejects_missing_observation_falsifier() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "user_job": "When I X, I need Y because Z"
        // observation_falsifier missing
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err(), "missing observation_falsifier should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("observation_falsifier"),
        "error should name the field. got: {err}"
    );
}

#[tokio::test]
async fn create_story_validation_passes_then_refresh_required_after_5n() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "title": "a story",
        "body": "author body",
        "user_job": "When I X, I need Y because Z",
        "observation_falsifier": "I would reject if I observed W"
    });
    let result = handle_create_story(args.as_object().unwrap(), "pm", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn deliver_story_rejects_missing_verified_observation() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 391
        // verified_observation missing
    });
    let result =
        handle_deliver_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err(), "missing verified_observation should fail");
    let err = result.unwrap_err();
    assert!(
        err.contains("verified_observation"),
        "error should name the field. got: {err}"
    );
    assert!(
        err.contains("retro") || err.contains("drove"),
        "error should cite retro framing / drove-end-to-end. got: {err}"
    );
}

#[tokio::test]
async fn deliver_story_rejects_empty_verified_observation() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 391,
        "verified_observation": ""
    });
    let result =
        handle_deliver_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err(), "empty verified_observation should fail");
    assert!(result.unwrap_err().contains("verified_observation"));
}

#[tokio::test]
async fn label_story_default_op_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_add_op_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "blocked",
        "operation": "add"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_remove_op_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "in progress",
        "operation": "remove"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_remove_idempotent_call_requires_refresh_after_5n() {
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 99,
        "label": "never-attached-label",
        "operation": "remove"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn label_story_rejects_invalid_operation_in_migrated_path() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        42,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "nuke"
    });
    let result = handle_label_story(
        args.as_object().unwrap(),
        "pm:claude-code",
        &ws_tx,
        Some(&orchestrator),
    )
    .await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("operation"));
    assert!(err.contains("add") && err.contains("remove"));
}

#[tokio::test]
async fn label_story_add_propagates_tracker_error_without_broadcast() {
    // Falsifier guard: when the GitHub API errors on add, the
    // handler must return the error and NOT fire BOARD_INVALIDATE
    // — peers shouldn't refresh against a no-op change. The Err
    // return path structurally short-circuits before broadcast.
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "add"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "add-with-API-error must surface error: {result:?}"
    );
}

#[tokio::test]
async fn label_story_remove_propagates_tracker_error_without_broadcast() {
    // Same falsifier guard on remove path — a genuine API error
    // (500, network, auth) must return Err and skip broadcast. The
    // 404-label-not-attached case is distinct: tracker maps that to
    // Ok, so the handler proceeds to broadcast (idempotent-success
    // contract, #392 AC3).
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "remove"
    });
    let result =
        handle_label_story(args.as_object().unwrap(), "pm:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "remove-with-API-error must surface error: {result:?}"
    );
}

#[tokio::test]
async fn label_story_still_pm_gated_with_operation_param() {
    // Permission gate must fire before operation parsing — a dev /
    // designer / ryan call with operation="remove" should fail with
    // a role rejection, not with an operation rejection. #392
    // doesn't relax the existing PM-only scope.
    let ws_tx = tokio::sync::Mutex::new(None);
    let args = serde_json::json!({
        "number": 42,
        "label": "ready",
        "operation": "remove"
    });
    for caller in ["dev:claude-code", "designer:claude-code", "ryan:human"] {
        let result = handle_label_story(args.as_object().unwrap(), caller, &ws_tx, None).await;
        assert!(result.is_err(), "{caller} must be rejected by PM-only gate");
        assert!(
            result.unwrap_err().contains("restricted to pm"),
            "{caller} rejection should cite role restriction"
        );
    }
}

// ── #398 start_story pickup-gate completeness tests ───────────────

#[tokio::test]
async fn start_story_rejects_missing_pickup_paraphrase() {
    // Schema requires it; handler also validates so pre-MCP callers
    // (tests) fail-fast with a descriptive message.
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 398,
        "falsifier_reframing": "I would reject if the pickup signal is invisible."
    });
    let result =
        handle_start_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "missing pickup_paraphrase must fail: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("pickup_paraphrase"),
        "error should name the field: {err}"
    );
    assert!(
        err.contains("retro") || err.contains("pickup time"),
        "error should cite retro framing so agents know the why: {err}"
    );
}

#[tokio::test]
async fn start_story_rejects_empty_pickup_paraphrase() {
    // Whitespace-only must also fail — matches create_story + deliver
    // validation semantics (trim + empty-check).
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "   \n  ",
        "falsifier_reframing": "I would reject if..."
    });
    let result =
        handle_start_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "whitespace-only pickup_paraphrase must fail"
    );
    assert!(result.unwrap_err().contains("pickup_paraphrase"));
}

#[tokio::test]
async fn start_story_rejects_missing_falsifier_reframing() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "As a dev, I restate the user-job..."
    });
    let result =
        handle_start_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(
        result.is_err(),
        "missing falsifier_reframing must fail: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("falsifier_reframing"),
        "error should name the field: {err}"
    );
}

#[tokio::test]
async fn start_story_validation_passes_then_refresh_required_after_5n() {
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "User-job: dev articulates their read of the story at pickup so the team knows work is in flight.",
        "falsifier_reframing": "I would reject if start_story without the new fields still succeeds."
    });
    let result =
        handle_start_story(args.as_object().unwrap(), "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn start_story_still_dev_only_with_new_params() {
    // Role gate fires before param parsing — a pm / designer / ryan
    // call with both new fields populated must still be rejected as
    // "restricted to dev." #398 doesn't relax the existing scope.
    let ws_tx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
    let args = serde_json::json!({
        "number": 398,
        "pickup_paraphrase": "any text",
        "falsifier_reframing": "any text"
    });
    for caller in ["pm:claude-code", "designer:claude-code", "ryan:human"] {
        let result = handle_start_story(args.as_object().unwrap(), caller, &ws_tx, None).await;
        assert!(
            result.is_err(),
            "{caller} must be rejected by dev-only gate"
        );
        assert!(
            result.unwrap_err().contains("restricted to dev"),
            "{caller} rejection should cite role restriction"
        );
    }
}

#[tokio::test]
async fn comment_on_story_migrated_path_enqueues_event_via_orchestrator() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        123,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(123));
    args.insert(
        "body".into(),
        serde_json::json!("new comment via orchestrator"),
    );
    let result =
        handle_comment_on_story(&args, "dev:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(123).unwrap();
    let comments = story.free_comments.expect("FreeCommentAppended applied");
    assert_eq!(comments.len(), 1);
    assert_eq!(comments[0].body, "new comment via orchestrator");
    assert_eq!(comments[0].author, "dev");

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1, "event must reach the adapter via drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::FreeCommentAppended { number: 123, .. }
    ));
}

#[tokio::test]
async fn label_story_migrated_path_enqueues_tag_added_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, Tag};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        200,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(200));
    args.insert("label".into(), serde_json::json!("blocked"));
    args.insert("operation".into(), serde_json::json!("add"));
    let result = handle_label_story(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(200).unwrap();
    assert_eq!(story.tags, vec![Tag::Blocked]);

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1, "event must reach the adapter via drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagAdded { number: 200, label, .. } if label == "blocked"
    ));
}

#[tokio::test]
async fn label_story_migrated_path_enqueues_tag_removed_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        200,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = tokio::sync::Mutex::new(None);

    let mut add_args = serde_json::Map::new();
    add_args.insert("number".into(), serde_json::json!(200));
    add_args.insert("label".into(), serde_json::json!("blocked"));
    add_args.insert("operation".into(), serde_json::json!("add"));
    handle_label_story(&add_args, "pm:claude-code", &ws_tx, Some(&orchestrator))
        .await
        .unwrap();

    let mut remove_args = serde_json::Map::new();
    remove_args.insert("number".into(), serde_json::json!(200));
    remove_args.insert("label".into(), serde_json::json!("blocked"));
    remove_args.insert("operation".into(), serde_json::json!("remove"));
    let result =
        handle_label_story(&remove_args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(
        result.is_ok(),
        "migrated remove path should succeed: {result:?}"
    );

    let story = store.story(200).unwrap();
    assert!(story.tags.is_empty(), "tags should be empty after remove");

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 2, "both events must reach the adapter via drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagAdded { number: 200, label, .. } if label == "blocked"
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::TagRemoved { number: 200, label, .. } if label == "blocked"
    ));
}

async fn connected_ws_sink_with_capture() -> (
    tokio::sync::Mutex<Option<super::tools::WsSink>>,
    std::sync::Arc<tokio::sync::Mutex<Vec<String>>>,
) {
    use futures::StreamExt;
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let captured: std::sync::Arc<tokio::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new()));
    let captured_for_task = std::sync::Arc::clone(&captured);
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await
            && let Ok(ws) = tokio_tungstenite::accept_async(stream).await
        {
            let (_srv_sink, mut srv_stream) = ws.split();
            while let Some(Ok(msg)) = srv_stream.next().await {
                if let Ok(text) = msg.into_text() {
                    captured_for_task.lock().await.push(text.to_string());
                }
            }
        }
    });
    let url = format!("ws://{}/", addr);
    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (sink, stream) = ws.split();
    tokio::spawn(async move {
        let mut stream = stream;
        while stream.next().await.is_some() {}
    });
    (tokio::sync::Mutex::new(Some(sink)), captured)
}

async fn connected_ws_sink_for_test() -> tokio::sync::Mutex<Option<super::tools::WsSink>> {
    use futures::StreamExt;
    use tokio::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await
            && let Ok(ws) = tokio_tungstenite::accept_async(stream).await
        {
            let (_srv_sink, mut srv_stream) = ws.split();
            while srv_stream.next().await.is_some() {}
        }
    });
    let url = format!("ws://{}/", addr);
    let (ws, _) = tokio_tungstenite::connect_async(&url).await.unwrap();
    let (sink, stream) = ws.split();
    tokio::spawn(async move {
        let mut stream = stream;
        while stream.next().await.is_some() {}
    });
    tokio::sync::Mutex::new(Some(sink))
}

#[tokio::test]
async fn pass_desk_check_migrated_path_emits_gate_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        201,
        "t".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(201));
    let result =
        handle_pass_desk_check(&args, "designer:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(201).unwrap();
    assert_eq!(story.state, StoryState::ReadyForAccept);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates.iter().any(|g| g.gate == Gate::PassDeskCheck),
        "gate_comments should contain PassDeskCheck entry: {gates:?}"
    );

    // #450: pass_desk_check now drains internally + emits TagRemoved
    // {delivered} alongside the gate + state events. 3 events expected.
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 3, "3 events expected: {pushed:?}");
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagRemoved { number: 201, label, .. } if label == "delivered"
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 201,
            gate: Gate::PassDeskCheck,
            payload: None,
            ..
        }
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 201,
            new_state: StoryState::ReadyForAccept,
            ..
        }
    ));
}

#[tokio::test]
async fn fail_desk_check_migrated_path_emits_gate_with_reason_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        202,
        "t".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(202));
    args.insert("reason".into(), serde_json::json!("test failure"));
    let result =
        handle_fail_desk_check(&args, "designer:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(202).unwrap();
    assert_eq!(story.state, StoryState::NeedsFixes);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates
            .iter()
            .any(|g| g.gate == Gate::FailDeskCheck && g.payload.as_deref() == Some("test failure")),
        "gate_comments should contain FailDeskCheck with payload: {gates:?}"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 2, "both events must reach the adapter via drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert!(matches!(
        &pushed[0],
        BoardEvent::GateCommentAppended {
            number: 202,
            gate: Gate::FailDeskCheck,
            payload: Some(p),
            ..
        } if p == "test failure"
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::StateTransitioned {
            number: 202,
            new_state: StoryState::NeedsFixes,
            ..
        }
    ));
}

#[tokio::test]
async fn start_story_migrated_path_emits_pickup_gate_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        203,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = crate::board::domain::StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(203));
    args.insert(
        "pickup_paraphrase".into(),
        serde_json::json!("paraphrase content"),
    );
    args.insert(
        "falsifier_reframing".into(),
        serde_json::json!("reframing content"),
    );
    let result = handle_start_story(&args, "dev:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(203).unwrap();
    assert_eq!(story.state, StoryState::InProgress);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates.iter().any(|g| g.gate == Gate::Pickup
            && g.payload.as_deref().is_some_and(
                |p| p.contains("paraphrase content") && p.contains("reframing content")
            )),
        "gate_comments should contain Pickup with both payload sections: {gates:?}"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 2, "both events must reach the adapter via drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert!(matches!(
        &pushed[0],
        BoardEvent::GateCommentAppended {
            number: 203,
            gate: Gate::Pickup,
            payload: Some(p),
            ..
        } if p.contains("paraphrase content") && p.contains("reframing content")
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::StateTransitioned {
            number: 203,
            new_state: StoryState::InProgress,
            ..
        }
    ));
}

#[tokio::test]
async fn deliver_story_migrated_path_emits_delivered_gate_with_observation_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        204,
        "t".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = crate::board::domain::StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(204));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("drove the falsifier scenario end-to-end"),
    );
    let result = handle_deliver_story(&args, "dev:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(204).unwrap();
    assert_eq!(story.state, StoryState::Delivered);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates.iter().any(|g| g.gate == Gate::Delivered
            && g.payload.as_deref() == Some("drove the falsifier scenario end-to-end")),
        "gate_comments should contain Delivered with the observation payload: {gates:?}"
    );

    // #450: deliver_story now drains pending internally so label-remove
    // failures surface load-bearing. Test asserts the 3 events landed at
    // the adapter (TagRemoved + GateCommentAppended + StateTransitioned).
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 3, "3 events expected: {pushed:?}");
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagRemoved {
            number: 204,
            label,
            ..
        } if label == "in progress"
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 204,
            gate: Gate::Delivered,
            payload: Some(p),
            ..
        } if p == "drove the falsifier scenario end-to-end"
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 204,
            new_state: StoryState::Delivered,
            ..
        }
    ));
}

#[tokio::test]
async fn accept_story_migrated_path_emits_tag_remove_gate_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        205,
        "t".into(),
        StoryState::ReadyForAccept,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(205));
    let result = handle_accept_story(&args, "ryan:human", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(205).unwrap();
    assert_eq!(story.state, StoryState::Accepted);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates
            .iter()
            .any(|g| g.gate == Gate::Accepted && g.payload.is_none()),
        "gate_comments should contain Accepted with no payload: {gates:?}"
    );

    // #450: accept_story now drains internally + emits an additional
    // TagRemoved{ready-for-accept} alongside TagRemoved{delivered}.
    // 5 events expected: 2x TagRemoved + GateCommentAppended +
    // StateTransitioned + StoryClosed.
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 5, "5 events expected: {pushed:?}");
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagRemoved { number: 205, label, .. } if label == "delivered"
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::TagRemoved { number: 205, label, .. } if label == "ready for accept"
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::GateCommentAppended {
            number: 205,
            gate: Gate::Accepted,
            payload: None,
            ..
        }
    ));
    assert!(matches!(
        &pushed[3],
        BoardEvent::StateTransitioned {
            number: 205,
            new_state: StoryState::Accepted,
            ..
        }
    ));
    assert!(matches!(
        &pushed[4],
        BoardEvent::StoryClosed {
            number: 205,
            reason: None,
            ..
        }
    ));
    assert!(
        store.story(205).expect("story exists").is_closed,
        "is_closed flag must be set after StoryClosed event applies"
    );
}

#[tokio::test]
async fn reject_story_migrated_path_emits_tag_remove_gate_with_reason_and_state_events() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        206,
        "t".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(206));
    args.insert("reason".into(), serde_json::json!("scope mismatch"));
    let result = handle_reject_story(&args, "ryan:human", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(206).unwrap();
    assert_eq!(story.state, StoryState::Rejected);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates
            .iter()
            .any(|g| g.gate == Gate::Rejected && g.payload.as_deref() == Some("scope mismatch")),
        "gate_comments should contain Rejected with reason payload: {gates:?}"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 3,
        "all three events must reach the adapter via drain"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 3);
    assert!(matches!(
        &pushed[0],
        BoardEvent::TagRemoved { number: 206, label, .. } if label == "delivered"
    ));
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 206,
            gate: Gate::Rejected,
            payload: Some(p),
            ..
        } if p == "scope mismatch"
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 206,
            new_state: StoryState::Rejected,
            ..
        }
    ));
}

#[tokio::test]
async fn create_story_migrated_path_assigns_number_and_emits_story_created_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        50,
        "seed".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    )]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert(
        "title".into(),
        serde_json::json!("[Bug] live-verify failure"),
    );
    args.insert(
        "user_job".into(),
        serde_json::json!("When pm files, dev needs..."),
    );
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("I would reject if..."),
    );
    args.insert("body".into(), serde_json::json!("author note"));

    let result = handle_create_story(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");
    let response = result.unwrap();
    assert!(
        response.contains("#51"),
        "response should carry the source-assigned number 51 (max+1 over seed 50): {response}"
    );

    let story = store
        .story(51)
        .expect("StoryCreated event applied to store");
    assert_eq!(story.title, "[Bug] live-verify failure");
    assert_eq!(story.state, StoryState::Icebox);
    assert_eq!(
        story.story_type,
        StoryType::Bug,
        "story_type derived from title prefix [Bug]"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1, "exactly one event (StoryCreated) drained");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::StoryCreated {
            number: 51,
            initial_state: StoryState::Icebox,
            story_type: StoryType::Bug,
            ..
        }
    ));
}

#[tokio::test]
async fn deliver_story_migrated_path_uses_store_title_not_tracker() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        207,
        "store-titled-207".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = crate::board::domain::StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let (ws_tx, captured) = connected_ws_sink_with_capture().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(207));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("ran end-to-end"),
    );
    let result = handle_deliver_story(&args, "dev:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let messages = captured.lock().await;
    let signal_text_combined = messages.join("\n");
    assert!(
        signal_text_combined.contains("store-titled-207"),
        "migrated deliver path must use store-side title; expected `store-titled-207` in WS signals: {signal_text_combined}"
    );
    assert!(
        !signal_text_combined.contains("(fake) story #207"),
        "migrated deliver path must NOT fall back to FakeTracker title; FakeTracker text leaked: {signal_text_combined}"
    );
}

#[cfg(test)]
mod classify_for_pass_desk_check_tests {
    use super::*;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    async fn orchestrator_with_seed(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            Arc::clone(&store),
            Arc::clone(&adapter),
        ));
        orch.refresh_all().await.unwrap();
        orch
    }

    fn make_story(number: u64, story_type: StoryType) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("story-{number}"),
            StoryState::Delivered,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        s.story_type = story_type;
        s
    }

    #[tokio::test]
    async fn refreshed_path_returns_feature_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(910, StoryType::Feature)]).await;
        assert_eq!(
            classify_for_pass_desk_check(910, Some(&orch)),
            StoryType::Feature
        );
    }

    #[tokio::test]
    async fn refreshed_path_returns_bug_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(911, StoryType::Bug)]).await;
        assert_eq!(
            classify_for_pass_desk_check(911, Some(&orch)),
            StoryType::Bug
        );
    }

    #[tokio::test]
    async fn refreshed_path_returns_chore_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(912, StoryType::Chore)]).await;
        assert_eq!(
            classify_for_pass_desk_check(912, Some(&orch)),
            StoryType::Chore
        );
    }

    #[tokio::test]
    async fn defaults_to_feature_when_refresh_absent() {
        assert_eq!(classify_for_pass_desk_check(913, None), StoryType::Feature);
    }

    #[tokio::test]
    async fn defaults_to_feature_when_story_not_in_cache() {
        let orch = orchestrator_with_seed(vec![]).await;
        assert_eq!(
            classify_for_pass_desk_check(914, Some(&orch)),
            StoryType::Feature
        );
    }
}

#[tokio::test]
async fn view_board_requires_refresh_after_5l_fallback_retirement() {
    let result = handle_view_board(&serde_json::Map::new(), None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn view_board_buckets_stories_by_story_state() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let stories = vec![
        Story::new_minimal(
            101,
            "icebox-story".into(),
            StoryState::Icebox,
            ts,
            "pm".into(),
        ),
        Story::new_minimal(
            102,
            "ready-story".into(),
            StoryState::Ready,
            ts,
            "pm".into(),
        ),
        Story::new_minimal(
            103,
            "in-progress-story".into(),
            StoryState::InProgress,
            ts,
            "dev".into(),
        ),
        Story::new_minimal(
            104,
            "delivered-story".into(),
            StoryState::Delivered,
            ts,
            "dev".into(),
        ),
        Story::new_minimal(
            105,
            "ready-for-accept-story".into(),
            StoryState::ReadyForAccept,
            ts,
            "designer".into(),
        ),
        Story::new_minimal(
            106,
            "needs-fixes-story".into(),
            StoryState::NeedsFixes,
            ts,
            "designer".into(),
        ),
        Story::new_minimal(
            107,
            "rejected-story".into(),
            StoryState::Rejected,
            ts,
            "ryan".into(),
        ),
        Story::new_minimal(
            108,
            "accepted-story".into(),
            StoryState::Accepted,
            ts,
            "ryan".into(),
        ),
    ];
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();

    for header in [
        "ICEBOX (1)",
        "READY (1)",
        "IN PROGRESS (1)",
        "DELIVERED (1)",
        "READY FOR ACCEPT (1)",
        "NEEDS FIXES (1)",
        "REJECTED (1)",
        "ACCEPTED (1)",
    ] {
        assert!(
            out.contains(header),
            "expected section header {header:?} in output:\n{out}"
        );
    }
    assert!(out.contains("#101 icebox-story"));
    assert!(out.contains("#108 accepted-story"));
}

#[tokio::test]
async fn view_board_hides_is_closed_stories_from_non_accepted_buckets() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut closed_in_progress = Story::new_minimal(
        201,
        "abandoned-mid-flight".into(),
        StoryState::InProgress,
        ts,
        "pm".into(),
    );
    closed_in_progress.is_closed = true;
    let mut accepted_closed = Story::new_minimal(
        202,
        "accepted-closed".into(),
        StoryState::Accepted,
        ts,
        "pm".into(),
    );
    accepted_closed.is_closed = true;
    adapter.seed_stories(vec![closed_in_progress, accepted_closed]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();

    assert!(
        out.contains("IN PROGRESS (0)"),
        "is_closed InProgress story must NOT appear in IN PROGRESS bucket: {out}"
    );
    assert!(
        !out.contains("#201"),
        "story #201 should be hidden across all buckets when is_closed and state != Accepted: {out}"
    );
    assert!(
        out.contains("ACCEPTED (1)") && out.contains("#202 accepted-closed"),
        "is_closed + state=Accepted story remains in ACCEPTED bucket: {out}"
    );
}

#[tokio::test]
async fn close_story_migrated_path_emits_story_closed_event_with_reason() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        301,
        "won't fix story".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(301));
    args.insert(
        "reason".into(),
        serde_json::json!("superseded by #999 — won't ship"),
    );
    let result = handle_close_story(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(301).expect("story exists");
    assert!(story.is_closed, "is_closed flag must be set");
    assert_eq!(
        story.state,
        StoryState::Delivered,
        "state preserved at last domain state — close is action-overlay, not state-transition"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1, "exactly one StoryClosed event must drain");
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::StoryClosed {
            number: 301,
            reason: Some(r),
            ..
        } if r == "superseded by #999 — won't ship"
    ));
}

#[tokio::test]
async fn set_parent_epic_migrated_path_emits_parent_epic_set_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        300,
        "child".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let mut epic = Story::new_minimal(
        457,
        "epic story".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    adapter.seed_stories(vec![child, epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(300));
    args.insert("epic_number".into(), serde_json::json!(457));
    let result = handle_set_parent_epic(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(300).unwrap();
    assert_eq!(story.parent_epic, Some(457));

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::ParentEpicSet {
            number: 300,
            epic_number: 457,
            ..
        }
    ));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_existent_epic() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        310,
        "orphan".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![child]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(310));
    args.insert("epic_number".into(), serde_json::json!(9999));
    let result = handle_set_parent_epic(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_epic_parent() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        311,
        "child".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let mut not_epic = Story::new_minimal(
        458,
        "ordinary feature".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    not_epic.story_type = StoryType::Feature;
    adapter.seed_stories(vec![child, not_epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(311));
    args.insert("epic_number".into(), serde_json::json!(458));
    let result = handle_set_parent_epic(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not an Epic"));
}

#[tokio::test]
async fn unset_parent_epic_migrated_path_emits_parent_epic_unset_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        301,
        "child".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    seeded.parent_epic = Some(457);
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(301));
    let result =
        handle_unset_parent_epic(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(301).unwrap();
    assert_eq!(story.parent_epic, None);

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::ParentEpicUnset { number: 301, .. }
    ));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_pm_role() {
    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(1));
    args.insert("epic_number".into(), serde_json::json!(2));
    let result = handle_set_parent_epic(&args, "dev:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[tokio::test]
async fn set_parent_epic_rejects_self_parent() {
    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(42));
    args.insert("epic_number".into(), serde_json::json!(42));
    let result = handle_set_parent_epic(&args, "pm:claude-code", &ws_tx, None).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("cannot be its own parent epic")
    );
}

#[tokio::test]
async fn create_story_with_parent_epic_emits_story_created_then_parent_epic_set() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut epic = Story::new_minimal(
        500,
        "epic story".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    adapter.seed_stories(vec![epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("title".into(), serde_json::json!("child of epic"));
    args.insert("user_job".into(), serde_json::json!("user job here"));
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("would reject if X"),
    );
    args.insert("parent_epic".into(), serde_json::json!(500));
    let result = handle_create_story(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(
        result.is_ok(),
        "create-with-parent should succeed: {result:?}"
    );
    assert!(result.unwrap().contains("under epic #500"));

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 2, "StoryCreated + ParentEpicSet both drain");
    let pushed = adapter.pushed_events();
    assert!(matches!(pushed[0], BoardEvent::StoryCreated { .. }));
    assert!(matches!(
        pushed[1],
        BoardEvent::ParentEpicSet {
            epic_number: 500,
            ..
        }
    ));
}

#[tokio::test]
async fn create_story_with_non_epic_parent_rejects_before_create() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut not_epic = Story::new_minimal(
        501,
        "not epic".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    not_epic.story_type = StoryType::Feature;
    adapter.seed_stories(vec![not_epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("title".into(), serde_json::json!("orphan"));
    args.insert("user_job".into(), serde_json::json!("user job here"));
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("would reject if X"),
    );
    args.insert("parent_epic".into(), serde_json::json!(501));
    let result = handle_create_story(&args, "pm:claude-code", &ws_tx, Some(&orchestrator)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not an Epic"));
    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 0, "no events emitted on parent-validation reject");
}

#[tokio::test]
async fn view_board_renders_epic_summary_section_with_child_progress() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        700,
        "umbrella story".into(),
        StoryState::Ready,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    let mut child_a =
        Story::new_minimal(701, "child a".into(), StoryState::Accepted, ts, "pm".into());
    child_a.parent_epic = Some(700);
    child_a.is_closed = true;
    let mut child_b = Story::new_minimal(
        702,
        "child b".into(),
        StoryState::Delivered,
        ts,
        "pm".into(),
    );
    child_b.parent_epic = Some(700);
    let mut child_c = Story::new_minimal(
        703,
        "child c".into(),
        StoryState::InProgress,
        ts,
        "pm".into(),
    );
    child_c.parent_epic = Some(700);
    adapter.seed_stories(vec![epic, child_a, child_b, child_c]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("EPICS (1)"),
        "expected EPICS section header in:\n{out}"
    );
    assert!(
        out.contains(
            "#700 [in-progress] umbrella story (3 children: 1 accepted, 1 delivered, 1 in-progress)"
        ),
        "expected Epic-summary line with derived state + progress in:\n{out}"
    );
    assert!(
        out.contains("↳ #702 child b (epic #700)"),
        "expected child glyph + epic-membership tag in:\n{out}"
    );
    assert!(
        out.contains("↳ #703 child c (epic #700)"),
        "expected in-progress child glyph in:\n{out}"
    );
}

#[tokio::test]
async fn view_board_omits_epic_section_when_no_epics_present() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let plain = Story::new_minimal(
        800,
        "plain feature".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![plain]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        !out.contains("EPICS"),
        "EPICS section must NOT appear when no Epics: {out}"
    );
}

#[tokio::test]
async fn view_board_filters_epic_stories_out_of_state_lanes() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        710,
        "another umbrella".into(),
        StoryState::Ready,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    adapter.seed_stories(vec![epic]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("EPICS (1)") && out.contains("#710"),
        "Epic must appear in EPICS section: {out}"
    );
    assert!(
        !out.contains("  #710 another umbrella")
            || out
                .lines()
                .filter(|line| line.contains("another umbrella"))
                .count()
                == 1,
        "Epic story must appear once (in EPICS only), not in state-lane: {out}"
    );
    let ready_section = out
        .split("READY (")
        .nth(1)
        .unwrap_or("")
        .split("\n\n")
        .next()
        .unwrap_or("");
    assert!(
        !ready_section.contains("710"),
        "Epic must be filtered OUT of READY lane: ready_section={ready_section}"
    );
}

#[tokio::test]
async fn view_board_epic_derived_state_reflects_all_delivered_children() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        720,
        "delivered umbrella".into(),
        StoryState::Ready,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    let mut a = Story::new_minimal(721, "a".into(), StoryState::Delivered, ts, "pm".into());
    a.parent_epic = Some(720);
    let mut b = Story::new_minimal(722, "b".into(), StoryState::Delivered, ts, "pm".into());
    b.parent_epic = Some(720);
    adapter.seed_stories(vec![epic, a, b]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("#720 [delivered] delivered umbrella"),
        "Epic with all-delivered children should derive 'delivered' state: {out}"
    );
}

#[tokio::test]
async fn view_board_epic_explicit_accepted_overrides_derivation() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        730,
        "owner-accepted umbrella".into(),
        StoryState::Accepted,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    let mut a = Story::new_minimal(731, "a".into(), StoryState::InProgress, ts, "pm".into());
    a.parent_epic = Some(730);
    adapter.seed_stories(vec![epic, a]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("#730 [accepted] owner-accepted umbrella"),
        "Epic with explicit Accepted state should NOT derive from in-progress child: {out}"
    );
}

#[test]
fn parse_acceptance_test_path_extracts_section_value() {
    let body = "## User job\n\nfoo\n\n## Observation falsifier\n\nbar\n\n## Acceptance test path\n\napplications/electron/tests/foo.spec.ts\n\n---\n\nauthor body";
    assert_eq!(
        parse_acceptance_test_path(body),
        Some("applications/electron/tests/foo.spec.ts".to_string())
    );
}

#[test]
fn parse_acceptance_test_path_returns_none_when_section_absent() {
    let body = "## User job\n\nfoo\n\n## Observation falsifier\n\nbar\n\n---\n\nauthor body";
    assert_eq!(parse_acceptance_test_path(body), None);
}

#[test]
fn parse_acceptance_test_path_returns_none_when_section_empty() {
    let body = "## User job\n\nfoo\n\n## Acceptance test path\n\n   \n\n---\n\n";
    assert_eq!(parse_acceptance_test_path(body), None);
}

#[tokio::test]
async fn start_story_blocks_feature_without_acceptance_test_path() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        881,
        "[Feature] needs path".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Feature;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(881));
    args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
    args.insert("falsifier_reframing".into(), serde_json::json!("f"));
    let result = handle_start_story(&args, "dev:claude-code", &ws_tx, Some(&orch)).await;
    assert!(
        result.is_err(),
        "Feature without path must block: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(err.contains("acceptance_test_path required"), "{err}");
}

#[tokio::test]
async fn start_story_blocks_when_acceptance_test_path_does_not_exist() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        882,
        "[Feature] missing-test-file".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Feature;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(882));
    args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
    args.insert("falsifier_reframing".into(), serde_json::json!("f"));
    args.insert(
        "acceptance_test_path".into(),
        serde_json::json!("does/not/exist.spec.ts"),
    );
    let result = handle_start_story(&args, "dev:claude-code", &ws_tx, Some(&orch)).await;
    assert!(result.is_err(), "missing path must block: {result:?}");
    assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn start_story_passes_chore_without_acceptance_test_path() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        883,
        "[Chore] no test required".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(883));
    args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
    args.insert("falsifier_reframing".into(), serde_json::json!("f"));
    let result = handle_start_story(&args, "dev:claude-code", &ws_tx, Some(&orch)).await;
    assert!(result.is_ok(), "Chore exempt: {result:?}");
}

#[tokio::test]
async fn deliver_story_blocks_feature_without_acceptance_test_path() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        884,
        "[Feature] no path".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Feature;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(884));
    args.insert("verified_observation".into(), serde_json::json!("drove it"));
    let result = handle_deliver_story(&args, "dev:claude-code", &ws_tx, Some(&orch)).await;
    assert!(
        result.is_err(),
        "Feature without path must block at deliver: {result:?}"
    );
    assert!(result.unwrap_err().contains("acceptance_test_path missing"));
}

#[tokio::test]
async fn reject_after_accept_emits_tag_remove_accepted_and_story_reopened() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        469,
        "[Bug] rework-after-accept".into(),
        StoryState::Accepted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "ryan".into(),
    );
    seeded.story_type = StoryType::Bug;
    seeded.is_closed = true;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(469));
    args.insert("reason".into(), serde_json::json!("rework needed"));
    let result = handle_reject_story(&args, "ryan:human", &ws_tx, Some(&orch)).await;
    assert!(result.is_ok(), "reject should succeed: {result:?}");

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();

    let tag_removed_accepted = pushed.iter().any(|e| {
        matches!(
            e,
            BoardEvent::TagRemoved { number: 469, label, .. } if label == "accepted"
        )
    });
    assert!(
        tag_removed_accepted,
        "reject_story on previously-Accepted must emit TagRemoved(accepted): {pushed:?}"
    );

    let story_reopened = pushed
        .iter()
        .any(|e| matches!(e, BoardEvent::StoryReopened { number: 469, .. }));
    assert!(
        story_reopened,
        "reject_story on previously-Accepted must emit StoryReopened: {pushed:?}"
    );

    let story = store.story(469).unwrap();
    assert!(
        !story.is_closed,
        "store must reflect re-opened state post-reject"
    );
    assert_eq!(story.state, StoryState::Rejected);
}

#[tokio::test]
async fn reject_on_non_accepted_skips_tag_remove_accepted_and_story_reopened() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        470,
        "[Bug] reject-from-delivered".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "ryan".into(),
    );
    seeded.story_type = StoryType::Bug;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(470));
    args.insert("reason".into(), serde_json::json!("not done"));
    handle_reject_story(&args, "ryan:human", &ws_tx, Some(&orch))
        .await
        .unwrap();

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    let any_accepted_remove = pushed.iter().any(|e| {
        matches!(
            e,
            BoardEvent::TagRemoved { number: 470, label, .. } if label == "accepted"
        )
    });
    assert!(
        !any_accepted_remove,
        "reject on non-Accepted must NOT emit TagRemoved(accepted): {pushed:?}"
    );
    let any_reopen = pushed
        .iter()
        .any(|e| matches!(e, BoardEvent::StoryReopened { number: 470, .. }));
    assert!(
        !any_reopen,
        "reject on non-Accepted must NOT emit StoryReopened: {pushed:?}"
    );
}

#[tokio::test]
async fn deliver_story_propagates_drain_failure_at_label_remove() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        450,
        "[Chore] label-leak repro".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    adapter.set_push_failures(true);

    let ws_tx = connected_ws_sink_for_test().await;
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(450));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("drove the label-leak failure-injection scenario"),
    );
    let result = handle_deliver_story(&args, "dev:claude-code", &ws_tx, Some(&orch)).await;
    assert!(
        result.is_err(),
        "deliver_story must surface drain failure load-bearing: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("drain after deliver failed"),
        "error must name the deliver-drain stage: {err}"
    );
}
