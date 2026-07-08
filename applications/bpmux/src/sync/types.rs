use std::future::Future;
use std::pin::Pin;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::product_management::story::events::{EventMeta, StoryEvent};
use crate::product_management::story::peer::PeerId;

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MutationId(pub u64);

impl std::fmt::Display for MutationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

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

pub(crate) fn cache_fill_meta() -> EventMeta {
    EventMeta {
        actor: "source-ingest".to_string(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
    }
}
