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
    let meta = build_notification_meta("ryan", "t", "broadcast", None, None);
    assert!(
        !meta.contains_key("addressee"),
        "addressee must be omitted when None"
    );
    assert!(
        !meta.contains_key("message_type"),
        "message_type must be omitted when None"
    );
}

#[test]
fn build_meta_includes_addressee_when_present() {
    let meta = build_notification_meta("ryan", "t", "broadcast", Some("dev"), None);
    assert_eq!(meta.get("addressee").and_then(|v| v.as_str()), Some("dev"));
}

#[test]
fn build_meta_includes_message_type_when_present() {
    let meta = build_notification_meta("ryan", "t", "broadcast", None, Some("insight"));
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
        build_notification_meta("r", "t", "c", None, None),
        build_notification_meta("r", "t", "c", Some("dev"), None),
        build_notification_meta("r", "t", "c", None, Some("chat")),
        build_notification_meta("r", "t", "c", Some("dev"), Some("chat")),
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
    let meta = build_notification_meta("ryan", "t", "broadcast", Some("dev"), Some("chat"));
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
    let meta = build_notification_meta("ryan", "t", "dev", Some("dev"), Some("chat"));
    assert!(meta.contains_key("addressee"));
    assert!(meta.contains_key("message_type"));
}

// ── curate_message tests (story #200) ────────────────────────
//
// curate_message is the read_messages response curator. It preserves
// addressee/topic/message_type/ts so agents can implement the #158
// addressee protocol and #191 topic scoping. Broadcast messages are
// distinguishable from addressed ones via null fields.

use super::tools::curate_message;

#[test]
fn curate_message_preserves_addressee_when_present() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "hello pm",
        "ts": "2026-04-15T21:00:00Z",
        "addressee": "pm",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(out.get("addressee").and_then(|v| v.as_str()), Some("pm"));
    assert_eq!(
        out.get("ts").and_then(|v| v.as_str()),
        Some("2026-04-15T21:00:00Z")
    );
}

#[test]
fn curate_message_preserves_topic_when_present() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "teaching point",
        "ts": "2026-04-15T21:00:00Z",
        "topic": "teaching",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(out.get("topic").and_then(|v| v.as_str()), Some("teaching"));
}

#[test]
fn curate_message_preserves_message_type_when_present() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "reasoning trace",
        "ts": "2026-04-15T21:00:00Z",
        "message_type": "insight",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(
        out.get("message_type").and_then(|v| v.as_str()),
        Some("insight")
    );
}

#[test]
fn curate_message_broadcast_has_null_addressee() {
    // Broadcast (no addressee) must be distinguishable from addressed.
    let raw = serde_json::json!({
        "from": "dev",
        "text": "broadcast",
        "ts": "2026-04-15T21:00:00Z",
    });
    let out = curate_message(&raw).expect("should curate");
    assert!(out.get("addressee").is_some(), "addressee key must exist");
    assert!(
        out["addressee"].is_null(),
        "broadcast addressee must be null"
    );
}

#[test]
fn curate_message_addressee_visible_across_recipients() {
    // pm must see a dev-addressed message with addressee: "dev" visible.
    // Curation does not filter based on viewer — agents decide.
    let raw = serde_json::json!({
        "from": "ryan",
        "text": "work on this dev",
        "ts": "2026-04-15T21:00:00Z",
        "addressee": "dev",
    });
    let out = curate_message(&raw).expect("should curate");
    assert_eq!(out.get("addressee").and_then(|v| v.as_str()), Some("dev"));
}

#[test]
fn curate_message_returns_none_when_required_fields_missing() {
    // Malformed message (missing `from`) is skipped entirely.
    let raw = serde_json::json!({
        "text": "no sender",
        "ts": "2026-04-15T21:00:00Z",
    });
    assert!(
        curate_message(&raw).is_none(),
        "missing `from` must skip the message"
    );

    let raw = serde_json::json!({
        "from": "dev",
        "ts": "2026-04-15T21:00:00Z",
    });
    assert!(
        curate_message(&raw).is_none(),
        "missing `text` must skip the message"
    );
}

