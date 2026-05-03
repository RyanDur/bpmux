use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::domain::{FreeComment, Gate, GateComment, Story, StoryState, StoryType};
use super::peer::PeerId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMeta {
    pub actor: String,
    pub ts: DateTime<Utc>,
    pub peer_id: PeerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoardEvent {
    StoryCreated {
        meta: EventMeta,
        number: u64,
        title: String,
        body: String,
        initial_state: StoryState,
        story_type: StoryType,
    },
    StateTransitioned {
        meta: EventMeta,
        number: u64,
        new_state: StoryState,
    },
    TagAdded {
        meta: EventMeta,
        number: u64,
        label: String,
    },
    TagRemoved {
        meta: EventMeta,
        number: u64,
        label: String,
    },
    GateCommentAppended {
        meta: EventMeta,
        number: u64,
        gate: Gate,
        payload: Option<String>,
    },
    FreeCommentAppended {
        meta: EventMeta,
        number: u64,
        body: String,
    },
    BodyFetched {
        meta: EventMeta,
        number: u64,
        body: String,
    },
    FreeCommentsFetched {
        meta: EventMeta,
        number: u64,
        comments: Vec<FreeComment>,
    },
    GateCommentsFetched {
        meta: EventMeta,
        number: u64,
        comments: Vec<GateComment>,
    },
    StoryClosed {
        meta: EventMeta,
        number: u64,
        reason: Option<String>,
    },
    StoryReopened {
        meta: EventMeta,
        number: u64,
    },
    ParentEpicSet {
        meta: EventMeta,
        number: u64,
        epic_number: u64,
    },
    ParentEpicUnset {
        meta: EventMeta,
        number: u64,
    },
    /// #1151: GH-API mutation response captured. The tracker's
    /// underlying GH endpoint returned the post-mutation Issue; the
    /// adapter parsed it into a `Story`; the orchestrator dispatches
    /// this event so the canonical store reflects the GH-side truth
    /// without a follow-up `refresh_story(N)` call.
    StoryRefreshed {
        meta: EventMeta,
        number: u64,
        story: Story,
    },
    /// #1151: GH-API label-mutation response captured. octocrab's
    /// `add_labels` / `remove_label` return the post-mutation labels
    /// list; the orchestrator dispatches this event so the reducer
    /// can re-derive the local Story's `state` via `state_from_labels`
    /// and refresh `tags` without a follow-up `refresh_story(N)` call.
    LabelsRefreshed {
        meta: EventMeta,
        number: u64,
        labels: Vec<String>,
    },
}

impl BoardEvent {
    pub fn meta(&self) -> &EventMeta {
        match self {
            BoardEvent::StoryCreated { meta, .. }
            | BoardEvent::StateTransitioned { meta, .. }
            | BoardEvent::GateCommentAppended { meta, .. }
            | BoardEvent::FreeCommentAppended { meta, .. }
            | BoardEvent::BodyFetched { meta, .. }
            | BoardEvent::FreeCommentsFetched { meta, .. }
            | BoardEvent::GateCommentsFetched { meta, .. }
            | BoardEvent::TagAdded { meta, .. }
            | BoardEvent::TagRemoved { meta, .. }
            | BoardEvent::StoryClosed { meta, .. }
            | BoardEvent::StoryReopened { meta, .. }
            | BoardEvent::ParentEpicSet { meta, .. }
            | BoardEvent::ParentEpicUnset { meta, .. }
            | BoardEvent::StoryRefreshed { meta, .. }
            | BoardEvent::LabelsRefreshed { meta, .. } => meta,
        }
    }

    pub fn story_number(&self) -> u64 {
        match self {
            BoardEvent::StoryCreated { number, .. }
            | BoardEvent::StateTransitioned { number, .. }
            | BoardEvent::GateCommentAppended { number, .. }
            | BoardEvent::FreeCommentAppended { number, .. }
            | BoardEvent::BodyFetched { number, .. }
            | BoardEvent::FreeCommentsFetched { number, .. }
            | BoardEvent::GateCommentsFetched { number, .. }
            | BoardEvent::TagAdded { number, .. }
            | BoardEvent::TagRemoved { number, .. }
            | BoardEvent::StoryClosed { number, .. }
            | BoardEvent::StoryReopened { number, .. }
            | BoardEvent::ParentEpicSet { number, .. }
            | BoardEvent::ParentEpicUnset { number, .. }
            | BoardEvent::StoryRefreshed { number, .. }
            | BoardEvent::LabelsRefreshed { number, .. } => *number,
        }
    }
}

pub fn resolve_order_key(meta: &EventMeta) -> (DateTime<Utc>, String) {
    (meta.ts, meta.peer_id.as_str().to_string())
}

pub fn event_wins(a: &EventMeta, b: &EventMeta) -> bool {
    resolve_order_key(a) > resolve_order_key(b)
}

impl GateComment {
    pub fn from_event(gate: Gate, meta: &EventMeta, payload: Option<String>) -> Self {
        Self {
            gate,
            ts: meta.ts,
            actor: meta.actor.clone(),
            payload,
        }
    }
}

impl FreeComment {
    pub fn from_event(body: String, meta: &EventMeta) -> Self {
        Self {
            body,
            author: meta.actor.clone(),
            ts: meta.ts,
        }
    }
}
