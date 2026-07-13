use serde::{Deserialize, Serialize};

use crate::epic::EpicId;
use crate::story::{
    Acceptor, CastVerdict, FreeComment, Story, StoryId, StoryKind, Tag, Verdict, Work,
};
use workflow::StageId;

use super::LabelDerivedFields;
use super::meta::EventMeta;

#[derive(Serialize, Deserialize)]
pub(super) enum StoryEventWire {
    StoryCreated {
        meta: EventMeta,
        id: StoryId,
        title: String,
        body: String,
        initial_state: Work,
        story_type: StoryKind,
    },
    StateTransitioned {
        meta: EventMeta,
        id: StoryId,
        new_state: Work,
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
}
