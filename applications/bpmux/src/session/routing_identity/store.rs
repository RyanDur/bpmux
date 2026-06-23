use std::sync::Arc;
use std::sync::Mutex;

use super::cursor_time::{cursor_after, is_future, later_cursor};
use super::identity::RoutingIdentity;
use crate::session::storage::Storage;

fn short(identity: &str) -> &str {
    identity.split(':').next().unwrap_or(identity)
}

pub fn bridge_agent_name(terminal_id: &str) -> String {
    let prefix = &terminal_id[..terminal_id.len().min(8)];
    format!("agent-{prefix}")
}

pub struct RoutingStore {
    identity: Mutex<RoutingIdentity>,
    storage: Arc<dyn Storage>,
}

impl RoutingStore {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        let mut identity = RoutingIdentity::new();
        if let Ok(records) = storage.load_agents() {
            for record in records {
                identity.register(&record.id, &record.name);
                if let Some(cursor) = record.cursor {
                    identity.advance_cursor(&record.id, &cursor);
                }
            }
        }
        Self {
            identity: Mutex::new(identity),
            storage,
        }
    }

    pub fn register(&self, session_id: &str, display: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.register(session_id, display);
        let _ = self
            .storage
            .upsert_agent(session_id, display, short(display));
    }

    pub fn link_terminal(&self, session_id: &str, terminal_id: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.link_terminal(session_id, terminal_id);
    }

    pub fn is_terminal_linked(&self, terminal_id: &str) -> bool {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.is_terminal_linked(terminal_id)
    }

    pub fn terminal_for(&self, session_id: &str) -> Option<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.terminal_for(session_id)
    }

    pub fn display_for_terminal(&self, terminal_id: &str) -> Option<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.display_for_terminal(terminal_id)
    }

    pub fn display_for(&self, session_id: &str) -> Option<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.display_for(session_id)
    }

    pub fn session_for_terminal(&self, terminal_id: &str) -> Option<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.session_for_terminal(terminal_id)
    }

    pub fn reap_unclaimed_duplicates_of(&self, display: &str) -> Vec<String> {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let reaped = identity.reap_unclaimed_duplicates_of(display);
        if !reaped.is_empty() {
            let _ = self.storage.delete_agents(&reaped);
        }
        reaped
    }

    pub fn resolve(&self, display: &str) -> Vec<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.resolve(display)
    }

    pub fn rename(&self, session_id: &str, new_display: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.rename(session_id, new_display);
        let _ = self
            .storage
            .upsert_agent(session_id, new_display, short(new_display));
    }

    pub fn drain_floor(&self, session_id: &str, client_ack: Option<&str>) -> Option<String> {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let stored = identity.cursor(session_id);
        let trusted_ack = client_ack.filter(|ack| !is_future(ack));
        let floor = later_cursor(stored.as_deref(), trusted_ack);
        if let Some(ref position) = floor {
            identity.advance_cursor(session_id, position);
            let _ = self.storage.set_agent_cursor(session_id, position);
        }
        floor
    }

    pub fn cursor(&self, session_id: &str) -> Option<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.cursor(session_id)
    }

    pub fn mark_delivered(&self, session_id: &str, ts: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let current = identity.cursor(session_id);
        if let Some(later) = later_cursor(current.as_deref(), Some(ts))
            && current.as_deref() != Some(later.as_str())
        {
            identity.advance_cursor(session_id, &later);
            let _ = self.storage.set_agent_cursor(session_id, &later);
        }
    }

    pub fn delivery_targets(&self, to: Option<&[String]>, sender: &str) -> Vec<(String, String)> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let sender_ids = identity.resolve(short(sender));
        let recipient_ids: Vec<String> = match to {
            None => identity.all_ids(),
            Some(names)
                if names
                    .iter()
                    .any(|n| n == crate::session::message::BROADCAST_ALL) =>
            {
                identity.all_ids()
            }
            Some(names) => names
                .iter()
                .flat_map(|name| identity.resolve(short(name)))
                .collect(),
        };
        recipient_ids
            .into_iter()
            .filter(|id| !sender_ids.contains(id))
            .filter_map(|id| identity.terminal_for(&id).map(|terminal| (id, terminal)))
            .filter(|(_, terminal)| bridge_agent_name(terminal) != short(sender))
            .collect()
    }

    pub fn register_agent(
        &self,
        session_id: &str,
        display: &str,
        terminal_id: &str,
        job_start: &str,
    ) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let stored = identity.cursor(session_id);
        let fresh = stored.is_none();
        if fresh {
            identity.register(session_id, display);
        }
        if fresh || cursor_after(stored.as_deref(), job_start) {
            identity.advance_cursor(session_id, job_start);
        }
        identity.link_terminal(session_id, terminal_id);
        let reaped = identity.reap_unclaimed_duplicates_of(display);
        if let Some(current) = identity.display_for(session_id) {
            let _ = self
                .storage
                .upsert_agent(session_id, &current, short(&current));
        }
        if let Some(cursor) = identity.cursor(session_id) {
            let _ = self.storage.set_agent_cursor(session_id, &cursor);
        }
        if !reaped.is_empty() {
            let _ = self.storage.delete_agents(&reaped);
        }
    }
}
