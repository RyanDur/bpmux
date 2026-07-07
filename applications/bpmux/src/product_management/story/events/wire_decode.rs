use super::StoryEvent;
use super::kind::StoryEventKind;
use super::wire::StoryEventWire;

impl From<StoryEventWire> for StoryEvent {
    fn from(wire: StoryEventWire) -> StoryEvent {
        match wire {
            StoryEventWire::StoryCreated {
                meta,
                id,
                title,
                body,
                initial_state,
                story_type,
            } => StoryEvent::new(
                meta,
                id,
                StoryEventKind::StoryCreated {
                    title,
                    body,
                    initial_state,
                    story_type,
                },
            ),
            StoryEventWire::StateTransitioned {
                meta,
                id,
                new_state,
                placement,
            } => StoryEvent::new(
                meta,
                id,
                StoryEventKind::StateTransitioned {
                    new_state,
                    placement,
                },
            ),
            StoryEventWire::TagAdded { meta, id, label } => {
                StoryEvent::new(meta, id, StoryEventKind::TagAdded { label })
            }
            StoryEventWire::TagRemoved { meta, id, label } => {
                StoryEvent::new(meta, id, StoryEventKind::TagRemoved { label })
            }
            StoryEventWire::TagSet { meta, id, tag } => {
                StoryEvent::new(meta, id, StoryEventKind::TagSet { tag })
            }
            StoryEventWire::TagUnset { meta, id, tag } => {
                StoryEvent::new(meta, id, StoryEventKind::TagUnset { tag })
            }
            StoryEventWire::IterationSlotSet {
                meta,
                id,
                name,
                position,
            } => StoryEvent::new(
                meta,
                id,
                StoryEventKind::IterationSlotSet { name, position },
            ),
            StoryEventWire::IterationSlotCleared { meta, id } => {
                StoryEvent::new(meta, id, StoryEventKind::IterationSlotCleared)
            }
            StoryEventWire::VerdictCast {
                meta,
                id,
                verdict,
                payload,
            } => StoryEvent::new(meta, id, StoryEventKind::VerdictCast { verdict, payload }),
            StoryEventWire::FreeCommentAppended { meta, id, body } => {
                StoryEvent::new(meta, id, StoryEventKind::FreeCommentAppended { body })
            }
            StoryEventWire::BodyFetched {
                meta,
                id,
                body,
                etag,
            } => StoryEvent::new(meta, id, StoryEventKind::BodyFetched { body, etag }),
            StoryEventWire::FreeCommentsFetched { meta, id, comments } => {
                StoryEvent::new(meta, id, StoryEventKind::FreeCommentsFetched { comments })
            }
            StoryEventWire::VerdictsFetched { meta, id, verdicts } => {
                StoryEvent::new(meta, id, StoryEventKind::VerdictsFetched { verdicts })
            }
            StoryEventWire::StoryClosed { meta, id, reason } => {
                StoryEvent::new(meta, id, StoryEventKind::StoryClosed { reason })
            }
            StoryEventWire::StoryReopened { meta, id } => {
                StoryEvent::new(meta, id, StoryEventKind::StoryReopened)
            }
            StoryEventWire::AcceptorAssigned { meta, id, acceptor } => {
                StoryEvent::new(meta, id, StoryEventKind::AcceptorAssigned { acceptor })
            }
            StoryEventWire::ParentEpicSet { meta, id, epic_id } => {
                StoryEvent::new(meta, id, StoryEventKind::ParentEpicSet { epic_id })
            }
            StoryEventWire::ParentEpicUnset { meta, id } => {
                StoryEvent::new(meta, id, StoryEventKind::ParentEpicUnset)
            }
            StoryEventWire::CurrentIterationDesignated { meta, id } => {
                StoryEvent::new(meta, id, StoryEventKind::CurrentIterationDesignated)
            }
            StoryEventWire::CurrentIterationCleared { meta, id } => {
                StoryEvent::new(meta, id, StoryEventKind::CurrentIterationCleared)
            }
            StoryEventWire::StoryRefreshed { meta, id, story } => {
                StoryEvent::new(meta, id, StoryEventKind::StoryRefreshed { story })
            }
            StoryEventWire::LabelsRefreshed { meta, id, fields } => {
                StoryEvent::new(meta, id, StoryEventKind::LabelsRefreshed { fields })
            }
            StoryEventWire::TitleUpdated { meta, id, title } => {
                StoryEvent::new(meta, id, StoryEventKind::TitleUpdated { title })
            }
            StoryEventWire::BodyUpdated { meta, id, body } => {
                StoryEvent::new(meta, id, StoryEventKind::BodyUpdated { body })
            }
            StoryEventWire::ConflictDetected { meta, id, kind } => {
                StoryEvent::new(meta, id, StoryEventKind::ConflictDetected { kind })
            }
            StoryEventWire::ConflictResolved { meta, id, origin } => {
                StoryEvent::new(meta, id, StoryEventKind::ConflictResolved { origin })
            }
        }
    }
}
