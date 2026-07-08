use serde::Deserialize;

use super::StoryEvent;
use super::kind::StoryEventKind;
use super::wire::StoryEventWire;
use crate::product_management::story::StoryId;

pub(crate) struct DroppedLegacyEvent {
    id: StoryId,
    variant: &'static str,
}

impl std::fmt::Display for DroppedLegacyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "outbox wire boundary dropped a retired {} event for story #{} (iteration_slot is retired)",
            self.variant,
            self.id.value()
        )
    }
}

impl TryFrom<StoryEventWire> for StoryEvent {
    type Error = DroppedLegacyEvent;

    fn try_from(wire: StoryEventWire) -> Result<StoryEvent, DroppedLegacyEvent> {
        let event = match wire {
            StoryEventWire::IterationSlotSet { id, .. } => {
                return Err(DroppedLegacyEvent {
                    id,
                    variant: "IterationSlotSet",
                });
            }
            StoryEventWire::IterationSlotCleared { id, .. } => {
                return Err(DroppedLegacyEvent {
                    id,
                    variant: "IterationSlotCleared",
                });
            }
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
        };
        Ok(event)
    }
}

#[derive(Deserialize)]
#[serde(from = "StoryEventWire")]
pub(crate) struct TolerantStoryEvent(pub(crate) Option<StoryEvent>);

impl From<StoryEventWire> for TolerantStoryEvent {
    fn from(wire: StoryEventWire) -> TolerantStoryEvent {
        match StoryEvent::try_from(wire) {
            Ok(event) => TolerantStoryEvent(Some(event)),
            Err(dropped) => {
                eprintln!("{dropped}");
                TolerantStoryEvent(None)
            }
        }
    }
}
