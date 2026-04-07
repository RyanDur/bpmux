use super::app::App;

impl App {
    /// Sidebar entries: project root node at index 0, channels indented below when expanded.
    /// Index 0 is always the project node (session_id). Selecting it shows all messages.
    pub fn sidebar_channels(&self) -> Vec<String> {
        let mut result = vec![self.session_id.clone()];
        if self.project_expanded {
            let mut channels: Vec<String> = self.active_channels.iter().cloned().collect();
            channels.sort();
            result.extend(channels);
        }
        result
    }

    /// Toggle sidebar open/closed. When opening, focus moves to sidebar.
    pub fn toggle_sidebar(&mut self) {
        if !self.sidebar_open {
            // Closed → open + focused
            self.sidebar_open = true;
            self.sidebar_focused = true;
        } else if !self.sidebar_focused {
            // Open + unfocused → refocus
            self.sidebar_focused = true;
        } else {
            // Open + focused → close
            self.sidebar_open = false;
            self.sidebar_focused = false;
        }
    }

    /// Move sidebar selection up (wraps around).
    pub fn sidebar_up(&mut self) {
        let count = self.sidebar_channels().len();
        if count == 0 {
            return;
        }
        self.sidebar_selected = if self.sidebar_selected == 0 {
            count - 1
        } else {
            self.sidebar_selected - 1
        };
    }

    /// Move sidebar selection down (wraps around).
    pub fn sidebar_down(&mut self) {
        let count = self.sidebar_channels().len();
        if count == 0 {
            return;
        }
        self.sidebar_selected = (self.sidebar_selected + 1) % count;
    }

    /// Select the current sidebar item: sets view filter and send target.
    /// Index 0 (project node) = show all. Index 1+ = specific channel.
    pub fn sidebar_select(&mut self) {
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar_selected) {
            if self.sidebar_selected == 0 {
                // Project root = all messages (deselected state)
                self.current_view = None;
                self.send_target = None;
            } else {
                self.current_view = Some(ch.clone());
                self.send_target = Some(ch.clone());
                self.channel_unread.remove(ch);
            }
            self.scroll_offset = 0;
            // Return focus to input so the user can type immediately
            self.sidebar_focused = false;
        }
        // Return focus to input after selection
        self.sidebar_focused = false;
    }

    /// Collapse the project tree (Left arrow on project node).
    pub fn sidebar_collapse(&mut self) {
        if self.project_expanded {
            self.project_expanded = false;
            self.sidebar_selected = 0; // snap to project node
        }
    }

    /// Expand the project tree (Right arrow on project node).
    pub fn sidebar_expand(&mut self) {
        self.project_expanded = true;
    }

    /// Unfocus sidebar (return focus to input). Sidebar stays open.
    pub fn sidebar_close(&mut self) {
        self.sidebar_focused = false;
    }
}
