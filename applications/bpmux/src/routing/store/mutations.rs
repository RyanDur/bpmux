use super::{RoutingStore, short_name};
use crate::routing::cursor_time::{cursor_after, is_future, later_cursor, to_millis};
use crate::routing::slice::RoutingAction;

impl RoutingStore {
    pub fn register(&self, session_id: &str, display: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.register(session_id, display);
        let _ = self
            .storage
            .upsert_user(session_id, display, short_name(display));
        self.emit(RoutingAction::Registered {
            session_id: session_id.to_string(),
            display: display.to_string(),
        });
    }

    pub fn link_terminal(&self, session_id: &str, terminal_id: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.link_terminal(session_id, terminal_id);
        self.emit(RoutingAction::TerminalLinked {
            session_id: session_id.to_string(),
            terminal_id: terminal_id.to_string(),
        });
    }

    pub fn reap_unclaimed_duplicates_of(&self, display: &str) -> Vec<String> {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let reaped = identity.reap_unclaimed_duplicates_of(display);
        if !reaped.is_empty() {
            let _ = self.storage.delete_users(&reaped);
        }
        self.emit(RoutingAction::Reaped {
            display: display.to_string(),
        });
        reaped
    }

    pub fn rename(&self, session_id: &str, new_display: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.rename(session_id, new_display);
        let _ = self
            .storage
            .upsert_user(session_id, new_display, short_name(new_display));
        self.emit(RoutingAction::Renamed {
            session_id: session_id.to_string(),
            display: new_display.to_string(),
        });
    }

    pub fn drain_floor(&self, session_id: &str, client_ack: Option<&str>) -> Option<String> {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let stored = identity.cursor(session_id);
        let trusted_ack = client_ack.filter(|ack| !is_future(ack));
        let floor = later_cursor(stored.as_deref(), trusted_ack);
        if let Some(ref position) = floor {
            identity.advance_cursor(session_id, position);
            if let Some(ms) = to_millis(position) {
                let _ = self.storage.set_user_last_delivered(session_id, ms);
            }
            self.emit(RoutingAction::CursorAdvanced {
                session_id: session_id.to_string(),
                cursor: position.clone(),
            });
        }
        floor
    }

    pub fn mark_delivered(&self, session_id: &str, ts: &str) {
        let mut identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let current = identity.cursor(session_id);
        if let Some(later) = later_cursor(current.as_deref(), Some(ts))
            && current.as_deref() != Some(later.as_str())
        {
            identity.advance_cursor(session_id, &later);
            if let Some(ms) = to_millis(&later) {
                let _ = self.storage.set_user_last_delivered(session_id, ms);
            }
            self.emit(RoutingAction::CursorAdvanced {
                session_id: session_id.to_string(),
                cursor: later.clone(),
            });
        }
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
        let anchored = fresh || cursor_after(stored.as_deref(), job_start);
        if anchored {
            identity.advance_cursor(session_id, job_start);
        }
        identity.link_terminal(session_id, terminal_id);
        let reaped = identity.reap_unclaimed_duplicates_of(display);
        if let Some(current) = identity.display_for(session_id) {
            let _ = self
                .storage
                .upsert_user(session_id, &current, short_name(&current));
        }
        if let Some(cursor) = identity.cursor(session_id)
            && let Some(ms) = to_millis(&cursor)
        {
            let _ = self.storage.set_user_last_delivered(session_id, ms);
        }
        if !reaped.is_empty() {
            let _ = self.storage.delete_users(&reaped);
        }
        if fresh {
            self.emit(RoutingAction::Registered {
                session_id: session_id.to_string(),
                display: display.to_string(),
            });
        }
        if anchored {
            self.emit(RoutingAction::CursorAnchored {
                session_id: session_id.to_string(),
                cursor: job_start.to_string(),
            });
        }
        self.emit(RoutingAction::TerminalLinked {
            session_id: session_id.to_string(),
            terminal_id: terminal_id.to_string(),
        });
        self.emit(RoutingAction::Reaped {
            display: display.to_string(),
        });
    }
}
