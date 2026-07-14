use serde::{Deserialize, Serialize};

use product_management::story::events::{StoryEvent, StoryEventKind};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MutationId(pub u64);

impl std::fmt::Display for MutationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    pub id: MutationId,
    pub event: StoryEvent,
}

impl QueuedEvent {
    pub fn new(event: StoryEvent) -> Self {
        Self {
            id: MutationId::default(),
            event,
        }
    }

    pub fn new_with_id(id: MutationId, event: StoryEvent) -> Self {
        Self { id, event }
    }
}

#[derive(Debug, Clone)]
pub struct PendingWrite {
    pub event: StoryEvent,
    pub iteration_class: Option<String>,
}

impl PendingWrite {
    pub fn from_event(
        codec: &dyn product_management::label_codec::LabelCodec,
        event: StoryEvent,
    ) -> Self {
        let iteration_class = match &event.kind {
            StoryEventKind::TagAdded { label } | StoryEventKind::TagRemoved { label } => {
                codec.iteration_label_class(label)
            }
            _ => None,
        };
        Self {
            event,
            iteration_class,
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueuedWrite {
    pub id: MutationId,
    pub event: StoryEvent,
    pub iteration_class: Option<String>,
}

impl QueuedWrite {
    pub fn minted(id: MutationId, pending: PendingWrite) -> Self {
        Self {
            id,
            event: pending.event,
            iteration_class: pending.iteration_class,
        }
    }
}
