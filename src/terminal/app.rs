use crate::session::Message;
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

const MAX_HISTORY: usize = 50;

pub struct App {
    pub messages: Vec<Message>,
    pub input: String,
    pub cursor_pos: usize,
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
    pub peer_activity: HashMap<String, (String, Instant)>,
    pub show_system_messages: bool,
    pub expanded_messages: HashSet<usize>,
}

impl App {
    pub fn new(peer_name: String, session_id: String, messages: Vec<Message>) -> Self {
        Self {
            messages,
            input: String::new(),
            cursor_pos: 0,
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
            peer_activity: HashMap::new(),
            show_system_messages: false,
            expanded_messages: HashSet::new(),
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            let prev = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.input.remove(prev);
            self.cursor_pos = prev;
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos = self.input[..self.cursor_pos]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos = self.input[self.cursor_pos..]
                .char_indices()
                .nth(1)
                .map(|(i, _)| self.cursor_pos + i)
                .unwrap_or(self.input.len());
        }
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.input.len();
    }

    pub fn kill_to_end(&mut self) {
        self.input.truncate(self.cursor_pos);
    }

    pub fn delete_word_backward(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        let word_start = trimmed
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
        self.input.drain(word_start..self.cursor_pos);
        self.cursor_pos = word_start;
    }

    pub fn move_word_left(&mut self) {
        if self.cursor_pos == 0 {
            return;
        }
        let before = &self.input[..self.cursor_pos];
        let trimmed = before.trim_end();
        self.cursor_pos = trimmed
            .rfind(|c: char| c.is_whitespace())
            .map(|i| i + 1)
            .unwrap_or(0);
    }

    pub fn move_word_right(&mut self) {
        if self.cursor_pos >= self.input.len() {
            return;
        }
        let after = &self.input[self.cursor_pos..];
        let skip_word = after
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after.len());
        let skip_space = after[skip_word..]
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(after.len() - skip_word);
        self.cursor_pos += skip_word + skip_space;
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
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

    pub fn toggle_last_truncated(&mut self) {
        let width = 78usize; // approximate inner width
        for (i, msg) in self.messages.iter().enumerate().rev() {
            if !msg.is_chat() && !self.show_system_messages {
                continue;
            }
            let prefix_len = msg.from.split(':').next().unwrap_or(&msg.from).len() + 2;
            let full_len = prefix_len + msg.text.len();
            let wrapped = full_len.max(1).div_ceil(width.max(1));
            if wrapped > 10 {
                if self.expanded_messages.contains(&i) {
                    self.expanded_messages.remove(&i);
                } else {
                    self.expanded_messages.insert(i);
                }
                return;
            }
        }
    }

    pub fn update_activity(&mut self, peer: String, status: String) {
        self.peer_activity.insert(peer, (status, Instant::now()));
    }

