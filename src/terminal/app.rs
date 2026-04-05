use crate::session::Message;
use chrono::Utc;

const MAX_HISTORY: usize = 50;

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub scroll_offset: u16,
    pub max_scroll: u16,
    pub at_boundary: bool,
    pub peer_name: String,
    pub session_id: String,
    pub should_quit: bool,
    pub show_timestamps: bool,
    pub connected_peers: usize,
    pub unread_count: usize,
    pub input_history: Vec<String>,
    pub history_index: Option<usize>,
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        Self {
            messages,
            input: String::new(),
            scroll_offset: 0,
            max_scroll: 0,
            at_boundary: false,
            peer_name,
            session_id,
            should_quit: false,
            show_timestamps: false,
            connected_peers: 0,
            unread_count: 0,
            input_history: Vec::new(),
            history_index: None,
        }
    }

    pub fn scroll_up(&mut self, lines: u16) {
        let before = self.scroll_offset;
        self.scroll_offset = self
            .scroll_offset
            .saturating_add(lines)
            .min(self.max_scroll);
        if self.scroll_offset != before {
            self.at_boundary = false;
        }
    }

    pub fn scroll_down(&mut self, lines: u16) {
        let before = self.scroll_offset;
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
        if self.scroll_offset != before {
            self.at_boundary = false;
        }
        if self.scroll_offset == 0 {
            self.unread_count = 0;
        }
    }

    pub fn receive_message(&mut self, msg: Message) {
        let was_at_bottom = self.scroll_offset == 0;
        self.messages.push(msg);
        if !was_at_bottom {
            self.scroll_offset = self.scroll_offset.saturating_add(1);
            self.unread_count += 1;
        }
    }

    pub fn submit_message(&mut self) -> Option<Message> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return None;
        }
        if text == "/help" {
            let help = format!(
                "Commands:\n  /help     — show this help\n  /quit     — exit bpmux\n\nKeybindings:\n  Enter     — send message\n  Esc       — clear input\n  Ctrl+C    — clear input (empty: quit)\n  Ctrl+U/D  — scroll up/down\n  Ctrl+T    — toggle timestamps\n  PageUp/Dn — scroll\n\nSession: {}",
                self.session_id
            );
            let msg = Message {
                from: "bpmux".to_string(),
                text: help,
                ts: Utc::now(),
                message_type: None,
            };
            self.messages.push(msg);
            self.input.clear();
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
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(msg.text.clone());
        self.history_index = None;
        Some(msg)
    }

    pub fn history_up(&mut self) {
        if self.input_history.is_empty() {
            return;
        }
        let new_index = match self.history_index {
            None => self.input_history.len() - 1,
            Some(0) => 0,
            Some(i) => i - 1,
        };
        self.history_index = Some(new_index);
        self.input = self.input_history[new_index].clone();
    }

    pub fn history_down(&mut self) {
        let Some(current) = self.history_index else {
            return;
        };
        if current + 1 >= self.input_history.len() {
            self.history_index = None;
            self.input.clear();
        } else {
            let new_index = current + 1;
            self.history_index = Some(new_index);
            self.input = self.input_history[new_index].clone();
        }
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
    fn help_command_adds_message_and_clears_input() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/help".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].from, "bpmux");
        assert!(app.messages[0].text.contains("/help"));
        assert!(app.messages[0].text.contains("/quit"));
        assert!(app.messages[0].text.contains("test-session"));
    }

    #[test]
    fn scroll_up_increases_offset() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.max_scroll = 100;
        app.scroll_up(5);
        assert_eq!(app.scroll_offset, 5);
        app.scroll_up(3);
        assert_eq!(app.scroll_offset, 8);
    }

    #[test]
    fn scroll_up_clamps_to_max_scroll() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.max_scroll = 10;
        app.scroll_up(15);
        assert_eq!(app.scroll_offset, 10);
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
    fn receiving_message_when_scrolled_up_increments_unread_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 5;
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
        });
        assert_eq!(app.unread_count, 1);
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "world".to_string(),
            ts: Utc::now(),
            message_type: None,
        });
        assert_eq!(app.unread_count, 2);
    }

    #[test]
    fn scrolling_to_bottom_resets_unread_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.scroll_offset = 3;
        app.unread_count = 5;
        app.scroll_down(3);
        assert_eq!(app.scroll_offset, 0);
        assert_eq!(app.unread_count, 0);
    }

    #[test]
    fn receiving_message_at_bottom_does_not_increment_unread_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        assert_eq!(app.scroll_offset, 0);
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "hello".to_string(),
            ts: Utc::now(),
            message_type: None,
        });
        assert_eq!(app.unread_count, 0);
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
    fn up_arrow_recalls_last_sent_message() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello".to_string();
        app.submit_message();
        app.input = "world".to_string();
        app.submit_message();

        app.history_up();
        assert_eq!(app.input, "world");
        assert_eq!(app.history_index, Some(1));

        app.history_up();
        assert_eq!(app.input, "hello");
        assert_eq!(app.history_index, Some(0));
    }

    #[test]
    fn down_arrow_returns_to_empty_after_recalling() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello".to_string();
        app.submit_message();

        app.history_up();
        assert_eq!(app.input, "hello");

        app.history_down();
        assert!(app.input.is_empty());
        assert_eq!(app.history_index, None);
    }

    #[test]
    fn history_caps_at_50_entries() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        for i in 0..60 {
            app.input = format!("msg {i}");
            app.submit_message();
        }
        assert_eq!(app.input_history.len(), MAX_HISTORY);
        assert_eq!(app.input_history[0], "msg 10");
        assert_eq!(app.input_history[49], "msg 59");
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
