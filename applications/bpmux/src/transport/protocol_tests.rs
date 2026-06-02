#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::protocol::*;
use crate::session::Message;

#[test]
fn deserializes_identify_without_tool() {
    let json = r#"{"type": "identify", "name": "claude"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Identify { name, tool } => {
            assert_eq!(name, "claude");
            assert_eq!(tool, None);
        }
        other => panic!("expected Identify, got {other:?}"),
    }
}

#[test]
fn deserializes_identify_with_tool() {
    let json = r#"{"type": "identify", "name": "claude", "tool": "claude-code"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Identify { name, tool } => {
            assert_eq!(name, "claude");
            assert_eq!(tool, Some("claude-code".to_string()));
        }
        other => panic!("expected Identify, got {other:?}"),
    }
}

#[test]
fn deserializes_send() {
    let json = r#"{"type": "send", "text": "hello"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Send { text, to, .. } => {
            assert_eq!(text, "hello");
            assert_eq!(to, None);
        }
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn deserializes_pty_subscribe() {
    let json = r#"{"type": "pty_subscribe", "terminal_id": "term-1"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::PtySubscribe { terminal_id } => assert_eq!(terminal_id, "term-1"),
        other => panic!("expected PtySubscribe, got {other:?}"),
    }
}

#[test]
fn deserializes_pty_input_with_terminal_id_and_data() {
    let json = r#"{"type": "pty_input", "terminal_id": "term-1", "data": "Aw=="}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::PtyInput { terminal_id, data } => {
            assert_eq!(terminal_id, "term-1");
            assert_eq!(data, "Aw==");
        }
        other => panic!("expected PtyInput, got {other:?}"),
    }
}

#[test]
fn pty_output_serializes_with_distinct_type_terminal_id_and_data() {
    let msg = ServerMessage::PtyOutput {
        terminal_id: "term-1".to_string(),
        data: "G1szMW1yZWQ=".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(
        json.contains(r#""type":"pty_output""#),
        "distinct frame type: {json}"
    );
    assert!(
        json.contains(r#""terminal_id":"term-1""#),
        "carries terminal_id: {json}"
    );
    assert!(
        json.contains(r#""data":"G1szMW1yZWQ=""#),
        "carries data: {json}"
    );
}

#[test]
fn deserializes_pty_resize_with_terminal_id_cols_and_rows() {
    let json = r#"{"type": "pty_resize", "terminal_id": "term-1", "cols": 120, "rows": 40}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::PtyResize {
            terminal_id,
            cols,
            rows,
        } => {
            assert_eq!(terminal_id, "term-1");
            assert_eq!(cols, 120);
            assert_eq!(rows, 40);
        }
        other => panic!("expected PtyResize, got {other:?}"),
    }
}

#[test]
fn deserializes_send_with_to() {
    let json = r#"{"type": "send", "text": "hey", "to": ["pm", "designer"]}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Send { text, to, .. } => {
            assert_eq!(text, "hey");
            assert_eq!(to, Some(vec!["pm".to_string(), "designer".to_string()]));
        }
        other => panic!("expected Send, got {other:?}"),
    }
}

#[test]
fn serializes_welcome() {
    let msg = ServerMessage::Welcome {
        session_id: "abc123".to_string(),
    };
    let json = serde_json::to_string(&msg).expect("serialize");
    assert!(json.contains(r#""type":"welcome"#));
    assert!(json.contains(r#""session_id":"abc123"#));
}

#[test]
fn deserializes_rename() {
    let json = r#"{"type": "rename", "name": "new-name"}"#;
    let msg: ClientMessage = serde_json::from_str(json).expect("parse");
    match msg {
        ClientMessage::Rename { name } => assert_eq!(name, "new-name"),
        other => panic!("expected Rename, got {other:?}"),
    }
}

#[test]
fn serializes_message_from_session_message() {
    let session_msg = Message {
        from: "dev:human".to_string(),
        text: "hello".to_string(),
        ts: chrono::Utc::now(),
        ..Message::default()
    };
    let server_msg: ServerMessage = session_msg.into();
    let json = serde_json::to_string(&server_msg).expect("serialize");
    assert!(json.contains(r#""type":"message"#));
    assert!(json.contains(r#""from":"dev:human"#));
}
