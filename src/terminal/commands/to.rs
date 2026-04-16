use super::MAX_HISTORY;
use crate::session::{Message, MessageType};
use crate::terminal::app::App;
use chrono::Utc;

impl App {
    pub(in crate::terminal) fn handle_to(&mut self, rest: String) -> Option<Message> {
        let rest = rest.trim_start();
        let Some((recipients_str, body)) = rest.split_once(' ') else {
            return self.show_to_usage();
        };
        let body = body.trim().to_string();
        if body.is_empty() {
            return self.show_to_usage();
        }
        let recipients: Vec<String> = recipients_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if recipients.is_empty() {
            return self.show_to_usage();
        }
        // `/to` is user-explicit privacy intent. Flag it so /share can
        // refuse to propagate it and preserve the DM guarantee.
        let msg = Message {
            from: self.from_identity(),
            text: body,
            ts: Utc::now(),
            message_type: MessageType::Chat,
            to: Some(recipients),
            addressee: None,
            explicit_privacy: true,
            multi_depth: None,
            topic: None,
        };
        self.push_message(msg.clone());
        self.clear_input();
        self.scroll.offset = 0;
        self.scroll.unread_count = 0;
        if self.editor.input_history.len() >= MAX_HISTORY {
            self.editor.input_history.remove(0);
        }
        self.editor.input_history.push(self.editor.input.clone());
        self.editor.history_index = None;
        Some(msg)
    }

    fn show_to_usage(&mut self) -> Option<Message> {
        self.system_message("/to <recipients> <message>\nExample: /to dev,pm hello".to_string());
        self.clear_input();
        None
    }
}
