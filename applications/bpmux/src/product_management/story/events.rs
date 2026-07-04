use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::product_management::epic::EpicId;
use crate::product_management::story::peer::PeerId;
use crate::product_management::story::{
    Acceptor, BoardState, FreeComment, Gate, GateComment, Story, StoryId, StoryKind, Tag,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventMeta {
    pub actor: String,
    pub ts: DateTime<Utc>,
    pub peer_id: PeerId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LabelDerivedFields {
    pub raw_labels: Vec<String>,
    #[serde(default)]
    pub acceptor: Option<Acceptor>,
    #[serde(default)]
    pub parent_epic: Option<EpicId>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub blocked_by: Option<StoryId>,
    #[serde(default)]
    pub ready_order: Option<u32>,
    #[serde(default)]
    pub iteration_slot: Option<crate::product_management::story::IterationSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoryEvent {
    StoryCreated {
        meta: EventMeta,
        id: StoryId,
        title: String,
        body: String,
        initial_state: BoardState,
        story_type: StoryKind,
    },
    StateTransitioned {
        meta: EventMeta,
        id: StoryId,
        new_state: BoardState,
    },
    TagAdded {
        meta: EventMeta,
        id: StoryId,
        label: String,
    },
    TagRemoved {
        meta: EventMeta,
        id: StoryId,
        label: String,
    },
    TagSet {
        meta: EventMeta,
        id: StoryId,
        tag: Tag,
    },
    TagUnset {
        meta: EventMeta,
        id: StoryId,
        tag: Tag,
    },
    BlockedBySet {
        meta: EventMeta,
        id: StoryId,
        blocker: StoryId,
    },
    BlockedByCleared {
        meta: EventMeta,
        id: StoryId,
    },
    IterationSlotSet {
        meta: EventMeta,
        id: StoryId,
        name: String,
        position: u32,
    },
    IterationSlotCleared {
        meta: EventMeta,
        id: StoryId,
    },
    GateCommentAppended {
        meta: EventMeta,
        id: StoryId,
        gate: Gate,
        payload: Option<String>,
    },
    FreeCommentAppended {
        meta: EventMeta,
        id: StoryId,
        body: String,
    },
    BodyFetched {
        meta: EventMeta,
        id: StoryId,
        body: String,
        #[serde(default)]
        etag: Option<String>,
    },
    FreeCommentsFetched {
        meta: EventMeta,
        id: StoryId,
        comments: Vec<FreeComment>,
    },
    GateCommentsFetched {
        meta: EventMeta,
        id: StoryId,
        comments: Vec<GateComment>,
    },
    StoryClosed {
        meta: EventMeta,
        id: StoryId,
        reason: Option<String>,
    },
    StoryReopened {
        meta: EventMeta,
        id: StoryId,
    },
    AcceptorAssigned {
        meta: EventMeta,
        id: StoryId,
        acceptor: Acceptor,
    },
    ParentEpicSet {
        meta: EventMeta,
        id: StoryId,
        epic_id: EpicId,
    },
    ParentEpicUnset {
        meta: EventMeta,
        id: StoryId,
    },
    CurrentIterationDesignated {
        meta: EventMeta,
        id: StoryId,
    },
    CurrentIterationCleared {
        meta: EventMeta,
        id: StoryId,
    },
    StoryRefreshed {
        meta: EventMeta,
        id: StoryId,
        story: Box<Story>,
    },
    LabelsRefreshed {
        meta: EventMeta,
        id: StoryId,
        fields: LabelDerivedFields,
    },
    TitleUpdated {
        meta: EventMeta,
        id: StoryId,
        title: String,
    },
    BodyUpdated {
        meta: EventMeta,
        id: StoryId,
        body: String,
    },
    ConflictDetected {
        meta: EventMeta,
        id: StoryId,
        kind: super::conflict::ConflictKind,
    },
    ConflictResolved {
        meta: EventMeta,
        id: StoryId,
        origin: super::conflict::ConflictResolveOrigin,
    },
}

impl StoryEvent {
    pub fn meta(&self) -> &EventMeta {
        match self {
            StoryEvent::StoryCreated { meta, .. }
            | StoryEvent::StateTransitioned { meta, .. }
            | StoryEvent::GateCommentAppended { meta, .. }
            | StoryEvent::FreeCommentAppended { meta, .. }
            | StoryEvent::BodyFetched { meta, .. }
            | StoryEvent::FreeCommentsFetched { meta, .. }
            | StoryEvent::GateCommentsFetched { meta, .. }
            | StoryEvent::TagAdded { meta, .. }
            | StoryEvent::TagRemoved { meta, .. }
            | StoryEvent::TagSet { meta, .. }
            | StoryEvent::TagUnset { meta, .. }
            | StoryEvent::BlockedBySet { meta, .. }
            | StoryEvent::BlockedByCleared { meta, .. }
            | StoryEvent::IterationSlotSet { meta, .. }
            | StoryEvent::IterationSlotCleared { meta, .. }
            | StoryEvent::StoryClosed { meta, .. }
            | StoryEvent::StoryReopened { meta, .. }
            | StoryEvent::AcceptorAssigned { meta, .. }
            | StoryEvent::ParentEpicSet { meta, .. }
            | StoryEvent::ParentEpicUnset { meta, .. }
            | StoryEvent::CurrentIterationDesignated { meta, .. }
            | StoryEvent::CurrentIterationCleared { meta, .. }
            | StoryEvent::StoryRefreshed { meta, .. }
            | StoryEvent::LabelsRefreshed { meta, .. }
            | StoryEvent::TitleUpdated { meta, .. }
            | StoryEvent::BodyUpdated { meta, .. }
            | StoryEvent::ConflictDetected { meta, .. }
            | StoryEvent::ConflictResolved { meta, .. } => meta,
        }
    }

    pub fn story_id(&self) -> StoryId {
        match self {
            StoryEvent::StoryCreated { id, .. }
            | StoryEvent::StateTransitioned { id, .. }
            | StoryEvent::GateCommentAppended { id, .. }
            | StoryEvent::FreeCommentAppended { id, .. }
            | StoryEvent::BodyFetched { id, .. }
            | StoryEvent::FreeCommentsFetched { id, .. }
            | StoryEvent::GateCommentsFetched { id, .. }
            | StoryEvent::TagAdded { id, .. }
            | StoryEvent::TagRemoved { id, .. }
            | StoryEvent::TagSet { id, .. }
            | StoryEvent::TagUnset { id, .. }
            | StoryEvent::BlockedBySet { id, .. }
            | StoryEvent::BlockedByCleared { id, .. }
            | StoryEvent::IterationSlotSet { id, .. }
            | StoryEvent::IterationSlotCleared { id, .. }
            | StoryEvent::StoryClosed { id, .. }
            | StoryEvent::StoryReopened { id, .. }
            | StoryEvent::AcceptorAssigned { id, .. }
            | StoryEvent::ParentEpicSet { id, .. }
            | StoryEvent::ParentEpicUnset { id, .. }
            | StoryEvent::CurrentIterationDesignated { id, .. }
            | StoryEvent::CurrentIterationCleared { id, .. }
            | StoryEvent::StoryRefreshed { id, .. }
            | StoryEvent::LabelsRefreshed { id, .. }
            | StoryEvent::TitleUpdated { id, .. }
            | StoryEvent::BodyUpdated { id, .. }
            | StoryEvent::ConflictDetected { id, .. }
            | StoryEvent::ConflictResolved { id, .. } => *id,
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
