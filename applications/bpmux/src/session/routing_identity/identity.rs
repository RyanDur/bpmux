use std::collections::HashMap;

#[derive(Default)]
pub struct RoutingIdentity {
    display_by_id: HashMap<String, String>,
    cursor_by_id: HashMap<String, String>,
    terminal_by_id: HashMap<String, String>,
}

impl RoutingIdentity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, session_id: &str, display: &str) {
        self.display_by_id
            .insert(session_id.to_string(), display.to_string());
        self.cursor_by_id.entry(session_id.to_string()).or_default();
    }

    pub fn advance_cursor(&mut self, session_id: &str, cursor: &str) {
        if let Some(slot) = self.cursor_by_id.get_mut(session_id) {
            *slot = cursor.to_string();
        }
    }

    pub fn cursor(&self, session_id: &str) -> Option<String> {
        self.cursor_by_id.get(session_id).cloned()
    }

    pub fn rename(&mut self, session_id: &str, new_display: &str) {
        if let Some(slot) = self.display_by_id.get_mut(session_id) {
            *slot = new_display.to_string();
        }
    }

    pub fn resolve(&self, display: &str) -> Vec<String> {
        self.display_by_id
            .iter()
            .filter(|(_, d)| d.as_str() == display)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn contains_id(&self, session_id: &str) -> bool {
        self.display_by_id.contains_key(session_id)
    }

    pub fn missed_after_cursor(&self, session_id: &str, event_cursors: &[String]) -> Vec<String> {
        let cursor = self.cursor(session_id).unwrap_or_default();
        event_cursors
            .iter()
            .filter(|candidate| **candidate > cursor)
            .cloned()
            .collect()
    }

    pub fn resolve_recipients(&self, to: Option<&[String]>) -> Vec<String> {
        match to {
            None => self.display_by_id.keys().cloned().collect(),
            Some(displays) => displays
                .iter()
                .flat_map(|display| self.resolve(display))
                .collect(),
        }
    }

    pub fn link_terminal(&mut self, session_id: &str, terminal_id: &str) {
        self.terminal_by_id
            .insert(session_id.to_string(), terminal_id.to_string());
    }

    pub fn terminal_for(&self, session_id: &str) -> Option<String> {
        self.terminal_by_id.get(session_id).cloned()
    }

    pub fn display_for(&self, session_id: &str) -> Option<String> {
        self.display_by_id.get(session_id).cloned()
    }

    pub fn session_for_terminal(&self, terminal_id: &str) -> Option<String> {
        self.terminal_by_id
            .iter()
            .find(|(_, linked)| linked.as_str() == terminal_id)
            .map(|(session_id, _)| session_id.clone())
    }

    pub fn is_terminal_linked(&self, terminal_id: &str) -> bool {
        self.terminal_by_id
            .values()
            .any(|linked| linked == terminal_id)
    }

    pub fn display_for_terminal(&self, terminal_id: &str) -> Option<String> {
        let session_id = self
            .terminal_by_id
            .iter()
            .find(|(_, linked)| linked.as_str() == terminal_id)
            .map(|(session_id, _)| session_id.clone())?;
        self.display_by_id.get(&session_id).cloned()
    }

    pub fn reap_unclaimed_duplicates_of(&mut self, display: &str) -> Vec<String> {
        let reaped: Vec<String> = self
            .display_by_id
            .iter()
            .filter(|(id, d)| {
                d.as_str() == display && !self.terminal_by_id.contains_key(id.as_str())
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &reaped {
            self.display_by_id.remove(id);
            self.cursor_by_id.remove(id);
        }
        reaped
    }

    pub fn unclaimed(&self) -> Vec<(String, String)> {
        self.display_by_id
            .iter()
            .filter(|(id, _)| !self.terminal_by_id.contains_key(*id))
            .map(|(id, display)| (id.clone(), display.clone()))
            .collect()
    }

    pub fn all_ids(&self) -> Vec<String> {
        self.display_by_id.keys().cloned().collect()
    }
}
