#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::message::*;
use chrono::{DateTime, Utc};

#[test]
fn ensure_id_mints_a_valid_uuid_when_absent() {
    let mut msg = Message::default();
    assert!(msg.id.is_empty());
    msg.ensure_id();
    assert!(uuid::Uuid::parse_str(&msg.id).is_ok());
}

#[test]
fn ensure_id_preserves_an_id_that_is_already_set() {
    let mut msg = Message {
        id: "already-minted".to_string(),
        ..Message::default()
    };
    msg.ensure_id();
    assert_eq!(msg.id, "already-minted");
}

#[test]
fn chat_message_is_chat() {
    let msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    assert!(msg.is_chat());
}

#[test]
fn system_message_is_not_chat() {
    let msg = Message {
        from: "owner:claude-code".to_string(),
        text: "[turn complete]".to_string(),
        ts: Utc::now(),
        message_type: MessageType::System,
        ..Message::default()
    };
    assert!(!msg.is_chat());
}

#[test]
fn explicit_chat_type_is_chat() {
    let msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        ..Message::default()
    };
    assert!(msg.is_chat());
}

#[test]
fn serializes_message_to_json() {
    let msg = Message {
        from: "owner:human".to_string(),
        text: "hello world".to_string(),
        ts: DateTime::parse_from_rfc3339("2026-04-02T12:00:00Z")
            .expect("test timestamp")
            .with_timezone(&Utc),
        ..Message::default()
    };

    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed, msg);
}

