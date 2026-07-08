use super::StoryEvent;
use super::kind::StoryEventKind;
use super::wire::StoryEventWire;

impl From<StoryEvent> for StoryEventWire {
    fn from(event: StoryEvent) -> StoryEventWire {
        let StoryEvent { meta, id, kind } = event;
        match kind {
            StoryEventKind::StoryCreated {
                title,
                body,
                initial_state,
                story_type,
            } => StoryEventWire::StoryCreated {
                meta,
                id,
                title,
                body,
                initial_state,
                story_type,
            },
            StoryEventKind::StateTransitioned {
                new_state,
                placement,
            } => StoryEventWire::StateTransitioned {
                meta,
                id,
                new_state,
                placement,
            },
            StoryEventKind::TagAdded { label } => StoryEventWire::TagAdded { meta, id, label },
            StoryEventKind::TagRemoved { label } => StoryEventWire::TagRemoved { meta, id, label },
            StoryEventKind::TagSet { tag } => StoryEventWire::TagSet { meta, id, tag },
            StoryEventKind::TagUnset { tag } => StoryEventWire::TagUnset { meta, id, tag },
            StoryEventKind::VerdictCast { verdict, payload } => StoryEventWire::VerdictCast {
                meta,
                id,
                verdict,
                payload,
            },
            StoryEventKind::FreeCommentAppended { body } => {
                StoryEventWire::FreeCommentAppended { meta, id, body }
            }
            StoryEventKind::BodyFetched { body, etag } => StoryEventWire::BodyFetched {
                meta,
                id,
                body,
                etag,
            },
            StoryEventKind::FreeCommentsFetched { comments } => {
                StoryEventWire::FreeCommentsFetched { meta, id, comments }
            }
            StoryEventKind::VerdictsFetched { verdicts } => {
                StoryEventWire::VerdictsFetched { meta, id, verdicts }
            }
            StoryEventKind::StoryClosed { reason } => {
                StoryEventWire::StoryClosed { meta, id, reason }
            }
            StoryEventKind::StoryReopened => StoryEventWire::StoryReopened { meta, id },
            StoryEventKind::AcceptorAssigned { acceptor } => {
                StoryEventWire::AcceptorAssigned { meta, id, acceptor }
            }
            StoryEventKind::ParentEpicSet { epic_id } => {
                StoryEventWire::ParentEpicSet { meta, id, epic_id }
            }
            StoryEventKind::ParentEpicUnset => StoryEventWire::ParentEpicUnset { meta, id },
            StoryEventKind::CurrentIterationDesignated => {
                StoryEventWire::CurrentIterationDesignated { meta, id }
            }
            StoryEventKind::CurrentIterationCleared => {
                StoryEventWire::CurrentIterationCleared { meta, id }
            }
            StoryEventKind::StoryRefreshed { story } => {
                StoryEventWire::StoryRefreshed { meta, id, story }
            }
            StoryEventKind::LabelsRefreshed { fields } => {
                StoryEventWire::LabelsRefreshed { meta, id, fields }
            }
            StoryEventKind::TitleUpdated { title } => {
                StoryEventWire::TitleUpdated { meta, id, title }
            }
            StoryEventKind::BodyUpdated { body } => StoryEventWire::BodyUpdated { meta, id, body },
            StoryEventKind::ConflictDetected { kind } => {
                StoryEventWire::ConflictDetected { meta, id, kind }
            }
            StoryEventKind::ConflictResolved { origin } => {
                StoryEventWire::ConflictResolved { meta, id, origin }
            }
        }
    }
}
