use serde::{Deserialize, Serialize};

use crate::session::Message;

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
        addressee: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "rename")]
    Rename { name: String },
    #[serde(rename = "pty_subscribe")]
    PtySubscribe { terminal_id: String },
    #[serde(rename = "pty_input")]
    PtyInput { terminal_id: String, data: String },
}

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
        addressee: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        multi_depth: Box<Option<crate::session::message::MultiDepthPayload>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state_delta: Option<Box<crate::session::message::StateDeltaPayload>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state_snapshot: Option<Box<crate::session::message::StateSnapshotPayload>>,
    },
    #[serde(rename = "error")]
    Error { reason: String },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "pty_output")]
    PtyOutput { terminal_id: String, data: String },
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
            multi_depth: Box::new(msg.multi_depth),
            state_delta: msg.state_delta.map(Box::new),
            state_snapshot: msg.state_snapshot.map(Box::new),
        }
    }
}
