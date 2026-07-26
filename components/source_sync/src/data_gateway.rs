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
        let raws = db::serialize_events(events)?;
        self.gateway
            .with_transaction(|conn| db::replace_rows(conn, &raws))
            .map_err(database_support::persist_err)
    }
}
