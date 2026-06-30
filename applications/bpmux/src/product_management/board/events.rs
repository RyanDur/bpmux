use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::peer::PeerId;
use crate::product_management::epic::EpicId;
use crate::product_management::iteration::IterationId;
use crate::product_management::story::{
    Acceptor, FreeComment, Gate, GateComment, Story, StoryId, StoryState, StoryType, Tag,
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
    pub iteration_membership: Option<(IterationId, u32)>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub ready_order: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoardEvent {
    StoryCreated {
        meta: EventMeta,
        id: StoryId,
        title: String,
        body: String,
        initial_state: StoryState,
        story_type: StoryType,
    },
    StateTransitioned {
        meta: EventMeta,
        id: StoryId,
        new_state: StoryState,
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
    ParentEpicSet {
        meta: EventMeta,
        id: StoryId,
        epic_id: EpicId,
    },
    ParentEpicUnset {
        meta: EventMeta,
        id: StoryId,
    },
    IterationMembershipSet {
        meta: EventMeta,
        id: StoryId,
        iteration_id: IterationId,
        position: u32,
    },
    IterationMembershipCleared {
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
        kind: super::orchestrator::ConflictKind,
    },
    ConflictResolved {
        meta: EventMeta,
        id: StoryId,
        origin: super::orchestrator::ConflictResolveOrigin,
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
            | BoardEvent::IterationMembershipSet { meta, .. }
            | BoardEvent::IterationMembershipCleared { meta, .. }
            | BoardEvent::CurrentIterationDesignated { meta, .. }
            | BoardEvent::CurrentIterationCleared { meta, .. }
            | BoardEvent::StoryRefreshed { meta, .. }
            | BoardEvent::LabelsRefreshed { meta, .. }
            | BoardEvent::TitleUpdated { meta, .. }
            | BoardEvent::BodyUpdated { meta, .. }
            | BoardEvent::ConflictDetected { meta, .. }
            | BoardEvent::ConflictResolved { meta, .. } => meta,
        }
    }

    pub fn story_id(&self) -> StoryId {
        match self {
            BoardEvent::StoryCreated { id, .. }
            | BoardEvent::StateTransitioned { id, .. }
            | BoardEvent::GateCommentAppended { id, .. }
            | BoardEvent::FreeCommentAppended { id, .. }
            | BoardEvent::BodyFetched { id, .. }
            | BoardEvent::FreeCommentsFetched { id, .. }
            | BoardEvent::GateCommentsFetched { id, .. }
            | BoardEvent::TagAdded { id, .. }
            | BoardEvent::TagRemoved { id, .. }
            | BoardEvent::StoryClosed { id, .. }
            | BoardEvent::StoryReopened { id, .. }
            | BoardEvent::ParentEpicSet { id, .. }
            | BoardEvent::ParentEpicUnset { id, .. }
            | BoardEvent::IterationMembershipSet { id, .. }
            | BoardEvent::IterationMembershipCleared { id, .. }
            | BoardEvent::CurrentIterationDesignated { id, .. }
            | BoardEvent::CurrentIterationCleared { id, .. }
            | BoardEvent::StoryRefreshed { id, .. }
            | BoardEvent::LabelsRefreshed { id, .. }
            | BoardEvent::TitleUpdated { id, .. }
            | BoardEvent::BodyUpdated { id, .. }
            | BoardEvent::ConflictDetected { id, .. }
            | BoardEvent::ConflictResolved { id, .. } => *id,
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
