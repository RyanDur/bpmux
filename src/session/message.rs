use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub from: String,
    pub text: String,
    pub ts: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_type: Option<String>,
    /// Addressed recipients. `None` = broadcast (everyone sees it).
    /// `Some(vec)` = only named participants (plus sender) see the push.
    /// Names are short-form (e.g. `"dev"`, not `"dev:claude-code"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,
}

impl Message {
    pub fn is_chat(&self) -> bool {
        self.message_type.as_deref().unwrap_or("chat") == "chat"
    }
}