#[test]
fn curate_message_broadcast_distinguishable_from_addressed() {
    let broadcast = serde_json::json!({
        "from": "ryan", "text": "hi team", "ts": "t",
    });
    let addressed = serde_json::json!({
        "from": "ryan", "text": "hi dev", "ts": "t", "addressee": "dev",
    });
    let broadcast_out = curate_message(&broadcast).expect("should curate");
    let addressed_out = curate_message(&addressed).expect("should curate");
    assert!(broadcast_out["addressee"].is_null());
    assert_eq!(addressed_out["addressee"].as_str(), Some("dev"));
    assert_ne!(broadcast_out, addressed_out, "outputs must differ in shape");
}

#[test]
fn curate_message_preserves_to_array_for_privacy_scoped() {
    let raw = serde_json::json!({
        "from": "dev",
        "text": "private",
        "ts": "t",
        "to": ["pm", "designer"],
    });
    let out = curate_message(&raw).expect("should curate");
    let to = out.get("to").and_then(|v| v.as_array()).expect("to array");
    assert_eq!(to.len(), 2);
    assert_eq!(to[0].as_str(), Some("pm"));
    assert_eq!(to[1].as_str(), Some("designer"));
}

// ── share_message pure helpers (#174) ────────────────────────

use super::tools::{format_share_marker, parse_share_scope, pick_most_recent_visible_chat};

#[test]
fn parse_share_scope_all_becomes_none() {
    assert_eq!(parse_share_scope("all"), Ok(None));
}

#[test]
fn parse_share_scope_single_role_wraps_in_vec() {
    assert_eq!(parse_share_scope("dev"), Ok(Some(vec!["dev".into()])));
}

#[test]
fn parse_share_scope_comma_list_splits_and_trims() {
    assert_eq!(
        parse_share_scope("dev, pm ,  designer"),
        Ok(Some(vec!["dev".into(), "pm".into(), "designer".into()]))
    );
}

#[test]
fn parse_share_scope_rejects_empty_and_whitespace_only() {
    assert!(parse_share_scope("").is_err());
    assert!(parse_share_scope(",").is_err());
    assert!(parse_share_scope(" , ").is_err());
}

#[test]
fn format_share_marker_form1_includes_source_scope() {
    let out = format_share_marker("dev", Some("pm"), "coordinate on story X");
    assert_eq!(out, "[Shared by dev from pm thread] coordinate on story X");
}

#[test]
fn format_share_marker_form2_omits_source_scope() {
    let out = format_share_marker("dev", None, "fresh thought");
    assert_eq!(out, "[Shared by dev] fresh thought");
}

#[test]
fn pick_most_recent_visible_chat_skips_non_chat_types() {
    // Most-recent-first ordering — an Activity message at index 0
    // (the most recent) must be skipped; the Chat at index 1 is
    // the correct pick.
    let messages = vec![
        serde_json::json!({
            "from": "dev",
            "text": "status update",
            "message_type": "activity",
        }),
        serde_json::json!({
            "from": "pm",
            "text": "the real one",
        }),
    ];
    let picked = pick_most_recent_visible_chat(&messages).expect("chat in list");
    assert_eq!(
        picked.get("text").and_then(|v| v.as_str()),
        Some("the real one")
    );
}

#[test]
fn pick_most_recent_visible_chat_returns_none_when_only_system_and_activity() {
    let messages = vec![
        serde_json::json!({
            "from": "bpmux",
            "text": "session started",
            "message_type": "system",
        }),
        serde_json::json!({
            "from": "dev",
            "text": "available",
            "message_type": "activity",
        }),
    ];
    assert!(pick_most_recent_visible_chat(&messages).is_none());
}

#[test]
fn pick_most_recent_visible_chat_treats_null_message_type_as_chat() {
    // The REST curator emits `message_type: null` for Chat (default
    // variant). The picker must treat that as chat, not skip it.
    let messages = vec![serde_json::json!({
        "from": "ryan",
        "text": "broadcast hello",
        "message_type": serde_json::Value::Null,
    })];
    let picked = pick_most_recent_visible_chat(&messages).expect("null == chat");
    assert_eq!(
        picked.get("text").and_then(|v| v.as_str()),
        Some("broadcast hello")
    );
}
