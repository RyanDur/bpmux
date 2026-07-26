#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use awareness::agents::AgentTracker;
use awareness::experience;
use awareness::handoff::HandoffTracker;
use chrono::Utc;
use comprehension::bayes::BayesModel;
use comprehension::classifier::MessageClass;
use message::{Message, MessageType};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::broadcast;

use hub::Hub;

fn system_join(name: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{name} joined"),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    }
}

fn chat_msg(from: &str, text: &str) -> Message {
    Message {
        from: from.to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        ..Message::default()
    }
}

fn make_hub() -> (Arc<Hub>, broadcast::Receiver<Message>, TempDir) {
    let dir = TempDir::new().unwrap();
    let (tx, rx) = broadcast::channel(64);
    let hub = Arc::new(Hub::new(dir.path().to_path_buf(), "owner".to_string(), tx));
    (hub, rx, dir)
}

fn host_over(hub: &Arc<Hub>) -> crate::awareness_host::HubAwarenessHost {
    let stores = storage::open(&hub.dir);
    crate::awareness_host::HubAwarenessHost {
        hub: hub.clone(),
        topics: stores.topics,
        lessons: stores.lessons,
    }
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
    let (hub, mut rx, _dir) = make_hub();

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

    detector.flush(&host_over(&hub), &agents);

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
    let (hub, mut rx, _dir) = make_hub();
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
    detector.flush(&host_over(&hub), &agents);

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
    let class = comprehension::classifier::classify(&nudge_msg, "owner");
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
    let (hub, mut rx, _dir) = make_hub();

    let mut agents = AgentTracker::new("owner");
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    let handoff_msg = chat_msg(
        "dev:claude-code",
        "designer, please review the new spec when you get a chance",
    );
    let _ = hub.broadcast(&handoff_msg);

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
    let _ = hub.broadcast(&ack_msg);

    let _ = drain_broadcasts(&mut rx);

    tracker.timeout_nudge(&host_over(&hub));

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
    let (hub, mut rx, _dir) = make_hub();
    let mut agents = AgentTracker::new("owner");
    agents.on_message(&system_join("designer:claude-code"));
    agents.on_message(&system_join("dev:claude-code"));

    let handoff_msg = chat_msg(
        "dev:claude-code",
        "designer, please desk check this change when you have time",
    );
    let _ = hub.broadcast(&handoff_msg);

    let mut tracker = HandoffTracker::new();
    tracker.on_message(&handoff_msg, "dev", &agents, "owner");
    assert!(tracker.is_pending());

    let _ = drain_broadcasts(&mut rx);
    tracker.timeout_nudge(&host_over(&hub));

    let broadcasts_after = drain_broadcasts(&mut rx);
    let fired = broadcasts_after
        .iter()
        .any(|m| m.from == "bpmux" && m.text.contains("hasn't acknowledged the handoff"));
    assert!(
        fired,
        "handoff nudge should fire when recipient truly hasn't responded"
    );
}
