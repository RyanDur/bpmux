#![allow(clippy::unwrap_used)]

use super::nudge::*;
use crate::session::message::{Message, MessageType};
use chrono::Utc;

fn owner_msg(text: &str) -> Message {
    Message {
        from: "ryan:human".to_string(),
        text: text.to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
    }
}

fn system_join(name: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{name} joined"),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
    }
}

fn system_leave(name: &str) -> Message {
    Message {
        from: "bpmux".to_string(),
        text: format!("{name} left"),
        ts: Utc::now(),
        message_type: MessageType::System,
        to: None,
    }
}

#[test]
fn track_agents_adds_on_join() {
    let mut agents = Vec::new();
    track_agents(&system_join("dev:claude-code"), "ryan", &mut agents);
    assert_eq!(agents, vec!["dev"]);
}

#[test]
fn track_agents_removes_on_leave() {
    let mut agents = vec!["dev".to_string(), "pm".to_string()];
    track_agents(&system_leave("dev:claude-code"), "ryan", &mut agents);
    assert_eq!(agents, vec!["pm"]);
}

#[test]
fn track_agents_ignores_owner() {
    let mut agents = Vec::new();
    track_agents(&system_join("ryan:human"), "ryan", &mut agents);
    assert!(agents.is_empty(), "owner should not be tracked as agent");
}

#[test]
fn track_agents_no_duplicates() {
    let mut agents = Vec::new();
    track_agents(&system_join("dev:claude-code"), "ryan", &mut agents);
    track_agents(&system_join("dev:claude-code"), "ryan", &mut agents);
    assert_eq!(agents.len(), 1);
}

#[test]
fn build_nudge_text_single_experience() {
    let text = build_nudge_text(&["you should focus on the problem".to_string()]);
    assert!(text.contains("1 experience noted"));
    assert!(text.contains("focus on the problem"));
    assert!(text.contains("save_lesson"));
}

#[test]
fn build_nudge_text_multiple_experiences() {
    let text = build_nudge_text(&["you should focus".to_string(), "quality is key".to_string()]);
    assert!(text.contains("2 experiences noted"));
    assert!(text.contains("1. you should focus"));
    assert!(text.contains("2. quality is key"));
}
