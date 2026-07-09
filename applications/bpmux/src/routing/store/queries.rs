use super::{RoutingStore, bridge_agent_name, short_name};

impl RoutingStore {
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

    pub fn resolve(&self, display: &str) -> Vec<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.resolve(display)
    }

    pub fn cursor(&self, session_id: &str) -> Option<String> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        identity.cursor(session_id)
    }

    pub fn delivery_targets(&self, to: Option<&[String]>, sender: &str) -> Vec<(String, String)> {
        let identity = self.identity.lock().unwrap_or_else(|e| e.into_inner());
        let sender_ids = identity.resolve(short_name(sender));
        let recipient_ids: Vec<String> = match to {
            None => identity.all_ids(),
            Some(names) if names.iter().any(|n| n == crate::message::BROADCAST_ALL) => {
                identity.all_ids()
            }
            Some(names) => names
                .iter()
                .flat_map(|name| identity.resolve(short_name(name)))
                .collect(),
        };
        recipient_ids
            .into_iter()
            .filter(|id| !sender_ids.contains(id))
            .filter_map(|id| identity.terminal_for(&id).map(|terminal| (id, terminal)))
            .filter(|(_, terminal)| bridge_agent_name(terminal) != short_name(sender))
            .collect()
    }
}
