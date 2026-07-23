use serde::{Deserialize, Serialize};

use message::Message;
use product_management::iteration::IterationId;
use product_management::outcome::OutcomeId;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "identify")]
    Identify {
        name: String,
        #[serde(default)]
        tool: Option<String>,
        #[serde(default)]
        cursor: Option<String>,
        #[serde(default)]
        user_id: Option<String>,
        #[serde(default)]
        owner_nonce: Option<String>,
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
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_ids: Option<Vec<String>>,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "rename")]
    Rename { name: String },
    #[serde(rename = "pty_subscribe")]
    PtySubscribe { terminal_id: String },
    #[serde(rename = "pty_input")]
    PtyInput { terminal_id: String, data: String },
    #[serde(rename = "pty_resize")]
    PtyResize {
        terminal_id: String,
        cols: u16,
        rows: u16,
    },
    #[serde(rename = "reaction_toggle")]
    ReactionToggle {
        channel: String,
        message_key: String,
        emoji: String,
        user: String,
    },
    #[serde(rename = "comments_hydrate")]
    CommentsHydrate { story_id: u64 },
    #[serde(rename = "timeline_hydrate")]
    TimelineHydrate { story_id: u64 },
    #[serde(rename = "history_hydrate")]
    HistoryHydrate {
        chat_limit: usize,
        activity_limit: usize,
    },
    #[serde(rename = "history_page")]
    HistoryPage {
        request_id: u64,
        before: String,
        limit: usize,
    },
    #[serde(rename = "set_outcome_horizon")]
    SetOutcomeHorizon {
        outcome_id: OutcomeId,
        horizon: String,
    },
    #[serde(rename = "reorder_outcome_iteration")]
    ReorderOutcomeIteration {
        outcome_id: OutcomeId,
        iteration_id: IterationId,
        position: u32,
    },
    #[serde(rename = "reprioritize_outcome")]
    ReprioritizeOutcome {
        outcome_id: OutcomeId,
        position: u32,
    },
    #[serde(rename = "reorder_iteration")]
    ReorderIteration {
        iteration_id: IterationId,
        position: u32,
    },
    #[serde(rename = "add_story_to_iteration")]
    AddStoryToIteration {
        iteration_id: IterationId,
        story_id: u64,
        position: u32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "welcome")]
    Welcome {},
    #[serde(rename = "message")]
    Message {
        from: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sender_id: Option<String>,
        text: String,
        ts: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_ids: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        addressee: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        addressee_ids: Option<Vec<String>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message_type: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        multi_depth: Box<Option<message::MultiDepthPayload>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state_delta: Option<Box<message::StateDeltaPayload>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state_snapshot: Option<Box<message::StateSnapshotPayload>>,
    },
    #[serde(rename = "error")]
    Error { reason: String },
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "pty_output")]
    PtyOutput { terminal_id: String, data: String },
    #[serde(rename = "pty_exit")]
    PtyExit { terminal_id: String },
    #[serde(rename = "pty_mode")]
    PtyMode {
        terminal_id: String,
        bracketed_paste: bool,
    },
    #[serde(rename = "comments_hydrated")]
    CommentsHydrated {
        story_id: u64,
        comments: Vec<serde_json::Value>,
    },
    #[serde(rename = "timeline_hydrated")]
    TimelineHydrated {
        story_id: u64,
        events: Vec<serde_json::Value>,
    },
    #[serde(rename = "history_hydrated")]
    HistoryHydrated { messages: Vec<serde_json::Value> },
    #[serde(rename = "history_page_loaded")]
    HistoryPageLoaded {
        request_id: u64,
        messages: Vec<serde_json::Value>,
    },
}

impl From<Message> for ServerMessage {
    fn from(msg: Message) -> Self {
        ServerMessage::Message {
            from: msg.from,
            sender_id: msg.sender_id,
            text: msg.text,
            ts: msg.ts.to_rfc3339(),
            to: msg.to,
            to_ids: msg.to_ids,
            addressee: msg.addressee,
            addressee_ids: msg.addressee_ids,
            message_type: msg.message_type.to_wire(),
            multi_depth: Box::new(msg.multi_depth),
            state_delta: msg.state_delta.map(Box::new),
            state_snapshot: msg.state_snapshot.map(Box::new),
        }
    }
}

pub const SEND_FIELDS: [&str; 6] = ["type", "text", "to", "addressee", "message_type", "to_ids"];

pub fn parse_client_frame(text: &str) -> Result<ClientMessage, String> {
    let message: ClientMessage = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if matches!(message, ClientMessage::Send { .. }) {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if let Some(frame) = value.as_object() {
            for key in frame.keys() {
                if !SEND_FIELDS.contains(&key.as_str()) {
                    return Err(format!(
                        "unknown field '{key}' on a send frame — client/server version skew; a send must never degrade silently (rebuild both sides)"
                    ));
                }
            }
        }
    }
    Ok(message)
}

#[cfg(test)]
mod frame_tests;
