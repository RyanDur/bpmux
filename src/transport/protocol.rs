use serde::{Deserialize, Serialize};

use crate::session::Message;

/// Messages sent from WebSocket client to server
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "identify")]
    Identify {
        name: String,
        #[serde(default)]
        tool: Option<String>,
    },
    #[serde(rename = "send")]
    Send {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "rename")]
    Rename { name: String },
    #[serde(rename = "handoff")]
    Handoff { text: String },
}

/// Messages sent from server to WebSocket client
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome { session_id: String },
    #[serde(rename = "message")]
    Message {
        from: String,
        text: String,
        ts: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
    },
    #[serde(rename = "error")]
    Error { reason: String },
    #[serde(rename = "pong")]
    Pong,
}

impl From<Message> for ServerMessage {
    fn from(msg: Message) -> Self {
        ServerMessage::Message {
            from: msg.from,
            text: msg.text,
            ts: msg.ts.to_rfc3339(),
            to: msg.to,
            message_type: msg.message_type,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

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
            message_type: None,
            to: None,
        };
        let server_msg: ServerMessage = session_msg.into();
        let json = serde_json::to_string(&server_msg).expect("serialize");
        assert!(json.contains(r#""type":"message"#));
        assert!(json.contains(r#""from":"dev:human"#));
    }
}
