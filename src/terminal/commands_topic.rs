//! `/topic` slash command — #191 topic-scoped conversations.

use super::app::App;
use crate::session::Message;

impl App {
    pub(super) fn handle_topic(&mut self, args: String) -> Option<Message> {
        let args = args.trim();

        if args.is_empty() {
            // List topics.
            let mut text = String::from("Topics:\n");
            for t in &self.topics {
                let active = if *t == self.active_topic { " ●" } else { "" };
                let unread = self.topic_unreads.get(t).copied().unwrap_or(0);
                if unread > 0 {
                    text.push_str(&format!("  {t}{active} ({unread} new)\n"));
                } else {
                    text.push_str(&format!("  {t}{active}\n"));
                }
            }
            self.system_message(text);
            self.clear_input();
            return None;
        }

        if let Some(name) = args.strip_prefix("create ") {
            let name = name.trim().to_string();
            if name.is_empty() {
                self.system_message("Usage: /topic create <name>".to_string());
            } else if self.topics.contains(&name) {
                self.system_message(format!("Topic '{name}' already exists."));
            } else {
                self.topics.push(name.clone());
                self.active_topic = name.clone();
                self.system_message(format!("Created and switched to topic '{name}'."));
            }
            self.clear_input();
            return None;
        }

        // Switch to existing topic.
        let name = args.to_string();
        if self.topics.contains(&name) {
            self.active_topic = name.clone();
            self.system_message(format!("Switched to topic '{name}'."));
        } else {
            self.system_message(format!(
                "Topic '{name}' not found. Use /topic create {name} to create it."
            ));
        }
        self.clear_input();
        None
    }
}
