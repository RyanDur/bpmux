use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::multi_depth::MultiDepthPayload;
use super::state_delta::StateDeltaPayload;
use super::state_snapshot::StateSnapshotPayload;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    #[default]
    Chat,
    Activity,
    System,
    Insight,
    Nudge,
    #[serde(rename = "state-delta")]
    StateDelta,
    #[serde(rename = "state-snapshot")]
    StateSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Message {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    pub from: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender_id: Option<String>,
    pub text: String,
    pub ts: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "MessageType::is_default_chat")]
    pub message_type: MessageType,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addressee: Option<String>,

    #[serde(default, skip_serializing_if = "is_false")]
    pub explicit_privacy: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_depth: Option<MultiDepthPayload>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_state_delta_tolerant"
    )]
    pub state_delta: Option<StateDeltaPayload>,

    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_state_snapshot_tolerant"
    )]
    pub state_snapshot: Option<StateSnapshotPayload>,
}

fn deserialize_state_delta_tolerant<'de, D>(
    deserializer: D,
) -> Result<Option<StateDeltaPayload>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(raw.and_then(|v| serde_json::from_value(v).ok()))
}

fn deserialize_state_snapshot_tolerant<'de, D>(
    deserializer: D,
) -> Result<Option<StateSnapshotPayload>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(raw.and_then(|v| serde_json::from_value(v).ok()))
}

pub const BROADCAST_ALL: &str = "all";

fn is_false(b: &bool) -> bool {
    !*b
}

impl MessageType {
    pub fn is_chat(&self) -> bool {
        matches!(self, MessageType::Chat | MessageType::Insight)
    }

    fn is_default_chat(&self) -> bool {
        matches!(self, MessageType::Chat)
    }

    pub fn is_insight(&self) -> bool {
        *self == MessageType::Insight
    }

    pub fn to_wire(&self) -> Option<String> {
        match self {
            MessageType::Chat => None,
            MessageType::Activity => Some("activity".into()),
            MessageType::System => Some("system".into()),
            MessageType::Insight => Some("insight".into()),
            MessageType::Nudge => Some("nudge".into()),
            MessageType::StateDelta => Some("state-delta".into()),
            MessageType::StateSnapshot => Some("state-snapshot".into()),
        }
    }
}

impl From<Option<&str>> for MessageType {
    fn from(s: Option<&str>) -> Self {
        match s {
            Some("activity") => MessageType::Activity,
            Some("system") => MessageType::System,
            Some("insight") => MessageType::Insight,
            Some("nudge") => MessageType::Nudge,
            Some("state-delta") => MessageType::StateDelta,
            Some("state-snapshot") => MessageType::StateSnapshot,
            _ => MessageType::Chat,
        }
    }
}

impl Message {
    pub fn ensure_id(&mut self) {
        if self.id.is_empty() {
            self.id = uuid::Uuid::new_v4().to_string();
        }
    }

    pub fn is_chat(&self) -> bool {
        self.message_type.is_chat()
    }

    pub fn is_addressed_to(&self, role: &str) -> bool {
        match self.addressee.as_deref() {
            None => true,
            Some(BROADCAST_ALL) => true,
            Some(target) => target == role,
        }
    }
}
