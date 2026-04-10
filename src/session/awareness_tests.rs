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
