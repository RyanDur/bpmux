#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::ws::*;
use crate::bridge::resolve_bridge_name_with_role;
use crate::session;
use chrono::Utc;

fn room_msg(from: &str, to: Option<Vec<&str>>) -> session::Message {
    session::Message {
        from: from.to_string(),
        text: "hello from PM room".to_string(),
        ts: Utc::now(),
        to: to.map(|v| v.into_iter().map(String::from).collect()),
        ..session::Message::default()
    }
}

#[test]
fn room_spawned_agent_identifies_with_its_room_role_not_a_random_agent_id() {
    assert_eq!(
        resolve_bridge_name_with_role("claude-code", Some("pm")),
        "pm",
        "a generic claude-code spawned in the pm-room must adopt BPMUX_ROLE as its bridge identity, not a random agent-id"
    );
    assert_eq!(
        resolve_bridge_name_with_role("${BPMUX_AGENT_NAME}", Some("designer")),
        "designer",
        "an unexpanded placeholder name must fall back to the room-role, not a random agent-id"
    );
}

#[test]
fn explicit_bridge_name_wins_over_room_role_env() {
    assert_eq!(
        resolve_bridge_name_with_role("dev", Some("pm")),
        "dev",
        "an explicit --name from a per-role workspace .mcp.json must win over BPMUX_ROLE"
    );
}

#[test]
fn generic_spawn_without_room_role_still_gets_a_stable_agent_id() {
    let resolved = resolve_bridge_name_with_role("claude-code", None);
    assert!(
        resolved.starts_with("agent-"),
        "with no room-role, a generic spawn keeps the existing random agent-id fallback; got {resolved}"
    );
}

#[test]
fn agent_in_room_send_reaches_team_broadcast_with_room_identity() {
    let from_pm_room = format!(
        "{}:claude-code",
        resolve_bridge_name_with_role("claude-code", Some("pm"))
    );
    let broadcast = room_msg(&from_pm_room, None);
    assert!(
        should_deliver(&broadcast, "designer:claude-code"),
        "a broadcast send from the pm-room must reach every other room's feed"
    );
    assert!(
        should_deliver(&broadcast, "ryan:human"),
        "the owner observes the routed message in the broadcast chat-feed"
    );
    assert_eq!(
        broadcast.from, "pm:claude-code",
        "the from-field carries the sender's room-identity, never the recipient room's"
    );
}

#[test]
fn room_identity_does_not_leak_across_rooms() {
    let pm_private = room_msg("pm:claude-code", Some(vec!["pm"]));
    assert!(
        !should_deliver(&pm_private, "designer:claude-code"),
        "the designer-room must not see a pm-scoped message — room-context isolation holds"
    );
    let designer_private = room_msg("designer:claude-code", Some(vec!["designer"]));
    assert!(
        !should_deliver(&designer_private, "pm:claude-code"),
        "isolation is symmetric — the pm-room must not see a designer-scoped message"
    );
}
