#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::message::*;
use chrono::{DateTime, Utc};

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
        from: "ryan:claude-code".to_string(),
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
        from: "ryan:human".to_string(),
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
        from: "ryan:human".to_string(),
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
        from: "ryan:human".to_string(),
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
    let json = r#"{"from":"ryan:human","text":"hi","ts":"2026-04-02T12:00:00Z","to":null}"#;
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
    let json = r#"{"from":"ryan:human","text":"hi","ts":"2026-04-02T12:00:00Z","to":null}"#;
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
