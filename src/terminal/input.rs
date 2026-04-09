use super::app::App;
use tui_textarea::CursorMove;

/// Registered slash commands with brief descriptions for the popup.
pub(super) const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/to", "send to recipients"),
    ("/view", "filter view by channel"),
    ("/search", "search the log"),
    ("/board", "show project board"),
    ("/roadmap", "show project roadmap"),
    ("/status", "declare your status"),
    ("/who", "list participants"),
    ("/session", "create P2P session"),
    ("/join", "join a remote session"),
    ("/accept", "accept connection answer"),
    ("/debug", "toggle system messages"),
    ("/quit", "exit bpmux"),
];

impl App {
    pub fn insert_char(&mut self, c: char) {
        self.textarea.insert_char(c);
        self.sync_input_from_textarea();
        self.popup_selected = None;
    }

    pub fn delete_char(&mut self) {
        self.textarea.delete_char();
        self.sync_input_from_textarea();
        self.popup_selected = None;
    }

    pub fn move_cursor_left(&mut self) {
        self.textarea.move_cursor(CursorMove::Back);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_right(&mut self) {
        self.textarea.move_cursor(CursorMove::Forward);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_home(&mut self) {
        self.textarea.move_cursor(CursorMove::Head);
        self.sync_input_from_textarea();
    }

    pub fn move_cursor_end(&mut self) {
        self.textarea.move_cursor(CursorMove::End);
        self.sync_input_from_textarea();
    }

    pub fn kill_to_end(&mut self) {
        self.textarea.delete_line_by_end();
        self.sync_input_from_textarea();
    }

    pub fn delete_word_backward(&mut self) {
        self.textarea.delete_word();
        self.sync_input_from_textarea();
    }

    pub fn move_word_left(&mut self) {
        self.textarea.move_cursor(CursorMove::WordBack);
        self.sync_input_from_textarea();
    }

    pub fn move_word_right(&mut self) {
        self.textarea.move_cursor(CursorMove::WordForward);
        self.sync_input_from_textarea();
    }

    pub fn clear_input(&mut self) {
        let mut ta = tui_textarea::TextArea::default();
        ta.set_hard_tab_indent(false);
        ta.set_wrap_mode(tui_textarea::WrapMode::Word);
        self.textarea = ta;
        self.sync_input_from_textarea();
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
        self.set_textarea_content(&self.input_history[new_index].clone());
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
            self.set_textarea_content(&self.input_history[new_index].clone());
        }
    }

    pub fn tab_complete(&mut self) -> bool {
        let input = self.input.trim_start();
        let rest = match input.strip_prefix("/to ") {
            Some(r) => r,
            None => return false,
        };
        if rest.contains(' ') {
            return false;
        }
        let (prefix_part, partial) = match rest.rfind(',') {
            Some(i) => (&rest[..=i], rest[i + 1..].trim()),
            None => ("", rest.trim()),
        };
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
        let completed = &candidates[0];
        let new_input = format!("/to {}{} ", prefix_part, completed);
        self.set_textarea_content(&new_input);
        true
    }

    pub fn popup_matches(&self) -> Vec<(&'static str, &'static str)> {
        if !self.input.starts_with('/') || self.input.contains(' ') {
            return vec![];
        }
        let typed = self.input.split_whitespace().next().unwrap_or(&self.input);
        SLASH_COMMANDS
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(typed))
            .copied()
            .collect()
    }

    pub fn popup_visible(&self) -> bool {
        !self.popup_matches().is_empty()
    }

    pub fn popup_up(&mut self) {
        let count = self.popup_matches().len();
        if count == 0 {
            return;
        }
        self.popup_selected = Some(match self.popup_selected {
            None | Some(0) => count - 1,
            Some(i) => i - 1,
        });
    }

    pub fn popup_down(&mut self) {
        let count = self.popup_matches().len();
        if count == 0 {
            return;
        }
        self.popup_selected = Some(match self.popup_selected {
            None => 0,
            Some(i) if i + 1 >= count => 0,
            Some(i) => i + 1,
        });
    }

    pub fn popup_accept(&mut self) -> bool {
        let matches = self.popup_matches();
        if let Some(idx) = self.popup_selected
            && let Some((cmd, _)) = matches.get(idx)
        {
            let new_input = format!("{} ", cmd);
            self.set_textarea_content(&new_input);
            self.popup_selected = None;
            return true;
        }
        false
    }

    pub fn popup_dismiss(&mut self) {
        self.popup_selected = None;
    }
}
