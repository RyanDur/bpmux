#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::message::*;
use chrono::{DateTime, Utc};

#[test]
fn chat_message_is_chat() {
    let msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
        addressee: None,
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
        to: None,
        addressee: None,
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
        to: None,
        addressee: None,
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
        message_type: MessageType::Chat,
        to: None,
        addressee: None,
    };

    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed, msg);
}

// ---------- #158: addressee field ----------

#[test]
fn addressee_omitted_by_default_in_json() {
    // Backward compatibility: a message with no addressee serializes
    // without the field, matching the pre-#158 wire format. Existing
    // consumers that don't know about `addressee` keep working.
    let msg = Message {
        from: "ryan:human".to_string(),
        text: "hi".to_string(),
        ts: Utc::now(),
        message_type: MessageType::Chat,
        to: None,
        addressee: None,
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
        message_type: MessageType::Chat,
        to: None,
        addressee: Some("pm".to_string()),
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(json.contains(r#""addressee":"pm""#));
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.addressee.as_deref(), Some("pm"));
}

#[test]
fn addressee_deserializes_from_pre_158_json() {
    // Messages saved before #158 shipped don't have the field. They
    // must deserialize cleanly as `addressee: None` — backward compat.
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
    // Core individuality-preserving check: a pm-addressed message
    // should not count as directed at dev or designer.
    let msg = Message {
        addressee: Some("pm".to_string()),
        ..Message::default()
    };
    assert!(msg.is_addressed_to("pm"));
    assert!(!msg.is_addressed_to("dev"));
    assert!(!msg.is_addressed_to("designer"));
}

#[test]
fn addressee_is_independent_of_to() {
    // Both fields coexist: a broadcast-visible message (to: None)
    // can still have an addressee, and a DM (to: Some([...])) can
    // have no addressee. The two fields answer different questions.
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
    assert!(dm_no_addressee.is_addressed_to("dev")); // addressee: None = everyone
}
