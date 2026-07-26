use std::collections::VecDeque;
use std::sync::Arc;

use crate::{OutboxRepo, QueuedEvent, db};
use database_support::DatabaseGateway;
use persist::PersistError;

pub struct OutboxDataGateway {
    gateway: Arc<DatabaseGateway>,
}

impl OutboxDataGateway {
    pub fn new(gateway: Arc<DatabaseGateway>) -> Self {
        Self { gateway }
    }
}

impl OutboxRepo for OutboxDataGateway {
    fn load_outbox(&self) -> Result<VecDeque<QueuedEvent>, PersistError> {
        db::load(&self.gateway.lock())
    }

    fn save_outbox(&self, events: &[QueuedEvent]) -> Result<(), PersistError> {
        db::replace(&self.gateway.lock(), events)
    }
}
