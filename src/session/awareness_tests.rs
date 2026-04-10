#![allow(clippy::unwrap_used)]

use super::awareness::agents::AgentTracker;
use super::awareness::experience;
use crate::session::message::{Message, MessageType};
use chrono::Utc;

fn system_join(name: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{name} joined"),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
        addressee: None,
        explicit_privacy: false,
    }
}

fn system_leave(name: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{name} left"),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
        addressee: None,
        explicit_privacy: false,
    }
}

#[test]
fn track_agents_adds_on_join() {
    let mut tracker = AgentTracker::new("ryan");
    tracker.on_message(&system_join("dev:claude-code"));
    assert_eq!(tracker.connected, vec!["dev"]);
}

#[test]
fn track_agents_removes_on_leave() {
    let mut tracker = AgentTracker::new("ryan");
    tracker.on_message(&system_join("dev:claude-code"));
    tracker.on_message(&system_join("pm:claude-code"));
    tracker.on_message(&system_leave("dev:claude-code"));
    assert_eq!(tracker.connected, vec!["pm"]);
}

#[test]
fn track_agents_ignores_owner() {
    let mut tracker = AgentTracker::new("ryan");
    tracker.on_message(&system_join("ryan:human"));
    assert!(
        tracker.connected.is_empty(),
        "owner should not be tracked as agent"
    );
}

#[test]
fn track_agents_no_duplicates() {
    let mut tracker = AgentTracker::new("ryan");
    tracker.on_message(&system_join("dev:claude-code"));
    tracker.on_message(&system_join("dev:claude-code"));
    assert_eq!(tracker.connected.len(), 1);
}

// ---------- #159: classifier decision logging ----------
//
// These tests drive the ExperienceDetector with the classify-log flag
// set directly via a test-only accessor, rather than mutating
// process-global env vars (which would require unsafe). The actual
// file-sink behavior is tested end-to-end by the CLI integration tests
// where BPMUX_LOG can be set once at process startup.

fn chat_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
        addressee: None,
        explicit_privacy: false,
    }
}

#[test]
fn classify_log_disabled_by_default_suppresses_log() {
    // Default state (flag false) means `last_classify_log` should
    // remain None even after messages are classified.
    let mut detector = experience::ExperienceDetector::new(&[], "ryan");
    // Explicit: disabled
    detector.set_classify_log_for_test(false);
    let msg = chat_msg("dev:claude-code", "this is a decision to ship");
    detector.on_message(&msg, "dev");
    assert_eq!(
        detector.classify_log_count(),
        0,
        "classify-log should not fire when disabled"
    );
}

#[test]
fn classify_log_enabled_fires_on_first_message() {
    let mut detector = experience::ExperienceDetector::new(&[], "ryan");
    detector.set_classify_log_for_test(true);
    let msg = chat_msg("dev:claude-code", "this is a test classification message");
    detector.on_message(&msg, "dev");
    assert_eq!(
        detector.classify_log_count(),
        1,
        "classify-log should fire once when enabled"
    );
}

#[test]
fn classify_log_rate_limits_rapid_messages() {
    // The rate limit is 1/sec. Feeding 10 messages in quick succession
    // should leave `last_classify_log` set (proves the first fired),
    // but a rate-limit-aware counter that tracked fires-per-interval
    // would reveal more detail. Since `last_classify_log` is Option,
    // we can't distinguish "fired once, rate-limited 9" from "fired 10
    // times" at this level. The guarantee we CAN test here is that
    // the code path doesn't panic or double-count on rapid input.
    let mut detector = experience::ExperienceDetector::new(&[], "ryan");
    detector.set_classify_log_for_test(true);
    for i in 0..10 {
        let msg = chat_msg("dev:claude-code", &format!("message {i}"));
        detector.on_message(&msg, "dev");
    }
    // The fact that we got here without panic is the invariant.
    assert_eq!(detector.classify_log_count(), 1);
}

#[test]
fn classify_log_does_not_affect_classification_behavior() {
    // The classify-log is purely observational. Enabling it must not
    // change which messages are classified as experiences. Drive the
    // same correction message through two detectors — one with the
    // flag off, one with it on — and verify the pending-experience
    // state is identical.
    let msg = chat_msg("ryan:human", "don't do that, it breaks the build");

    let mut detector_a = experience::ExperienceDetector::new(&[], "ryan");
    detector_a.set_classify_log_for_test(false);
    detector_a.on_message(&msg, "ryan");
    let has_pending_without = detector_a.has_pending();

    let mut detector_b = experience::ExperienceDetector::new(&[], "ryan");
    detector_b.set_classify_log_for_test(true);
    detector_b.on_message(&msg, "ryan");
    let has_pending_with = detector_b.has_pending();

    assert_eq!(
        has_pending_without, has_pending_with,
        "classify-log must not affect classification behavior"
    );
}
