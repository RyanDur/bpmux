use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    pub from: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "domain")]
pub enum StateDeltaPayload {
    #[serde(rename = "reactions")]
    Reactions {
        channel: String,
        #[serde(rename = "aggregated")]
        aggregated: Vec<ReactionAggregateWire>,
    },
    #[serde(rename = "ci")]
    Ci { snapshot: CiSnapshotWire },
    #[serde(rename = "roster")]
    Roster { members: Vec<RosterMemberWire> },
    #[serde(rename = "iteration_close")]
    IterationClose {
        promoted: Vec<u64>,
        deleted_marker_id: Option<u64>,
    },
    #[serde(rename = "iteration_close_blocked")]
    IterationCloseBlocked {
        blockers: Vec<IterationCloseBlockerPayload>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IterationCloseBlockerPayload {
    pub number: u64,
    pub title: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StateSnapshotPayload {
    #[serde(default)]
    pub reactions: Vec<ReactionsChannelSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ci: Option<CiSnapshotWire>,
    #[serde(default)]
    pub roster: Vec<RosterMemberWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReactionsChannelSnapshot {
    pub channel: String,
    pub aggregated: Vec<ReactionAggregateWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReactionAggregateWire {
    pub message_key: String,
    pub emoji: String,
    pub count: usize,
    pub users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiSnapshotWire {
    pub state: crate::ci::CiState,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TerminalState {
    Alive,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RosterMemberWire {
    pub name: String,
    pub identity: String,
    pub role: crate::orchestrator::stores::RosterRole,
    pub presence: crate::orchestrator::stores::Presence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_state: Option<TerminalState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiDepthPayload {
    pub summary: String,

    #[serde(default)]
    pub role_default: std::collections::HashMap<String, String>,

    pub deep: DeepDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeepDetail {
    Preloaded {
        text: String,
    },

    OnDemand {
        command: String,
        description: String,
    },
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tolerant_state_delta_tests {
    use super::*;

    #[test]
    fn message_with_unknown_state_delta_variant_parses_as_none() {
        let raw = r#"{
          "from": "bpmux",
          "text": "",
          "ts": "2026-05-15T01:23:45Z",
          "message_type": "state-delta",
          "state_delta": {"domain": "icebox_migration", "details": "retired"}
        }"#;
        let m: Message = serde_json::from_str(raw).expect("retired variant must parse tolerantly");
        assert_eq!(m.from, "bpmux");
        assert!(m.state_delta.is_none());
    }

    #[test]
    fn message_with_known_state_delta_variant_still_parses_canonically() {
        let raw = r#"{
          "from": "bpmux",
          "text": "",
          "ts": "2026-05-16T01:23:45Z",
          "message_type": "state-delta",
          "state_delta": {"domain": "roster", "members": []}
        }"#;
        let m: Message = serde_json::from_str(raw).expect("known variant parses");
        assert!(matches!(
            m.state_delta,
            Some(StateDeltaPayload::Roster { members: _ })
        ));
    }

    #[test]
    fn read_messages_skips_zero_lines_with_legacy_state_delta_variant() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("messages-current.jsonl");
        let lines = [
            r#"{"from":"a","text":"first","ts":"2026-05-01T00:00:00Z"}"#,
            r#"{"from":"bpmux","text":"","ts":"2026-05-15T00:00:00Z","message_type":"state-delta","state_delta":{"domain":"icebox_migration","details":"retired"}}"#,
            r#"{"from":"b","text":"third","ts":"2026-06-01T00:00:00Z"}"#,
        ];
        std::fs::write(&path, lines.join("\n")).unwrap();
        let parsed = crate::session::log::read_messages(dir.path())
            .expect("read_messages tolerates legacy state-delta variants");
        assert_eq!(parsed.len(), 3, "all 3 lines parse — none dropped");
        assert!(parsed[1].state_delta.is_none(), "legacy variant → None");
    }
}
