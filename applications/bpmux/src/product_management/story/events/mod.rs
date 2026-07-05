use serde::{Deserialize, Serialize};

use crate::product_management::epic::EpicId;
use crate::product_management::story::{
    Acceptor, BoardState, CastVerdict, FreeComment, Story, StoryId, StoryKind, Tag, Verdict,
};
use crate::workflow::StageId;

mod accessors;
mod constructors;
mod label_fields;
mod meta;

pub use label_fields::LabelDerivedFields;
pub use meta::{EventMeta, event_wins, resolve_order_key};

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
        #[serde(default)]
        placement: Option<StageId>,
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
    VerdictCast {
        meta: EventMeta,
        id: StoryId,
        verdict: Verdict,
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
    VerdictsFetched {
        meta: EventMeta,
        id: StoryId,
        verdicts: Vec<CastVerdict>,
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
