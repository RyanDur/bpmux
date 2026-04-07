#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::message::*;
use chrono::{DateTime, Utc};

#[test]
fn chat_message_is_chat() {
    let msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: Utc::now(),
        message_type: None,
        to: None,
    };
    assert!(msg.is_chat());
}

#[test]
fn system_message_is_not_chat() {
    let msg = Message {
        from: "ryan:claude-code".to_string(),
        text: "[turn complete]".to_string(),
        ts: Utc::now(),
        message_type: Some("system".to_string()),
        to: None,
    };
    assert!(!msg.is_chat());
}

#[test]
fn explicit_chat_type_is_chat() {
    let msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: Utc::now(),
        message_type: Some("chat".to_string()),
        to: None,
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
        message_type: None,
        to: None,
    };

    let json = serde_json::to_string(&msg).expect("serialize");
    let parsed: Message = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(parsed, msg);
}
