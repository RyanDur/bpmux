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

// ── build_notification_meta tests ──────────────────────────
//
// Why these tests matter: Claude Code v2.1.109+ rejects channel
// notifications whose meta contains null values. The rejection breaks
// the entire MCP stdio pipe, not just the single notification. This
// function is the last step before the payload crosses the Claude Code
// boundary — if it emits null, team communication breaks.
//
// Presence-as-signal is also load-bearing: "addressee: null" and
// "no addressee key" are semantically different. Dropping absent keys
// preserves that distinction.

#[test]
fn build_meta_always_includes_core_fields() {
    let meta = build_notification_meta(
        "ryan:human",
        "2026-04-15T20:00:00Z",
        "broadcast",
        None,
        None,
        None,
    );
    assert_eq!(
        meta.get("from").and_then(|v| v.as_str()),
        Some("ryan:human")
    );
    assert_eq!(
        meta.get("ts").and_then(|v| v.as_str()),
        Some("2026-04-15T20:00:00Z")
    );
    assert_eq!(
        meta.get("channel").and_then(|v| v.as_str()),
        Some("broadcast")
    );
    assert_eq!(meta.get("source").and_then(|v| v.as_str()), Some("bpmux"));
}

#[test]
fn build_meta_omits_optional_keys_when_none() {
    let meta = build_notification_meta("ryan", "t", "broadcast", None, None, None);
    assert!(
        !meta.contains_key("addressee"),
        "addressee must be omitted when None"
    );
    assert!(
        !meta.contains_key("topic"),
        "topic must be omitted when None"
    );
    assert!(
        !meta.contains_key("message_type"),
        "message_type must be omitted when None"
    );
}

#[test]
fn build_meta_includes_addressee_when_present() {
    let meta = build_notification_meta("ryan", "t", "broadcast", Some("dev"), None, None);
    assert_eq!(meta.get("addressee").and_then(|v| v.as_str()), Some("dev"));
}

#[test]
fn build_meta_includes_topic_when_present() {
    let meta = build_notification_meta("ryan", "t", "broadcast", None, Some("scroll-fix"), None);
    assert_eq!(
        meta.get("topic").and_then(|v| v.as_str()),
        Some("scroll-fix")
    );
}

#[test]
fn build_meta_includes_message_type_when_present() {
    let meta = build_notification_meta("ryan", "t", "broadcast", None, None, Some("insight"));
    assert_eq!(
        meta.get("message_type").and_then(|v| v.as_str()),
        Some("insight")
    );
}

#[test]
fn build_meta_never_contains_null_values() {
    // Contract for Claude Code v2.1.109+: null in meta crashes the
    // MCP stdio pipe. Verify across every combination of present/absent.
    let cases = [
        build_notification_meta("r", "t", "c", None, None, None),
        build_notification_meta("r", "t", "c", Some("dev"), None, None),
        build_notification_meta("r", "t", "c", None, Some("x"), None),
        build_notification_meta("r", "t", "c", None, None, Some("chat")),
        build_notification_meta("r", "t", "c", Some("dev"), Some("x"), Some("chat")),
    ];
    for meta in &cases {
        for (key, value) in meta {
            assert!(!value.is_null(), "meta[{key}] must not be null");
        }
    }
}

// ── validate_meta boundary check ───────────────────────────
//
// This is the runtime validator wired in just before peer.send_notification.
// Its job is to catch our own contract violations at the boundary rather
// than silently shipping them into Claude Code (where they vanish silently
// on v2.1.92 or crash the MCP pipe on v2.1.109+).

#[test]
fn validate_meta_accepts_payload_with_no_nulls() {
    let meta = build_notification_meta(
        "ryan",
        "t",
        "broadcast",
        Some("dev"),
        Some("topic"),
        Some("chat"),
    );
    assert!(validate_meta(&meta).is_ok());
}

#[test]
fn validate_meta_rejects_payload_with_null_value() {
    let mut meta = serde_json::Map::new();
    meta.insert("from".to_string(), serde_json::json!("ryan"));
    meta.insert("addressee".to_string(), serde_json::Value::Null);
    let err = validate_meta(&meta).expect_err("should reject null");
    assert!(err.contains("addressee"));
    assert!(err.contains("null"));
}

#[test]
fn validate_meta_rejects_payload_with_null_in_any_position() {
    let keys = ["from", "ts", "channel", "source", "addressee", "topic"];
    for null_key in keys {
        let mut meta = serde_json::Map::new();
        for k in keys {
            if k == null_key {
                meta.insert(k.to_string(), serde_json::Value::Null);
            } else {
                meta.insert(k.to_string(), serde_json::json!("value"));
            }
        }
        let result = validate_meta(&meta);
        assert!(
            result.is_err(),
            "validator must catch null at key {null_key}"
        );
    }
}

#[test]
fn validate_meta_accepts_empty_map() {
    let meta = serde_json::Map::new();
    assert!(validate_meta(&meta).is_ok());
}

#[test]
fn build_meta_mixed_presence_only_includes_present_keys() {
    let meta = build_notification_meta("ryan", "t", "dev", Some("dev"), None, Some("chat"));
    assert!(meta.contains_key("addressee"));
    assert!(!meta.contains_key("topic"), "topic None should be absent");
    assert!(meta.contains_key("message_type"));
}
