use crate::session::Message;
use chrono::Utc;

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub scroll_offset: u16,
    pub peer_name: String,
    pub session_id: String,
    pub should_quit: bool,
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
}
