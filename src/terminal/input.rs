use super::app::App;

/// Registered slash commands with brief descriptions for the popup.
pub(super) const SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/to", "send to recipients"),
    ("/view", "filter view by channel"),
    ("/search", "search the log"),
    ("/board", "show project board"),
    ("/roadmap", "show project roadmap"),
    ("/status", "declare your status"),
    ("/who", "list participants"),
    ("/debug", "toggle system messages"),
    ("/quit", "exit bpmux"),
];

impl App {
    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += c.len_utf8();
        self.popup_selected = None;
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
            self.popup_selected = None;
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

    /// Returns the filtered list of slash commands matching current input.
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

    /// Returns true if the popup is currently visible.
    pub fn popup_visible(&self) -> bool {
        !self.popup_matches().is_empty()
    }

    /// Move popup selection up.
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

    /// Move popup selection down.
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

    /// Accept the currently selected popup item — fills input with the command + trailing space.
    /// Returns true if a selection was applied.
    pub fn popup_accept(&mut self) -> bool {
        let matches = self.popup_matches();
        if let Some(idx) = self.popup_selected
            && let Some((cmd, _)) = matches.get(idx)
        {
            self.input = format!("{} ", cmd);
            self.cursor_pos = self.input.len();
            self.popup_selected = None;
            return true;
        }
        false
    }

    /// Dismiss the popup without selecting.
    pub fn popup_dismiss(&mut self) {
        self.popup_selected = None;
    }
}
