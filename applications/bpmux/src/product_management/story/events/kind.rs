use crate::product_management::epic::EpicId;
use crate::product_management::story::{
    Acceptor, CastVerdict, FreeComment, Story, StoryKind, Tag, Verdict, Work,
};
use workflow::StageId;

use super::LabelDerivedFields;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoryEventKind {
    StoryCreated {
        title: String,
        body: String,
        initial_state: Work,
        story_type: StoryKind,
    },
    StateTransitioned {
        new_state: Work,
        placement: Option<StageId>,
    },
    TagAdded {
        label: String,
    },
    TagRemoved {
        label: String,
    },
    TagSet {
        tag: Tag,
    },
    TagUnset {
        tag: Tag,
    },
    VerdictCast {
        verdict: Verdict,
        payload: Option<String>,
    },
    FreeCommentAppended {
        body: String,
    },
    BodyFetched {
        body: String,
        etag: Option<String>,
    },
    FreeCommentsFetched {
        comments: Vec<FreeComment>,
    },
    VerdictsFetched {
        verdicts: Vec<CastVerdict>,
    },
    StoryClosed {
        reason: Option<String>,
    },
    StoryReopened,
    AcceptorAssigned {
        acceptor: Acceptor,
    },
    ParentEpicSet {
        epic_id: EpicId,
    },
    ParentEpicUnset,
    CurrentIterationDesignated,
    CurrentIterationCleared,
    StoryRefreshed {
        story: Box<Story>,
    },
    LabelsRefreshed {
        fields: LabelDerivedFields,
    },
    TitleUpdated {
        title: String,
    },
    BodyUpdated {
        body: String,
    },
}
