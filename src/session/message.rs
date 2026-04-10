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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Message {
    pub from: String,
    pub text: String,
    pub ts: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "MessageType::is_chat")]
    pub message_type: MessageType,

    /// **Privacy filter** — recipients who can see this message. `None` or
    /// `Some(["all"])` means everyone. Used for directed DMs via `/to <role>`.
    /// Enforced as a hard delivery filter server-side (`ws::should_deliver`).
    /// Distinct from `addressee`, which is an attention signal on broadcast
    /// messages rather than a privacy filter.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,

    /// **Attention signal** (#158) — which single role this message is
    /// directed at. Everyone in `to` still RECEIVES the message; the
    /// addressee is the role whose turn it is to respond. Agents whose
    /// role doesn't match should see the message in their log but not
    /// treat it as a call to respond. Soft/advisory — enforcement is
    /// agent-side convention, not bridge-side filtering. See
    /// `docs/turn-aware-channel-spec.md` Layer 1 for the full design.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub addressee: Option<String>,
}

impl MessageType {
    pub fn is_chat(&self) -> bool {
        matches!(self, MessageType::Chat | MessageType::Insight)
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
        }
    }
}

impl From<Option<&str>> for MessageType {
    fn from(s: Option<&str>) -> Self {
        match s {
            Some("activity") => MessageType::Activity,
            Some("system") => MessageType::System,
            Some("insight") => MessageType::Insight,
            _ => MessageType::Chat,
        }
    }
}

impl Message {
    pub fn is_chat(&self) -> bool {
        self.message_type.is_chat()
    }

    /// Advisory: should this role process the message as their turn to
    /// respond? Per #158's spec, a message with `addressee: None` or
    /// `addressee: Some("all")` is for everyone; a specific role means
    /// other roles should see it in their log but skip responding.
    ///
    /// Soft/advisory — this is a helper for agent-side discipline, not a
    /// gate. The bridge doesn't call this; agents do.
    pub fn is_addressed_to(&self, role: &str) -> bool {
        match self.addressee.as_deref() {
            None => true,
            Some("all") => true,
            Some(target) => target == role,
        }
    }
}
