#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn resolve_bridge_name_uses_given_name_when_provided() {
    assert_eq!(resolve_bridge_name("dev"), "dev");
    assert_eq!(resolve_bridge_name("pm"), "pm");
    assert_eq!(resolve_bridge_name("designer"), "designer");
}

#[test]
fn resolve_bridge_name_generates_agent_suffix_for_default() {
    let name = resolve_bridge_name("claude-code");
    assert!(
        name.starts_with("agent-"),
        "expected agent-XXXX, got: {name}"
    );
    assert_eq!(name.len(), "agent-".len() + 4);
}

#[test]
fn resolve_bridge_name_generates_agent_suffix_when_empty() {
    let name = resolve_bridge_name("");
    assert!(
        name.starts_with("agent-"),
        "empty name should auto-generate, got: {name}"
    );
}

#[test]
fn resolve_bridge_name_generates_agent_suffix_for_unexpanded_placeholder() {
    let name = resolve_bridge_name("${BPMUX_ROLE}");
    assert!(
        name.starts_with("agent-"),
        "unexpanded placeholder should auto-generate, got: {name}"
    );
}
