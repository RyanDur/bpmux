use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MessageType {
    #[default]
    Chat,
    Activity,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub from: String,
    pub text: String,
    pub ts: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "MessageType::is_chat")]
    pub message_type: MessageType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,
}

impl MessageType {
    pub fn is_chat(&self) -> bool {
        *self == MessageType::Chat
    }

    pub fn to_wire(&self) -> Option<String> {
        match self {
            MessageType::Chat => None,
            MessageType::Activity => Some("activity".into()),
            MessageType::System => Some("system".into()),
        }
    }
}

impl From<Option<&str>> for MessageType {
    fn from(s: Option<&str>) -> Self {
        match s {
            Some("activity") => MessageType::Activity,
            Some("system") => MessageType::System,
            _ => MessageType::Chat,
        }
    }
}

impl Message {
    pub fn is_chat(&self) -> bool {
        self.message_type.is_chat()
    }
}
