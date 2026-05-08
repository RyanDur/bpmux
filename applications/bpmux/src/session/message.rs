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
    /// Automated bpmux-generated nudge (classifier experience-nudge,
    /// handoff liveness warning, awareness signal). Distinct from
    /// `Chat` because these messages are SYSTEM-authored operational
    /// signals, not human/agent chat content, and #161's default
    /// view filter hides them. Distinct from `Activity` because
    /// activity is ephemeral state (status dots, presence), whereas
    /// nudges are content the classifier produced.
    ///
    /// This variant exists so handoff nudges classify reliably.
    /// Before its introduction, nudges were emitted as `Chat` and
    /// escaped the 📝-text-prefix filter because they didn't carry
    /// the marker.
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

    /// **Privacy intent marker** (#160) — set by senders who explicitly
    /// chose to scope a message via `/to <role> "text"`. Distinguishes
    /// user-typed directed DMs from channel-derived `to` lists (which
    /// look identical on the wire otherwise). When `true`, `/share`
    /// refuses to propagate the message — the sender's explicit privacy
    /// intent takes precedence over any cross-scope propagation
    /// request. False/default for channel-view-derived scoping.
    #[serde(default, skip_serializing_if = "is_false")]
    pub explicit_privacy: bool,

    /// **Multi-depth rendering payload** (#184 / #187) — optional
    /// structured data that lets a single event carry multiple
    /// rendering depths so each receiving role can render at its
    /// own default granularity with drill-down available. See
    /// `MultiDepthPayload` for the shape and `DeepDetail` for the
    /// preload-vs-on-demand split.
    ///
    /// `None` means "no multi-depth metadata, render the text as
    /// usual." Every existing sender produces `None`; receivers
    /// that don't understand the field ignore it. Backward-compat
    /// by construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_depth: Option<MultiDepthPayload>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_delta: Option<StateDeltaPayload>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_snapshot: Option<StateSnapshotPayload>,
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
    #[serde(rename = "pins")]
    Pins {
        channel: String,
        pins: Vec<PinnedMessageWire>,
    },
    #[serde(rename = "ci")]
    Ci { snapshot: CiSnapshotWire },
    #[serde(rename = "roster")]
    Roster { members: Vec<RosterMemberWire> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct StateSnapshotPayload {
    #[serde(default)]
    pub reactions: Vec<ReactionsChannelSnapshot>,
    #[serde(default)]
    pub pins: Vec<PinsChannelSnapshot>,
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
pub struct PinsChannelSnapshot {
    pub channel: String,
    pub pins: Vec<PinnedMessageWire>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReactionAggregateWire {
    pub message_key: String,
    pub emoji: String,
    pub count: usize,
    pub users: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PinnedMessageWire {
    pub message_key: String,
    pub pinner: String,
    pub ts: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CiSnapshotWire {
    pub state: crate::ci::CiState,
    pub run_id: Option<u64>,
    pub head_sha: Option<String>,
    pub display_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RosterMemberWire {
    pub name: String,
    pub identity: String,
    pub role: crate::orchestrator::stores::RosterRole,
    pub presence: crate::orchestrator::stores::Presence,
}

/// Multi-depth rendering payload carried on a `Message` (#187).
///
/// Encodes three rendering depths on a single event:
/// - `summary` — shallowest form, one short line, always shown
///   when the event is shown at all
/// - `role_default` — per-role default depth, keyed by role name.
///   Roles not in the map fall back to `summary`.
/// - `deep` — full detail, either preloaded inline or fetched
///   on demand via a bridge-executable command (token economy)
///
/// The sender decides what each role's default depth is. Receivers
/// still render through their own lens — the schema allows role-
/// tuned defaults without forcing them.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MultiDepthPayload {
    /// Shallowest representation — one short line. If the depth
    /// system displays the event at all, `summary` is the minimum
    /// that renders. Example for CI: `"CI \u{2717}"`.
    pub summary: String,

    /// Role-default depth variants. Keys are short role names
    /// (`"pm"`, `"dev"`, `"designer"`); values are the text to
    /// render for that role's default depth. Omitted roles fall
    /// back to `summary`.
    #[serde(default)]
    pub role_default: std::collections::HashMap<String, String>,

    /// Deep-detail data. See `DeepDetail` for the two variants.
    pub deep: DeepDetail,
}

/// Deep-detail payload on a multi-depth event. The discriminant
/// distinguishes preloaded from fetch-on-demand so downstream code
/// can decide whether to absorb the cost inline or defer it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DeepDetail {
    /// Full text embedded in the payload. Cheap for the renderer
    /// (no extra fetch) but every receiving agent absorbs the
    /// token cost even if they never drill down.
    Preloaded { text: String },

    /// Fetchable via a shell command when drill-down is requested.
    /// The command is executed by whichever consumer handles the
    /// drill-down (likely a bridge MCP tool in a follow-up story).
    /// `description` is a one-line human-readable label for the
    /// fetch so renderers can show "click to load the build log"
    /// without knowing the exact command.
    OnDemand {
        command: String,
        description: String,
    },
}

/// Broadcast marker for addressee and to-list routing. When a message's
/// `addressee` or `to` list contains this value, the message reaches
/// everyone — it's the "addressed to all" / "visible to all" signal.
pub const BROADCAST_ALL: &str = "all";

fn is_false(b: &bool) -> bool {
    !*b
}

impl MessageType {
    pub fn is_chat(&self) -> bool {
        matches!(self, MessageType::Chat | MessageType::Insight)
    }

    /// Skip-serialization predicate for the wire `message_type` field.
    /// Only the actual default (Chat) is omitted; Insight / Activity /
    /// System / Nudge round-trip explicitly so the renderer can
    /// distinguish them at the message-class layer (#492 — without
    /// this, Insight messages reached the renderer with no type
    /// discriminator and rendered as Chat with no class-glyph).
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
            Some(BROADCAST_ALL) => true,
            Some(target) => target == role,
        }
    }
}
