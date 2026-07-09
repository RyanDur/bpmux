use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;

use super::cursor_time::from_millis;
use super::identity::RoutingIdentity;
use super::slice::RoutingAction;
use crate::storage::Storage;
use crate::store::Dispatch;

mod mutations;
mod queries;

pub(crate) fn short_name(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub fn bridge_agent_name(terminal_id: &str) -> String {
    let prefix = &terminal_id[..terminal_id.len().min(8)];
    format!("agent-{prefix}")
}

pub struct RoutingStore {
    identity: Mutex<RoutingIdentity>,
    storage: Arc<dyn Storage>,
    dispatch: OnceLock<Dispatch>,
}

impl RoutingStore {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        let mut identity = RoutingIdentity::new();
        if let Ok(records) = storage.load_users() {
            for record in records {
                identity.register(&record.session_id, &record.name);
                if let Some(cursor) = record.last_delivered_at.and_then(from_millis) {
                    identity.advance_cursor(&record.session_id, &cursor);
                }
            }
        }
        Self {
            identity: Mutex::new(identity),
            storage,
            dispatch: OnceLock::new(),
        }
    }

    pub fn bind_dispatch(&self, dispatch: Dispatch) {
        let _ = self.dispatch.set(dispatch);
    }

    fn emit(&self, action: RoutingAction) {
        if let Some(dispatch) = self.dispatch.get() {
            dispatch.dispatch(action.into());
        }
    }
}
