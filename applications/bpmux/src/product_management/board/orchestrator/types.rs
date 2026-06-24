use std::future::Future;
use std::pin::Pin;

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::product_management::board::events::{BoardEvent, EventMeta};
use crate::product_management::board::peer::PeerId;
use crate::product_management::board::sync::SyncError;
use crate::product_management::story::Story;

use super::ConflictKind;

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
    RolledBack { server_story: Option<Box<Story>> },
    TransientError(SyncError),
    NotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEvent {
    #[serde(default)]
    pub id: MutationId,
    pub event: BoardEvent,
    #[serde(default)]
    pub pre_state: Option<Story>,
    #[serde(default)]
    pub last_classification: Option<ConflictKind>,
}

impl QueuedEvent {
    pub fn new(event: BoardEvent, pre_state: Option<Story>) -> Self {
        Self {
            id: MutationId::default(),
            event,
            pre_state,
            last_classification: None,
        }
    }

    pub fn new_with_id(id: MutationId, event: BoardEvent, pre_state: Option<Story>) -> Self {
        Self {
            id,
            event,
            pre_state,
            last_classification: None,
        }
    }

    pub fn from_legacy(event: BoardEvent) -> Self {
        Self {
            id: MutationId::default(),
            event,
            pre_state: None,
            last_classification: None,
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
