use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct RoutingIdentity {
    display_by_id: HashMap<String, String>,
    cursor_by_id: HashMap<String, String>,
    terminal_by_id: HashMap<String, String>,
    role_by_id: HashMap<String, String>,
}

impl RoutingIdentity {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, user_id: &str, display: &str, role: &str) {
        self.display_by_id
            .insert(user_id.to_string(), display.to_string());
        self.cursor_by_id.entry(user_id.to_string()).or_default();
        self.role_by_id
            .insert(user_id.to_string(), role.to_string());
    }

    pub fn role_for(&self, user_id: &str) -> Option<String> {
        self.role_by_id.get(user_id).cloned()
    }

    pub fn advance_cursor(&mut self, user_id: &str, cursor: &str) {
        if let Some(slot) = self.cursor_by_id.get_mut(user_id) {
            *slot = cursor.to_string();
        }
    }

    pub fn cursor(&self, user_id: &str) -> Option<String> {
        self.cursor_by_id.get(user_id).cloned()
    }

    pub fn rename(&mut self, user_id: &str, new_display: &str) {
        if let Some(slot) = self.display_by_id.get_mut(user_id) {
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

    pub fn contains_id(&self, user_id: &str) -> bool {
        self.display_by_id.contains_key(user_id)
    }

    pub fn missed_after_cursor(&self, user_id: &str, event_cursors: &[String]) -> Vec<String> {
        let cursor = self.cursor(user_id).unwrap_or_default();
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

    pub fn link_terminal(&mut self, user_id: &str, terminal_id: &str) {
        self.terminal_by_id
            .insert(user_id.to_string(), terminal_id.to_string());
    }

    pub fn terminal_for(&self, user_id: &str) -> Option<String> {
        self.terminal_by_id.get(user_id).cloned()
    }

    pub fn display_for(&self, user_id: &str) -> Option<String> {
        self.display_by_id.get(user_id).cloned()
    }

    pub fn user_for_terminal(&self, terminal_id: &str) -> Option<String> {
        self.terminal_by_id
            .iter()
            .find(|(_, linked)| linked.as_str() == terminal_id)
            .map(|(user_id, _)| user_id.clone())
    }

    pub fn is_terminal_linked(&self, terminal_id: &str) -> bool {
        self.terminal_by_id
            .values()
            .any(|linked| linked == terminal_id)
    }

    pub fn display_for_terminal(&self, terminal_id: &str) -> Option<String> {
        let user_id = self
            .terminal_by_id
            .iter()
            .find(|(_, linked)| linked.as_str() == terminal_id)
            .map(|(user_id, _)| user_id.clone())?;
        self.display_by_id.get(&user_id).cloned()
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
            self.role_by_id.remove(id);
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
