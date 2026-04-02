use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub from: String,
    pub text: String,
    pub ts: DateTime<Utc>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn serializes_message_to_json() {
        let msg = Message {
            from: "ryan:human".to_string(),
            text: "hello world".to_string(),
            ts: DateTime::parse_from_rfc3339("2026-04-02T12:00:00Z")
                .expect("test timestamp")
                .with_timezone(&Utc),
        };

        let json = serde_json::to_string(&msg).expect("serialize");
        let parsed: Message = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(parsed, msg);
    }
}
