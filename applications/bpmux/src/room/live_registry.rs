use std::collections::HashMap;
use std::sync::Mutex;

use tokio::sync::broadcast;

use super::spawn::LivePty;

#[derive(Default)]
pub struct LivePtyRegistry {
    rooms: Mutex<HashMap<String, LivePty>>,
}

impl LivePtyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, role: String, live: LivePty) {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(role, live);
    }

    pub fn contains(&self, role: &str) -> bool {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains_key(role)
    }

    pub fn subscribe_for(&self, role: &str) -> Option<broadcast::Receiver<Vec<u8>>> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .map(|live| live.subscribe())
    }

    pub fn write_to(&self, role: &str, bytes: &[u8]) -> bool {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .map(|live| live.write_input(bytes).is_ok())
            .unwrap_or(false)
    }

    pub fn submit_to(&self, role: &str, line: &[u8]) -> bool {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(role)
            .map(|live| live.submit_input(line).is_ok())
            .unwrap_or(false)
    }

    pub fn resize(&self, terminal_id: &str, rows: u16, cols: u16) -> bool {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(terminal_id)
            .map(|live| live.resize(rows, cols).is_ok())
            .unwrap_or(false)
    }

    pub fn remove(&self, terminal_id: &str) -> Option<LivePty> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(terminal_id)
    }

    pub fn has_foreground_job(&self, terminal_id: &str) -> Option<bool> {
        self.rooms
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(terminal_id)
            .map(|live| live.has_foreground_job())
    }

    pub fn reap_dead_and_list_live(&self) -> Vec<String> {
        let mut rooms = self.rooms.lock().unwrap_or_else(|e| e.into_inner());
        let mut live = Vec::new();
        rooms.retain(|terminal_id, pty| {
            if pty.is_alive() {
                live.push(terminal_id.clone());
                true
            } else {
                false
            }
        });
        live
    }
}
