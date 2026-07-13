#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::agents::AgentTracker;
use crate::experience;
use chrono::Utc;
use message::{Message, MessageType};

fn system_join(name: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{name} joined"),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    }
}

fn system_leave(name: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{name} left"),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    }
}

#[test]
fn track_agents_adds_on_join() {
    let mut tracker = AgentTracker::new("owner");
    tracker.on_message(&system_join("dev:claude-code"));
    assert_eq!(tracker.connected, vec!["dev"]);
}

#[test]
fn track_agents_removes_on_leave() {
    let mut tracker = AgentTracker::new("owner");
    tracker.on_message(&system_join("dev:claude-code"));
    tracker.on_message(&system_join("pm:claude-code"));
    tracker.on_message(&system_leave("dev:claude-code"));
    assert_eq!(tracker.connected, vec!["pm"]);
}

#[test]
fn track_agents_ignores_owner() {
    let mut tracker = AgentTracker::new("owner");
    tracker.on_message(&system_join("owner:human"));
    assert!(
        tracker.connected.is_empty(),
        "owner should not be tracked as agent"
    );
}

#[test]
fn track_agents_no_duplicates() {
    let mut tracker = AgentTracker::new("owner");
    tracker.on_message(&system_join("dev:claude-code"));
    tracker.on_message(&system_join("dev:claude-code"));
    assert_eq!(tracker.connected.len(), 1);
}

fn chat_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        ..Message::default()
    }
}

#[test]
fn classify_log_disabled_by_default_suppresses_log() {
    let mut detector = experience::ExperienceDetector::with_classify_log(&[], "owner", false);
    let msg = chat_msg("dev:claude-code", "this is a decision to ship");
    detector.on_message(&msg, "dev");
    assert!(
        !detector.has_classify_log(),
        "classify-log should not fire when disabled"
    );
}

#[test]
fn classify_log_enabled_fires_on_first_message() {
    let mut detector = experience::ExperienceDetector::with_classify_log(&[], "owner", true);
    let msg = chat_msg("dev:claude-code", "this is a test classification message");
    detector.on_message(&msg, "dev");
    assert!(
        detector.has_classify_log(),
        "classify-log should fire once when enabled"
    );
}

#[test]
fn classify_log_rate_limits_rapid_messages() {
    let mut detector = experience::ExperienceDetector::with_classify_log(&[], "owner", true);
    for i in 0..10 {
        let msg = chat_msg("dev:claude-code", &format!("message {i}"));
        detector.on_message(&msg, "dev");
    }
    assert!(detector.has_classify_log());
}

#[test]
fn classify_log_does_not_affect_classification_behavior() {
    let msg = chat_msg("owner:human", "don't do that, it breaks the build");

    let mut detector_a = experience::ExperienceDetector::with_classify_log(&[], "owner", false);
    detector_a.on_message(&msg, "owner");
    let has_pending_without = detector_a.has_pending();

    let mut detector_b = experience::ExperienceDetector::with_classify_log(&[], "owner", true);
    detector_b.on_message(&msg, "owner");
    let has_pending_with = detector_b.has_pending();

    assert_eq!(
        has_pending_without, has_pending_with,
        "classify-log must not affect classification behavior"
    );
}
