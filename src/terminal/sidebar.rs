use super::app::App;

fn channel_sort_key(channel: &str) -> (u8, String) {
    if channel == "broadcast" {
        (0, channel.to_string())
    } else if channel.contains('+') {
        (2, channel.to_string())
    } else {
        (1, channel.to_string())
    }
}

impl App {
    pub fn sidebar_channels(&self) -> Vec<String> {
        let mut result = vec![self.session_id.clone()];
        if self.project_expanded {
            let mut channels: Vec<String> = self
                .active_channels
                .iter()
                .filter(|ch| {
                    self.sidebar_filter.is_empty()
                        || ch
                            .to_lowercase()
                            .contains(&self.sidebar_filter.to_lowercase())
                })
                .cloned()
                .collect();
            channels.sort_by_key(|ch| channel_sort_key(ch));
            result.extend(channels);
        }
        result
    }

    pub fn toggle_sidebar(&mut self) {
        if !self.sidebar_open {
            self.sidebar_open = true;
            self.sidebar_focused = true;
        } else if !self.sidebar_focused {
            self.sidebar_focused = true;
        } else {
            self.sidebar_open = false;
            self.sidebar_focused = false;
            self.sidebar_filter.clear();
        }
    }

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

    pub fn sidebar_down(&mut self) {
        let count = self.sidebar_channels().len();
        if count == 0 {
            return;
        }
        self.sidebar_selected = (self.sidebar_selected + 1) % count;
    }

    pub fn sidebar_select(&mut self) {
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar_selected) {
            if self.sidebar_selected == 0 {
                self.current_view = None;
                self.send_target = None;
            } else {
                self.current_view = Some(ch.clone());
                self.send_target = Some(ch.clone());
                self.channel_unread.remove(ch);
            }
            self.scroll_offset = 0;
            self.sidebar_focused = false;
            self.sidebar_filter.clear();
        }
        self.sidebar_focused = false;
    }

    pub fn sidebar_collapse(&mut self) {
        if self.project_expanded {
            self.project_expanded = false;
            self.sidebar_selected = 0;
        }
    }

    pub fn sidebar_expand(&mut self) {
        self.project_expanded = true;
    }

    pub fn sidebar_close(&mut self) {
        self.sidebar_focused = false;
        self.sidebar_filter.clear();
    }

    pub fn sidebar_filter_char(&mut self, c: char) {
        self.sidebar_filter.push(c);
        self.sidebar_selected = if self.sidebar_channels().len() > 1 {
            1
        } else {
            0
        };
    }

    pub fn sidebar_filter_backspace(&mut self) {
        self.sidebar_filter.pop();
    }
}
