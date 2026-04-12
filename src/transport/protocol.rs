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
        /// Attention signal (#158): role this message is directed at.
        /// See `Message::addressee` for the full semantic.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        addressee: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
        /// Topic tag (#191). Scopes the message to a named topic.
        /// Omit for broadcast (visible in all topics).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        topic: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "rename")]
    Rename { name: String },
    /// Subscribe to topics so delivery-side filtering knows what
    /// this peer should receive (#191).
    #[serde(rename = "subscribe")]
    Subscribe { topics: Vec<String> },
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
        /// Attention signal (#158) — see `Message::addressee`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        addressee: Option<String>,
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
            addressee: msg.addressee,
            message_type: msg.message_type.to_wire(),
        }
    }
}
