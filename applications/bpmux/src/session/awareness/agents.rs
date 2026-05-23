use crate::session::message::{Message, MessageType};

pub struct AgentTracker {
    owner_name: String,
    pub connected: Vec<String>,
}

impl AgentTracker {
    pub fn new(owner_name: &str) -> Self {
        Self {
            owner_name: owner_name.to_string(),
            connected: Vec::new(),
        }
    }

    pub fn on_message(&mut self, msg: &Message) {
        if msg.message_type != MessageType::System {
            return;
        }
        if let Some(peer) = msg.text.strip_suffix(" joined") {
            let short = peer.split(':').next().unwrap_or(peer);
            if short != self.owner_name && !self.connected.contains(&short.to_string()) {
                self.connected.push(short.to_string());
            }
        } else if let Some(peer) = msg.text.strip_suffix(" left") {
            let short = peer.split(':').next().unwrap_or(peer);
            self.connected.retain(|a| a != short);
        }
    }
}