#[test]
fn addressee_omitted_by_default_in_json() {
    let msg = Message {
        from: "owner:human".to_string(),
        text: "hi".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(!json.contains("addressee"));
}

#[test]
fn addressee_roundtrips_through_json() {
    let msg = Message {
        from: "owner:human".to_string(),
        text: "what about X?".to_string(),
        ts: Utc::now(),
        addressee: Some("pm".to_string()),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(json.contains(r#""addressee":"pm""#));
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.addressee.as_deref(), Some("pm"));
}

#[test]
fn addressee_deserializes_from_pre_158_json() {
    let json = r#"{"from":"owner:human","text":"hi","ts":"2026-04-02T12:00:00Z","to":null}"#;
    let parsed: Message = serde_json::from_str(json).expect("deserialize");
    assert!(parsed.addressee.is_none());
}

#[test]
fn is_addressed_to_none_is_for_everyone() {
    let msg = Message {
        addressee: None,
        ..Message::default()
    };
    assert!(msg.is_addressed_to("dev"));
    assert!(msg.is_addressed_to("pm"));
    assert!(msg.is_addressed_to("designer"));
}

#[test]
fn is_addressed_to_all_is_for_everyone() {
    let msg = Message {
        addressee: Some("all".to_string()),
        ..Message::default()
    };
    assert!(msg.is_addressed_to("dev"));
    assert!(msg.is_addressed_to("pm"));
    assert!(msg.is_addressed_to("designer"));
}

#[test]
fn is_addressed_to_specific_role_excludes_others() {
    let msg = Message {
        addressee: Some("pm".to_string()),
        ..Message::default()
    };
    assert!(msg.is_addressed_to("pm"));
    assert!(!msg.is_addressed_to("dev"));
    assert!(!msg.is_addressed_to("designer"));
}

#[test]
fn message_without_multi_depth_serializes_without_the_field() {
    let msg = Message {
        from: "dev:claude-code".to_string(),
        text: "plain".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(
        !json.contains("multi_depth"),
        "absent field should be omitted from wire format: {json}"
    );
}

#[test]
fn message_deserializes_from_pre_187_json_without_multi_depth() {
    let json = r#"{"from":"owner:human","text":"hi","ts":"2026-04-02T12:00:00Z","to":null}"#;
    let parsed: Message = serde_json::from_str(json).expect("deserialize");
    assert!(parsed.multi_depth.is_none());
}

#[test]
fn multi_depth_payload_roundtrips_preloaded_deep_detail() {
    let mut role_default = std::collections::HashMap::new();
    role_default.insert("pm".to_string(), "CI ✗".to_string());
    role_default.insert(
        "dev".to_string(),
        "CI ✗ — run #12345 failed at cargo clippy".to_string(),
    );
    let payload = MultiDepthPayload {
        summary: "CI ✗".to_string(),
        role_default,
        deep: DeepDetail::Preloaded {
            text: "full build log contents here".to_string(),
        },
    };
    let msg = Message {
        from: "bpmux".to_string(),
        text: "CI ✗".to_string(),
        ts: Utc::now(),
        multi_depth: Some(payload.clone()),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.multi_depth, Some(payload));
}

#[test]
fn multi_depth_payload_roundtrips_on_demand_deep_detail() {
    let payload = MultiDepthPayload {
        summary: "CI ✗".to_string(),
        role_default: std::collections::HashMap::new(),
        deep: DeepDetail::OnDemand {
            command: "gh run view 12345".to_string(),
            description: "click to load the build log".to_string(),
        },
    };
    let msg = Message {
        from: "bpmux".to_string(),
        text: "CI ✗".to_string(),
        ts: Utc::now(),
        multi_depth: Some(payload.clone()),
        ..Message::default()
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(json.contains(r#""kind":"on_demand""#), "got: {json}");
    assert!(json.contains(r#""command":"gh run view 12345""#));
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.multi_depth, Some(payload));
}

#[test]
fn multi_depth_with_empty_role_default_still_roundtrips() {
    let payload = MultiDepthPayload {
        summary: "ok".to_string(),
        role_default: std::collections::HashMap::new(),
        deep: DeepDetail::Preloaded {
            text: "detail".to_string(),
        },
    };
    let json = serde_json::to_string(&payload).expect("serialize");
    let parsed: MultiDepthPayload = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed, payload);
}

#[test]
fn is_addressed_to_none_is_for_everyone_sanity() {
    let msg = Message::default();
    assert!(msg.is_addressed_to("dev"));
}

#[test]
fn addressee_is_independent_of_to() {
    let broadcast_with_addressee = Message {
        to: None,
        addressee: Some("pm".to_string()),
        ..Message::default()
    };
    assert!(broadcast_with_addressee.is_addressed_to("pm"));
    assert!(!broadcast_with_addressee.is_addressed_to("dev"));

    let dm_no_addressee = Message {
        to: Some(vec!["pm".to_string()]),
        addressee: None,
        ..Message::default()
    };
    assert!(dm_no_addressee.is_addressed_to("pm"));
    assert!(dm_no_addressee.is_addressed_to("dev"));
}

#[test]
fn nudge_message_is_not_chat() {
    let msg = Message {
        message_type: MessageType::Nudge,
        ..Message::default()
    };
    assert!(
        !msg.is_chat(),
        "Nudge is a system-authored operational signal, not chat — it must classify non-chat so the noise-filter hides it"
    );
}

#[test]
fn chat_omits_message_type_but_non_chat_serializes_it() {
    let chat = Message {
        message_type: MessageType::Chat,
        ..Message::default()
    };
    let chat_json = serde_json::to_string(&chat).expect("serialize");
    assert!(
        !chat_json.contains("message_type"),
        "Chat must omit message_type on the wire (default-chat): {chat_json}"
    );

    let insight = Message {
        message_type: MessageType::Insight,
        ..Message::default()
    };
    let insight_json = serde_json::to_string(&insight).expect("serialize");
    assert!(
        insight_json.contains(r#""message_type":"insight""#),
        "non-Chat types must serialize their wire-type so the renderer keeps the class-glyph: {insight_json}"
    );
}

#[test]
fn message_with_unknown_state_delta_variant_parses_as_none() {
    let raw = r#"{
          "from": "bpmux",
          "text": "",
          "ts": "2026-05-15T01:23:45Z",
          "message_type": "state-delta",
          "state_delta": {"domain": "icebox_migration", "details": "retired"}
        }"#;
    let m: Message = serde_json::from_str(raw).expect("retired variant must parse tolerantly");
    assert_eq!(m.from, "bpmux");
    assert!(m.state_delta.is_none());
}

#[test]
fn message_with_known_state_delta_variant_still_parses_canonically() {
    let raw = r#"{
          "from": "bpmux",
          "text": "",
          "ts": "2026-05-16T01:23:45Z",
          "message_type": "state-delta",
          "state_delta": {"domain": "roster", "members": []}
        }"#;
    let m: Message = serde_json::from_str(raw).expect("known variant parses");
    assert!(matches!(
        m.state_delta,
        Some(StateDeltaPayload::Roster { members: _ })
    ));
}

#[test]
fn read_messages_skips_zero_lines_with_legacy_state_delta_variant() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("messages-current.jsonl");
    let lines = [
        r#"{"from":"a","text":"first","ts":"2026-05-01T00:00:00Z"}"#,
        r#"{"from":"bpmux","text":"","ts":"2026-05-15T00:00:00Z","message_type":"state-delta","state_delta":{"domain":"icebox_migration","details":"retired"}}"#,
        r#"{"from":"b","text":"third","ts":"2026-06-01T00:00:00Z"}"#,
    ];
    std::fs::write(&path, lines.join("\n")).unwrap();
    let parsed = crate::log::read_messages(dir.path())
        .expect("read_messages tolerates legacy state-delta variants");
    assert_eq!(parsed.len(), 3, "all 3 lines parse — none dropped");
    assert!(parsed[1].state_delta.is_none(), "legacy variant → None");
}
