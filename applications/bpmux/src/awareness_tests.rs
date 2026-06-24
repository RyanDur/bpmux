#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::awareness::agents::AgentTracker;
use super::awareness::experience;
use crate::message::{Message, MessageType};
use chrono::Utc;

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

use super::awareness::handoff::HandoffTracker;
use crate::comprehension::bayes::BayesModel;
use crate::comprehension::classifier::MessageClass;
use crate::session::Session;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;

fn make_session() -> (Arc<Session>, broadcast::Receiver<Message>, TempDir) {
    let dir = TempDir::new().unwrap();
    let (tx, rx) = broadcast::channel(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "owner".to_string(),
        tx,
    ));
    (session, rx, dir)
}

fn drain_broadcasts(rx: &mut broadcast::Receiver<Message>) -> Vec<Message> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

#[test]
fn flush_excludes_recipient_own_messages() {
    let (session, mut rx, _dir) = make_session();

    let mut agents = AgentTracker::new("owner");
    agents.on_message(&system_join("dev:claude-code"));
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("pm:claude-code"));

    let mut detector = experience::ExperienceDetector::new(&[], "owner");
    detector.on_message(
        &chat_msg(
            "pm:claude-code",
            "don't do that, we already decided against it",
        ),
        "pm",
    );
    detector.on_message(
        &chat_msg(
            "dev:claude-code",
            "my bad, i flipped my position again on that",
        ),
        "dev",
    );

    assert!(
        detector.has_pending(),
        "precondition: detector should have pending experiences after feeding corrections"
    );

    detector.flush(&session, &agents);

    let nudges = drain_broadcasts(&mut rx);
    assert!(!nudges.is_empty(), "expected at least one nudge");

    let find_to = |role: &str| {
        nudges
            .iter()
            .find(|m| m.to.as_deref() == Some(&[role.to_string()][..]))
    };

    let pm_nudge = find_to("pm").expect("pm nudge");
    let dev_nudge = find_to("dev").expect("dev nudge");
    let designer_nudge = find_to("designer").expect("designer nudge");

    assert!(
        !pm_nudge.text.contains("don't do"),
        "pm's nudge leaked pm's own message: {}",
        pm_nudge.text
    );
    assert!(
        pm_nudge.text.contains("my bad"),
        "pm's nudge missing dev's message: {}",
        pm_nudge.text
    );

    assert!(
        !dev_nudge.text.contains("my bad"),
        "dev's nudge leaked dev's own message: {}",
        dev_nudge.text
    );
    assert!(
        dev_nudge.text.contains("don't do"),
        "dev's nudge missing pm's message: {}",
        dev_nudge.text
    );

    assert!(designer_nudge.text.contains("don't do"));
    assert!(designer_nudge.text.contains("my bad"));
}

