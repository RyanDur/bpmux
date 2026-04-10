use super::app::App;

const SECTION_PREFIX_COLLAPSED: &str = "▸ ";
const SECTION_PREFIX_EXPANDED: &str = "▾ ";

fn is_system_channel(channel: &str) -> bool {
    channel.contains("bpmux") && channel != "broadcast"
}

fn is_group_channel(channel: &str) -> bool {
    channel.contains('+')
}

fn section_header(name: &str, count: usize, expanded: bool) -> String {
    let prefix = if expanded {
        SECTION_PREFIX_EXPANDED
    } else {
        SECTION_PREFIX_COLLAPSED
    };
    format!("{prefix}{name} ({count})")
}

fn is_section_header(entry: &str) -> bool {
    entry.starts_with(SECTION_PREFIX_COLLAPSED) || entry.starts_with(SECTION_PREFIX_EXPANDED)
}

impl App {
    pub fn sidebar_channels(&self) -> Vec<String> {
        let mut result = vec![self.session_id.clone()];
        if !self.project_expanded {
            return result;
        }

        let filter = self.sidebar_filter.to_lowercase();
        let matches_filter = |ch: &str| filter.is_empty() || ch.to_lowercase().contains(&filter);

        // Broadcast always at top level
        if self.active_channels.contains("broadcast") && matches_filter("broadcast") {
            result.push("broadcast".to_string());
        }

        // Build channels from connected peers, not message history
        let connected: std::collections::HashSet<&str> =
            self.peer_activity.keys().map(|s| s.as_str()).collect();

        // Direct: connected agents + channels with unread messages
        let mut direct_set: std::collections::HashSet<String> = connected
            .iter()
            .filter(|name| matches_filter(name))
            .map(|s| s.to_string())
            .collect();
        for (ch, &count) in &self.channel_unread {
            if count > 0
                && !is_group_channel(ch)
                && !is_system_channel(ch)
                && ch != "broadcast"
                && matches_filter(ch)
            {
                direct_set.insert(ch.clone());
            }
        }
        let mut direct: Vec<String> = direct_set.into_iter().collect();

        // Groups: from active_channels, but only where all participants are connected
        let mut groups: Vec<String> = self
            .active_channels
            .iter()
            .filter(|ch| {
                is_group_channel(ch)
                    && !is_system_channel(ch)
                    && matches_filter(ch)
                    && ch.split('+').all(|name| connected.contains(name))
            })
            .cloned()
            .collect();

        // System: bpmux channels from active_channels
        let mut system: Vec<String> = self
            .active_channels
            .iter()
            .filter(|ch| is_system_channel(ch) && matches_filter(ch))
            .cloned()
            .collect();

        direct.sort();
        groups.sort();
        system.sort();

        // Direct section
        if !direct.is_empty() {
            result.push(section_header("direct", direct.len(), self.direct_expanded));
            if self.direct_expanded {
                result.extend(direct);
            }
        }

        // Groups section
        if !groups.is_empty() {
            result.push(section_header("groups", groups.len(), self.groups_expanded));
            if self.groups_expanded {
                result.extend(groups);
            }
        }

        // System section
        if !system.is_empty() {
            result.push(section_header("system", system.len(), self.system_expanded));
            if self.system_expanded {
                result.extend(system);
            }
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
            // Toggle section headers
            if is_section_header(ch) {
                if ch.contains("direct") {
                    self.direct_expanded = !self.direct_expanded;
                } else if ch.contains("groups") {
                    self.groups_expanded = !self.groups_expanded;
                } else if ch.contains("system") {
                    self.system_expanded = !self.system_expanded;
                }
                return;
            }
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
        // Collapse the section the selected item belongs to, not the whole tree
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar_selected) {
            if is_section_header(ch) {
                // Already on a header — collapse the project
                self.project_expanded = false;
                self.sidebar_selected = 0;
                return;
            }
            // Find which section this item is in by looking backwards for a header
            for i in (0..self.sidebar_selected).rev() {
                if let Some(header) = channels.get(i)
                    && is_section_header(header)
                {
                    if header.contains("direct") {
                        self.direct_expanded = false;
                    } else if header.contains("groups") {
                        self.groups_expanded = false;
                    } else if header.contains("system") {
                        self.system_expanded = false;
                    }
                    self.sidebar_selected = i;
                    return;
                }
            }
        }
        // Fallback: collapse project
        if self.project_expanded {
            self.project_expanded = false;
            self.sidebar_selected = 0;
        }
    }

    pub fn sidebar_expand(&mut self) {
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar_selected)
            && is_section_header(ch)
        {
            if ch.contains("direct") {
                self.direct_expanded = true;
            } else if ch.contains("groups") {
                self.groups_expanded = true;
            } else if ch.contains("system") {
                self.system_expanded = true;
            }
            return;
        }
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
