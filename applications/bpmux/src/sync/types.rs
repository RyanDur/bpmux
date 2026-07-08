use std::future::Future;
use std::pin::Pin;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::product_management::story::Story;
use crate::product_management::story::events::{EventMeta, StoryEvent};
use crate::product_management::story::peer::PeerId;
use crate::sync::SyncError;

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

#[derive(Debug)]
pub enum DrainOneOutcome {
    Confirmed,
    TransientError(SyncError),
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    pub id: MutationId,
    pub event: StoryEvent,
    pub pre_state: Option<Story>,
}

impl QueuedEvent {
    pub fn new(event: StoryEvent, pre_state: Option<Story>) -> Self {
        Self {
            id: MutationId::default(),
            event,
            pre_state,
        }
    }

    pub fn new_with_id(id: MutationId, event: StoryEvent, pre_state: Option<Story>) -> Self {
        Self {
            id,
            event,
            pre_state,
        }
    }
}

pub(crate) fn cache_fill_meta() -> EventMeta {
    EventMeta {
        actor: "source-ingest".to_string(),
        ts: Utc::now(),
        peer_id: PeerId::local(),
    }
}