#[test]
fn flush_skips_recipients_with_only_own_experiences() {
    let (session, mut rx, _dir) = make_session();
    let mut agents = AgentTracker::new("owner");
    agents.on_message(&system_join("pm:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    let mut detector = experience::ExperienceDetector::new(&[], "owner");
    detector.on_message(
        &chat_msg(
            "pm:claude-code",
            "don't do that in that way for this reason",
        ),
        "pm",
    );
    detector.flush(&session, &agents);

    let nudges = drain_broadcasts(&mut rx);
    let pm_nudge = nudges
        .iter()
        .find(|m| m.to.as_deref() == Some(&["pm".to_string()][..]));
    assert!(
        pm_nudge.is_none(),
        "pm should not receive a nudge when only their own messages are pending"
    );
    let dev_nudge = nudges
        .iter()
        .find(|m| m.to.as_deref() == Some(&["dev".to_string()][..]));
    assert!(dev_nudge.is_some(), "dev should still receive the nudge");
}

#[test]
fn nudge_text_does_not_classify_as_experience() {
    let nudge_msg = chat_msg(
        "somebody:else",
        "📝 3 candidate moments from recent chat. Pointers, not content to copy — reflect through your own role's lens and save your own framing if any apply:\n  1. [from dev] test preview\nCall save_lesson(topic, lesson) only for durable rules in your own words.",
    );
    let class = crate::comprehension::classifier::classify(&nudge_msg, "owner");
    assert!(
        !class.is_experience(),
        "reworded nudge text must not classify as experience, got {class:?}"
    );
}

#[test]
fn bayes_classify_returns_routine_for_bpmux_sender() {
    let mut model = BayesModel::default();
    let training = vec![
        chat_msg("owner:human", "don't do that, it breaks the build"),
        chat_msg("dev:claude-code", "my bad, fixed"),
        chat_msg(
            "pm:claude-code",
            "some routine coordination about stories today",
        ),
    ];
    model.train(&training, "owner");
    assert!(model.is_trained());

    let bpmux_msg = chat_msg(
        "bpmux",
        "3 candidate moments from recent chat — don't do that, my bad",
    );
    assert_eq!(model.classify(&bpmux_msg, "owner"), MessageClass::Routine);
}

#[test]
fn bayes_classify_returns_routine_for_non_chat_type() {
    let mut model = BayesModel::default();
    model.train(
        &[
            chat_msg("owner:human", "don't do that"),
            chat_msg("dev:claude-code", "my bad"),
            chat_msg("pm:claude-code", "coordination"),
        ],
        "owner",
    );
    assert!(model.is_trained());

    let sys_msg = Message {
        from: "dev:claude-code".to_string(),
        text: "dev joined".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    };
    assert_eq!(model.classify(&sys_msg, "owner"), MessageClass::Routine);
}

#[test]
fn handoff_nudge_suppressed_when_recipient_already_responded() {
    let (session, mut rx, _dir) = make_session();

    let mut agents = AgentTracker::new("owner");
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    let handoff_msg = chat_msg(
        "dev:claude-code",
        "designer, please review the new spec when you get a chance",
    );
    let _ = session.broadcast(&handoff_msg);

    let mut tracker = HandoffTracker::new();
    tracker.on_message(&handoff_msg, "dev", &agents, "owner");
    assert!(
        tracker.is_pending(),
        "precondition: handoff should be detected and pending"
    );

    let ack_msg = Message {
        from: "designer:claude-code".to_string(),
        text: "on it, pulling the spec now".to_string(),
        ts: handoff_msg.ts + chrono::Duration::seconds(5),
        ..Message::default()
    };
    let _ = session.broadcast(&ack_msg);

    let _ = drain_broadcasts(&mut rx);

    tracker.timeout_nudge(&session);

    let broadcasts_after = drain_broadcasts(&mut rx);
    let suppressed = !broadcasts_after
        .iter()
        .any(|m| m.from == "bpmux" && m.text.contains("hasn't acknowledged the handoff"));
    assert!(
        suppressed,
        "handoff nudge should be suppressed when recipient already responded; got {broadcasts_after:?}"
    );
}

#[test]
fn handoff_nudge_fires_when_recipient_silent() {
    let (session, mut rx, _dir) = make_session();
    let mut agents = AgentTracker::new("owner");
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    let handoff_msg = chat_msg(
        "dev:claude-code",
        "designer, please desk check this change when you have time",
    );
    let _ = session.broadcast(&handoff_msg);

    let mut tracker = HandoffTracker::new();
    tracker.on_message(&handoff_msg, "dev", &agents, "owner");
    assert!(tracker.is_pending());

    let _ = drain_broadcasts(&mut rx);
    tracker.timeout_nudge(&session);

    let broadcasts_after = drain_broadcasts(&mut rx);
    let fired = broadcasts_after
        .iter()
        .any(|m| m.from == "bpmux" && m.text.contains("hasn't acknowledged the handoff"));
    assert!(
        fired,
        "handoff nudge should fire when recipient truly hasn't responded"
    );
}
