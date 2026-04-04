use serde::{Deserialize, Serialize};

use crate::session::Message;

/// Messages sent from WebSocket client to server
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "identify")]
    Identify { name: String },
    #[serde(rename = "send")]
    Send { text: String },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "rename")]
    Rename { name: String },
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_identify() {
        let json = r#"{"type": "identify", "name": "claude"}"#;
        let msg: ClientMessage = serde_json::from_str(json).expect("parse");
        match msg {
            ClientMessage::Identify { name } => assert_eq!(name, "claude"),
            other => panic!("expected Identify, got {other:?}"),
        }
    }

    #[test]
    fn deserializes_send() {
        let json = r#"{"type": "send", "text": "hello"}"#;
        let msg: ClientMessage = serde_json::from_str(json).expect("parse");
        match msg {
            ClientMessage::Send { text } => assert_eq!(text, "hello"),
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
    fn serializes_message_from_session_message() {
        let session_msg = Message {
            from: "dev:human".to_string(),
            text: "hello".to_string(),
            ts: chrono::Utc::now(),
            message_type: None,
        };
        let server_msg: ServerMessage = session_msg.into();
        let json = serde_json::to_string(&server_msg).expect("serialize");
        assert!(json.contains(r#""type":"message"#));
        assert!(json.contains(r#""from":"dev:human"#));
    }
}
