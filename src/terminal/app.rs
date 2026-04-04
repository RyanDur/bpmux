use crate::session::Message;
use chrono::Utc;

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub scroll_offset: u16,
    pub peer_name: String,
    pub session_id: String,
    pub should_quit: bool,
    pub show_timestamps: bool,
    pub connected_peers: usize,
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        Self {
            messages,
            input: String::new(),
            scroll_offset: 0,
            peer_name,
            session_id,
            should_quit: false,
            show_timestamps: false,
            connected_peers: 0,
        }
    }

    pub fn scroll_up(&mut self, lines: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(lines);
    }

    pub fn scroll_down(&mut self, lines: u16) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn receive_message(&mut self, msg: Message) {
        let was_at_bottom = self.scroll_offset == 0;
        self.messages.push(msg);
        if !was_at_bottom {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
        }
    }

    pub fn submit_message(&mut self) -> Option<Message> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }
        if text == "/quit" {
            self.should_quit = true;
            return None;
        }
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text,
            ts: Utc::now(),
            message_type: None,
        };
        self.messages.push(msg.clone());
        self.input.clear();
        Some(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(clippy::unwrap_used)]
    #[test]
    fn submits_message_and_clears_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello world".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.from, "ryan:human");
        assert_eq!(msg.text, "hello world");
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn ignores_empty_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "   ".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
    }

    #[test]
    fn quit_command_sets_flag() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/quit".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.should_quit);
    }

    #[test]
    fn scroll_up_increases_offset() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_up(5);
        assert_eq!(app.scroll_offset, 5);
        app.scroll_up(3);
        assert_eq!(app.scroll_offset, 8);
    }

    #[test]
    fn scroll_down_decreases_offset_to_zero() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 10;
        app.scroll_down(3);
        assert_eq!(app.scroll_offset, 7);
        app.scroll_down(10);
        assert_eq!(app.scroll_offset, 0);
    }

    #[test]
    fn incoming_message_stays_at_bottom_when_at_bottom() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        assert_eq!(app.scroll_offset, 0);
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
        });
        assert_eq!(app.scroll_offset, 0);
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn incoming_message_preserves_viewport_when_scrolled_up() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 5;
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
        });
        assert_eq!(app.scroll_offset, 6);
        assert_eq!(app.messages.len(), 1);
    }
}
