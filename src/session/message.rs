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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

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
}