    pub fn receive_message(&mut self, msg: Message) {
        if msg.message_type.as_deref() == Some("activity") {
            self.update_activity(msg.from, msg.text);
            return;
        }
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
                "Commands:\n  /to <who> <msg> — send to specific recipients (e.g. /to dev,pm hello)\n  /help     — show this help\n  /who      — list participants\n  /debug    — toggle system messages\n  /quit     — exit bpmux\n\nKeybindings:\n  Enter     — send message\n  Esc       — clear input\n  Ctrl+C    — clear input (empty: quit)\n  Ctrl+U/D  — scroll up/down\n  Ctrl+T    — toggle timestamps\n  PageUp/Dn — scroll\n\nSession: {}",
                self.session_id
            );
            let msg = Message {
                from: "bpmux".to_string(),
                text: help,
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/who" {
            let mut participants = vec![format!("{} (you)", self.peer_name)];
            for (peer, (status, _)) in &self.peer_activity {
                let short = peer.split(':').next().unwrap_or(peer);
                participants.push(format!("{} [{}]", short, status));
            }
            if self.connected_peers > 0 {
                participants.push(format!("{} connected", self.connected_peers));
            }
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!("Participants:\n  {}", participants.join("\n  ")),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/quit" {
            self.should_quit = true;
            return None;
        }
        if text == "/debug" {
            self.show_system_messages = !self.show_system_messages;
            let state = if self.show_system_messages {
                "on"
            } else {
                "off"
            };
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!(
                    "Debug mode {state}. System messages are now {}.",
                    if self.show_system_messages {
                        "visible"
                    } else {
                        "hidden"
                    }
                ),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if let Some(rest) = text.strip_prefix("/to ") {
            let rest = rest.trim_start();
            if let Some((recipients_str, body)) = rest.split_once(' ') {
                let body = body.trim().to_string();
                if body.is_empty() {
                    let msg = Message {
                        from: "bpmux".to_string(),
                        text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello"
                            .to_string(),
                        ts: Utc::now(),
                        message_type: None,
                        to: None,
                    };
                    self.messages.push(msg);
                    self.clear_input();
                    return None;
                }
                let recipients: Vec<String> = recipients_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if recipients.is_empty() {
                    let msg = Message {
                        from: "bpmux".to_string(),
                        text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello"
                            .to_string(),
                        ts: Utc::now(),
                        message_type: None,
                        to: None,
                    };
                    self.messages.push(msg);
                    self.clear_input();
                    return None;
                }
                let msg = Message {
                    from: format!("{}:human", self.peer_name),
                    text: body,
                    ts: Utc::now(),
                    message_type: None,
                    to: Some(recipients),
                };
                self.messages.push(msg.clone());
                self.clear_input();
                self.scroll_offset = 0;
                self.unread_count = 0;
                if self.input_history.len() >= MAX_HISTORY {
                    self.input_history.remove(0);
                }
                self.input_history.push(self.input.clone());
                self.history_index = None;
                return Some(msg);
            }
            // `/to <something>` with no body
            let msg = Message {
                from: "bpmux".to_string(),
                text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text == "/to" {
            let msg = Message {
                from: "bpmux".to_string(),
                text: "Usage: /to <recipients> <message>. Example: /to dev,pm hello".to_string(),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        if text.starts_with('/') {
            let msg = Message {
                from: "bpmux".to_string(),
                text: format!(
                    "Unknown command: {}. Type /help for available commands.",
                    text
                ),
                ts: Utc::now(),
                message_type: None,
                to: None,
            };
            self.messages.push(msg);
            self.clear_input();
            return None;
        }
        let msg = Message {
            from: format!("{}:human", self.peer_name),
            text,
            ts: Utc::now(),
            message_type: None,
            to: None,
        };
        self.messages.push(msg.clone());
        self.clear_input();
        self.scroll_offset = 0;
        self.unread_count = 0;
        if self.input_history.len() >= MAX_HISTORY {
            self.input_history.remove(0);
        }
        self.input_history.push(msg.text.clone());
        self.history_index = None;
        Some(msg)
    }

    /// Tab-complete participant names after `/to `.
    /// Returns true if completion was applied, false if no match.
    pub fn tab_complete(&mut self) -> bool {
        let input = self.input.trim_start();
        let rest = match input.strip_prefix("/to ") {
            Some(r) => r,
            None => return false,
        };
        // Only complete in the recipients portion (before any space in the rest)
        if rest.contains(' ') {
            return false;
        }
        // Get the partial name being typed (after last comma, if CSV)
        let (prefix_part, partial) = match rest.rfind(',') {
            Some(i) => (&rest[..=i], rest[i + 1..].trim()),
            None => ("", rest.trim()),
        };
        // Collect known peer short names
        let mut candidates: Vec<String> = self
            .peer_activity
            .keys()
            .map(|k| k.split(':').next().unwrap_or(k).to_string())
            .filter(|name| name.starts_with(partial) && name != partial)
            .collect();
        candidates.sort();
        if candidates.is_empty() {
            return false;
        }
        // Pick first match (simple, no cycling for now)
        let completed = &candidates[0];
        self.input = format!("/to {}{} ", prefix_part, completed);
        self.cursor_pos = self.input.len();
        true
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
            self.clear_input();
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
            to: None,
        });
        assert_eq!(app.unread_count, 1);
        app.receive_message(Message {
            from: "bot:ws".to_string(),
            text: "world".to_string(),
            ts: Utc::now(),
            message_type: None,
            to: None,
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
            to: None,
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
            to: None,
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
    fn who_command_lists_peer_name_and_returns_none() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/who".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].from, "bpmux");
        assert!(app.messages[0].text.contains("ryan (you)"));
    }

    #[test]
    fn who_command_includes_peer_activity_and_connected_count() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.connected_peers = 2;
        app.input = "/who".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("dev [working]"));
        assert!(app.messages[0].text.contains("2 connected"));
    }

    #[test]
    fn unrecognized_command_adds_error_message_without_broadcast() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/foo".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert_eq!(app.messages[0].from, "bpmux");
        assert!(app.messages[0].text.contains("Unknown command"));
        assert!(app.messages[0].text.contains("/foo"));
        assert!(app.messages[0].text.contains("/help"));
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
            to: None,
        });
        assert_eq!(app.scroll_offset, 6);
        assert_eq!(app.messages.len(), 1);
    }

    #[test]
    fn update_activity_adds_entry() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        assert!(app.peer_activity.contains_key("dev:claude-code"));
        let (status, _) = &app.peer_activity["dev:claude-code"];
        assert_eq!(status, "working");
    }

    #[test]
    fn activity_message_does_not_appear_in_messages() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.receive_message(Message {
            from: "dev:claude-code".to_string(),
            text: "working".to_string(),
            ts: Utc::now(),
            message_type: Some("activity".to_string()),
            to: None,
        });
        assert!(app.messages.is_empty());
        assert!(app.peer_activity.contains_key("dev:claude-code"));
    }

    #[test]
    fn debug_toggles_show_system_messages() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        assert!(!app.show_system_messages);

        app.input = "/debug".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.show_system_messages);
        assert!(app.input.is_empty());
        assert_eq!(app.messages.len(), 1);
        assert!(app.messages[0].text.contains("Debug mode on"));
        assert!(app.messages[0].text.contains("visible"));

        app.input = "/debug".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(!app.show_system_messages);
        assert_eq!(app.messages.len(), 2);
        assert!(app.messages[1].text.contains("Debug mode off"));
        assert!(app.messages[1].text.contains("hidden"));
    }

    #[test]
    fn help_includes_debug_command() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/help".to_string();
        app.submit_message();
        assert!(app.messages[0].text.contains("/debug"));
    }

    #[test]
    fn help_includes_to_command() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/help".to_string();
        app.submit_message();
        assert!(app.messages[0].text.contains("/to"));
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn to_single_recipient_returns_directed_message() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to dev hello there".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.from, "ryan:human");
        assert_eq!(msg.text, "hello there");
        assert_eq!(msg.to, Some(vec!["dev".to_string()]));
    }

    #[allow(clippy::unwrap_used)]
    #[test]
    fn to_csv_recipients_returns_directed_message() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to dev,designer check this".to_string();
        let msg = app.submit_message().unwrap();
        assert_eq!(msg.text, "check this");
        assert_eq!(
            msg.to,
            Some(vec!["dev".to_string(), "designer".to_string()])
        );
    }

    #[test]
    fn to_bare_shows_usage() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("Usage:"));
    }

    #[test]
    fn to_recipient_no_body_shows_usage() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "/to dev".to_string();
        let msg = app.submit_message();
        assert!(msg.is_none());
        assert!(app.messages[0].text.contains("Usage:"));
    }

    #[test]
    fn tab_completes_participant_name_after_to() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.update_activity("designer:claude-code".to_string(), "idle".to_string());
        app.input = "/to de".to_string();
        app.cursor_pos = app.input.len();
        assert!(app.tab_complete());
        // Should complete to first alphabetical match
        assert!(
            app.input.starts_with("/to de"),
            "should complete a 'de' prefix, got: {}",
            app.input
        );
        assert!(
            app.input.ends_with(' '),
            "should add trailing space for body"
        );
    }

    #[test]
    fn tab_completes_csv_second_recipient() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.update_activity("pm:claude-code".to_string(), "idle".to_string());
        app.input = "/to dev,p".to_string();
        app.cursor_pos = app.input.len();
        assert!(app.tab_complete());
        assert_eq!(app.input, "/to dev,pm ");
    }

    #[test]
    fn tab_does_nothing_without_to_prefix() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.input = "hello".to_string();
        app.cursor_pos = app.input.len();
        assert!(!app.tab_complete());
        assert_eq!(app.input, "hello");
    }

    #[test]
    fn tab_does_nothing_in_body_portion() {
        let mut app = App::new("ryan".to_string(), "test-session".to_string(), vec![]);
        app.update_activity("dev:claude-code".to_string(), "working".to_string());
        app.input = "/to dev hel".to_string();
        app.cursor_pos = app.input.len();
        assert!(!app.tab_complete());
    }
}
