use crate::session::{Message, MessageType};
use crate::terminal::app::App;
use chrono::Utc;

impl App {
    pub(in crate::terminal) fn handle_status(&mut self, text: String) -> Option<Message> {
        let msg = Message {
            from: self.from_identity(),
            text,
            ts: Utc::now(),
            message_type: MessageType::Activity,
            ..Message::default()
        };
        self.push_message(msg.clone());
        self.clear_input();
        self.scroll.offset = 0;
        self.scroll.unread_count = 0;
        Some(msg)
    }
}
