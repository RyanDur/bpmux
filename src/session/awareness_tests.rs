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
        multi_depth: None,
        topic: None,
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
        multi_depth: None,
        topic: None,
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
        multi_depth: None,
        topic: None,
    }
}

#[test]
fn classify_log_disabled_by_default_suppresses_log() {
    // Default state (flag false) means `last_classify_log` should
    // remain None even after messages are classified.
    let mut detector = experience::ExperienceDetector::with_classify_log(&[], "ryan", false);
    let msg = chat_msg("dev:claude-code", "this is a decision to ship");
    detector.on_message(&msg, "dev");
    assert!(
        !detector.has_classify_log(),
        "classify-log should not fire when disabled"
    );
}

#[test]
fn classify_log_enabled_fires_on_first_message() {
    let mut detector = experience::ExperienceDetector::with_classify_log(&[], "ryan", true);
    let msg = chat_msg("dev:claude-code", "this is a test classification message");
    detector.on_message(&msg, "dev");
    assert!(
        detector.has_classify_log(),
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
    let mut detector = experience::ExperienceDetector::with_classify_log(&[], "ryan", true);
    for i in 0..10 {
        let msg = chat_msg("dev:claude-code", &format!("message {i}"));
        detector.on_message(&msg, "dev");
    }
    assert!(detector.has_classify_log());
}

#[test]
fn classify_log_does_not_affect_classification_behavior() {
    // The classify-log is purely observational. Enabling it must not
    // change which messages are classified as experiences. Drive the
    // same correction message through two detectors — one with the
    // flag off, one with it on — and verify the pending-experience
    // state is identical.
    let msg = chat_msg("ryan:human", "don't do that, it breaks the build");

    let mut detector_a = experience::ExperienceDetector::with_classify_log(&[], "ryan", false);
    detector_a.on_message(&msg, "ryan");
    let has_pending_without = detector_a.has_pending();

    let mut detector_b = experience::ExperienceDetector::with_classify_log(&[], "ryan", true);
    detector_b.on_message(&msg, "ryan");
    let has_pending_with = detector_b.has_pending();

    assert_eq!(
        has_pending_without, has_pending_with,
        "classify-log must not affect classification behavior"
    );
}

// ---------- #163: nudge-hygiene tunings ----------

use super::Session;
use super::awareness::handoff::HandoffTracker;
use super::bayes::BayesModel;
use super::classifier::MessageClass;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;

fn make_session() -> (Arc<Session>, broadcast::Receiver<Message>, TempDir) {
    let dir = TempDir::new().unwrap();
    let (tx, rx) = broadcast::channel(64);
    let session = Arc::new(Session::new(
        dir.path().to_path_buf(),
        "ryan".to_string(),
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
    // Tuning 1: per-recipient filtering. When flush fires with
    // experiences from multiple senders, each recipient should see a
    // nudge containing only messages from OTHER senders, not their own.
    let (session, mut rx, _dir) = make_session();

    // Connected agents: dev, designer, pm.
    let mut agents = AgentTracker::new("ryan");
    agents.on_message(&system_join("dev:claude-code"));
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("pm:claude-code"));

    let mut detector = experience::ExperienceDetector::new(&[], "ryan");
    // Feed a correction from pm (classifies as Experience(Correction)).
    detector.on_message(
        &chat_msg(
            "pm:claude-code",
            "don't do that, we already decided against it",
        ),
        "pm",
    );
    // Feed an admission from dev (classifies as Experience(Correction) via admission patterns).
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

    // Drain the session broadcast channel — there should be one nudge
    // per connected agent who has at least one OTHER agent's experience
    // to be nudged about.
    let nudges = drain_broadcasts(&mut rx);
    assert!(!nudges.is_empty(), "expected at least one nudge");

    // Find each per-recipient nudge.
    let find_to = |role: &str| {
        nudges
            .iter()
            .find(|m| m.to.as_deref() == Some(&[role.to_string()][..]))
    };

    let pm_nudge = find_to("pm").expect("pm nudge");
    let dev_nudge = find_to("dev").expect("dev nudge");
    let designer_nudge = find_to("designer").expect("designer nudge");

    // pm's nudge should NOT contain "don't do" (pm's own message),
    // but should contain "my bad" (dev's message).
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

    // dev's nudge should NOT contain "my bad" (dev's own), but should
    // contain "don't do" (pm's).
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

    // designer wasn't a sender — their nudge should contain BOTH.
    assert!(designer_nudge.text.contains("don't do"));
    assert!(designer_nudge.text.contains("my bad"));
}

#[test]
fn flush_skips_recipients_with_only_own_experiences() {
    // Edge case of tuning 1: if the only pending experience came from
    // the recipient themselves, no nudge fires for that recipient at
    // all. Otherwise we'd send an empty nudge or a "0 candidates" nudge.
    let (session, mut rx, _dir) = make_session();
    let mut agents = AgentTracker::new("ryan");
    agents.on_message(&system_join("pm:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    let mut detector = experience::ExperienceDetector::new(&[], "ryan");
    // Only one experience, from pm.
    detector.on_message(
        &chat_msg(
            "pm:claude-code",
            "don't do that in that way for this reason",
        ),
        "pm",
    );
    detector.flush(&session, &agents);

    let nudges = drain_broadcasts(&mut rx);
    // pm should NOT receive a nudge (their only pending is their own).
    // dev SHOULD receive one (pm's experience is a candidate for them).
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
    // Tuning 2: the reworded nudge text must not self-trigger the
    // classifier. The classifier's sender-check early-return catches
    // bpmux messages, but this is defense in depth against any path
    // that might not check sender.
    //
    // Synthesize a nudge-shaped message as if it were from a
    // non-bpmux sender, so the sender guard doesn't mask the test.
    let nudge_msg = chat_msg(
        "somebody:else",
        "📝 3 candidate moments from recent chat. Pointers, not content to copy — reflect through your own role's lens and save your own framing if any apply:\n  1. [from dev] test preview\nCall save_lesson(topic, lesson) only for durable rules in your own words.",
    );
    let class = super::classifier::classify(&nudge_msg, "ryan");
    assert!(
        !class.is_experience(),
        "reworded nudge text must not classify as experience, got {class:?}"
    );
}

#[test]
fn bayes_classify_returns_routine_for_bpmux_sender() {
    // Tuning 2 root cause: BayesModel::classify was missing the
    // bpmux sender guard that the pattern classifier had. With the fix,
    // the trained Bayes path returns Routine for bpmux messages.
    let mut model = BayesModel::default();
    // Train with enough diverse data that is_trained() returns true.
    let training = vec![
        chat_msg("ryan:human", "don't do that, it breaks the build"),
        chat_msg("dev:claude-code", "my bad, fixed"),
        chat_msg(
            "pm:claude-code",
            "some routine coordination about stories today",
        ),
    ];
    model.train(&training, "ryan");
    assert!(model.is_trained());

    // A bpmux-sourced message that LOOKS like it could match experience
    // patterns must classify as Routine via the sender guard.
    let bpmux_msg = chat_msg(
        "bpmux",
        "3 candidate moments from recent chat — don't do that, my bad",
    );
    assert_eq!(model.classify(&bpmux_msg, "ryan"), MessageClass::Routine);
}

#[test]
fn bayes_classify_returns_routine_for_non_chat_type() {
    // Same guard class — non-chat messages (activity, system) should
    // classify as Routine at the Bayes layer too, matching the pattern
    // classifier's behavior.
    let mut model = BayesModel::default();
    model.train(
        &[
            chat_msg("ryan:human", "don't do that"),
            chat_msg("dev:claude-code", "my bad"),
            chat_msg("pm:claude-code", "coordination"),
        ],
        "ryan",
    );
    assert!(model.is_trained());

    let sys_msg = Message {
        from: "dev:claude-code".to_string(),
        text: "dev joined".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    };
    assert_eq!(model.classify(&sys_msg, "ryan"), MessageClass::Routine);
}

#[test]
fn handoff_nudge_suppressed_when_recipient_already_responded() {
    // Tuning 3: conversation-state check. If the handoff recipient has
    // posted any message to the session log AFTER the handoff, the
    // timeout nudge is suppressed — even if the in-memory pending
    // state didn't catch the clear due to timing.
    let (session, mut rx, _dir) = make_session();

    let mut agents = AgentTracker::new("ryan");
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    // Dev issues a handoff to designer.
    let handoff_msg = chat_msg(
        "dev:claude-code",
        "designer, please review the new spec when you get a chance",
    );
    let _ = session.broadcast(&handoff_msg);

    let mut tracker = HandoffTracker::new();
    tracker.on_message(&handoff_msg, "dev", &agents, "ryan");
    assert!(
        tracker.is_pending(),
        "precondition: handoff should be detected and pending"
    );

    // Simulate that designer's ack message bypassed the in-memory
    // tracker (e.g. tokio::select race) — but IS persisted in the
    // session log. We have to use a higher timestamp for the ack to be
    // "after" the handoff.
    let ack_msg = Message {
        from: "designer:claude-code".to_string(),
        text: "on it, pulling the spec now".to_string(),
        ts: handoff_msg.ts + chrono::Duration::seconds(5),
        message_type: MessageType::Chat,
        to: None,
        addressee: None,
        explicit_privacy: false,
        multi_depth: None,
        topic: None,
    };
    let _ = session.broadcast(&ack_msg);

    // Drain the two pre-existing broadcasts so only the potential
    // nudge is left in the channel after we call timeout_nudge.
    let _ = drain_broadcasts(&mut rx);

    // Fire the timeout nudge. With tuning 3, the session-log check
    // should find the ack and suppress the nudge.
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
    // Complement to the above: if the recipient has NOT responded, the
    // timeout nudge should still fire (the defense doesn't swallow
    // real failures).
    let (session, mut rx, _dir) = make_session();
    let mut agents = AgentTracker::new("ryan");
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    let handoff_msg = chat_msg(
        "dev:claude-code",
        "designer, please desk check this change when you have time",
    );
    let _ = session.broadcast(&handoff_msg);

    let mut tracker = HandoffTracker::new();
    tracker.on_message(&handoff_msg, "dev", &agents, "ryan");
    assert!(tracker.is_pending());

    // NO ack message. Designer has been silent.
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
