//! `/topic` slash command — #191 topic-scoped conversations.
//!
//! Topics are collaborative rooms with chosen participants.
//! `/topic create teaching pm,designer` creates a room where
//! ryan, pm, and designer see each other's messages. dev doesn't
//! receive them. The topic creator is always a participant.

use super::app::App;
use crate::session::{Message, MessageType};
use chrono::Utc;

impl App {
    pub(super) fn handle_topic(&mut self, args: String) -> Option<Message> {
        let args = args.trim();

        if args.is_empty() {
            // List topics.
            let mut text = String::from("Topics:\n");
            for t in &self.topic.topics {
                let active = if *t == self.topic.active { " ●" } else { "" };
                let participants = self
                    .topic
                    .participants
                    .get(t)
                    .map(|p| p.join(", "))
                    .unwrap_or_else(|| "everyone".to_string());
                let unread = self.topic.unreads.get(t).copied().unwrap_or(0);
                if unread > 0 {
                    text.push_str(&format!("  {t}{active} [{participants}] ({unread} new)\n"));
                } else {
                    text.push_str(&format!("  {t}{active} [{participants}]\n"));
                }
            }
            self.system_message(text);
            self.clear_input();
            return None;
        }

        if let Some(rest) = args.strip_prefix("create ") {
            // Parse: /topic create <name> <participant1>,<participant2>,...
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            let name = parts[0].trim().to_string();
            if name.is_empty() {
                self.system_message(
                    "Usage: /topic create <name> [participants]\n\
                     Example: /topic create teaching pm,designer"
                        .to_string(),
                );
                self.clear_input();
                return None;
            }
            if self.topic.topics.contains(&name) {
                self.system_message(format!("Topic '{name}' already exists."));
                self.clear_input();
                return None;
            }

            // Parse participants — always include self (the creator).
            let mut participants: Vec<String> = vec![self.peer_name.clone()];
            if let Some(participant_str) = parts.get(1) {
                for p in participant_str.split(',') {
                    let p = p.trim().to_string();
                    if !p.is_empty() && !participants.contains(&p) {
                        participants.push(p);
                    }
                }
            }

            self.topic.topics.push(name.clone());
            self.topic
                .participants
                .insert(name.clone(), participants.clone());
            self.topic.active = name.clone();
            self.mark_filters_dirty();

            let participant_list = participants.join(", ");
            self.system_message(format!("Created topic '{name}' with: {participant_list}"));

            // Broadcast a topic-created message so agents know
            // to subscribe. The message goes to the participants
            // via the `to` field so non-participants don't see it.
            let notify = Message {
                from: "bpmux".to_string(),
                text: format!("Created topic '{name}' with: {participant_list}"),
                ts: Utc::now(),
                message_type: MessageType::System,
                to: Some(participants),
                addressee: None,
                explicit_privacy: false,
                multi_depth: None,
                topic: None, // system message, visible everywhere
            };
            self.push_message(notify.clone());
            self.clear_input();
            return Some(notify);
        }

        // Switch to existing topic.
        let name = args.to_string();
        if self.topic.topics.contains(&name) {
            self.topic.active = name.clone();
            self.mark_filters_dirty();
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
