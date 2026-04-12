use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::app::App;
use super::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_PURPLE, DRACULA_SELECTION,
};

const SECTION_PREFIX_COLLAPSED: &str = "▸ ";
const SECTION_PREFIX_EXPANDED: &str = "▾ ";
const SIDEBAR_NAME_WIDTH: usize = 16;

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) {
    let channels = app.sidebar_channels();
    let items: Vec<Line> = channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let selected = i == app.sidebar_selected && app.sidebar_focused;
            let bg = if selected {
                DRACULA_SELECTION
            } else {
                DRACULA_BG
            };

            // Index 0 = project root node
            if i == 0 {
                let arrow = if app.project_expanded { "▾ " } else { "▸ " };
                let is_all = app.current_view.is_none();
                let name_style = if is_all {
                    Style::default()
                        .fg(DRACULA_FG)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DRACULA_FG).bg(bg)
                };
                let select_marker = if selected { "▸ " } else { "  " };
                return Line::from(vec![
                    Span::styled(select_marker, Style::default().bg(bg).fg(DRACULA_PURPLE)),
                    Span::styled(arrow, Style::default().bg(bg).fg(DRACULA_COMMENT)),
                    Span::styled(
                        ch.chars().take(SIDEBAR_NAME_WIDTH).collect::<String>(),
                        name_style,
                    ),
                ]);
            }

            // Detect section headers vs channel entries
            let is_section = ch.starts_with("▸ ") || ch.starts_with("▾ ");
            let is_current = !is_section && app.current_view.as_deref() == Some(ch.as_str());
            let unread = if ch == "broadcast" || is_section {
                0
            } else {
                app.channel_unread.get(ch).copied().unwrap_or(0)
            };

            let select_marker = if selected { "▸ " } else { "  " };
            let inner_width = area.width.saturating_sub(2) as usize;
            // Section headers: 2-char indent. Children: 4-char indent.
            let indent = if is_section || ch == "broadcast" {
                "  "
            } else {
                "    "
            };
            let prefix_len = 2 + indent.len();
            let available = inner_width.saturating_sub(prefix_len);

            let badge = if unread > 0 {
                format!(" {unread}")
            } else {
                String::new()
            };
            let name_budget = available.saturating_sub(badge.len());
            let display_name: String = if ch.len() > name_budget {
                format!(
                    "{}…",
                    ch.chars()
                        .take(name_budget.saturating_sub(1))
                        .collect::<String>()
                )
            } else {
                ch.clone()
            };

            let name_style = if is_section {
                Style::default().fg(DRACULA_CYAN).bg(bg)
            } else if is_current || unread > 0 {
                Style::default()
                    .fg(DRACULA_FG)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DRACULA_COMMENT).bg(bg)
            };

            let mut spans = vec![
                Span::styled(select_marker, Style::default().bg(bg).fg(DRACULA_PURPLE)),
                Span::styled(indent, Style::default().bg(bg)),
                Span::styled(display_name, name_style),
            ];
            if unread > 0 {
                spans.push(Span::styled(
                    badge,
                    Style::default()
                        .bg(bg)
                        .fg(DRACULA_CYAN)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Line::from(spans)
        })
        .collect();

    let border_color = if app.sidebar_focused {
        DRACULA_PURPLE
    } else {
        DRACULA_SELECTION
    };

    // #183: title gets a `▾` collapse glyph when the sidebar is
    // open (the click target says "click to collapse"). When in
    // filter mode, the title shows the filter text instead — the
    // glyph is omitted to keep the filter label visually distinct
    // from the normal header state.
    let title_text = if app.sidebar_filter.is_empty() {
        format!(" {} channels ", super::icons::COLLAPSE)
    } else {
        format!(" filter: {} ", app.sidebar_filter)
    };

    let sidebar = Paragraph::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title_text)
            .title_style(
                Style::default()
                    .fg(DRACULA_CYAN)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
            .border_style(Style::default().fg(border_color))
            .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG)),
    );
    frame.render_widget(sidebar, area);
}

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
                // On a header: if the section itself is expanded, collapse it.
                // Otherwise fall through to collapsing the project.
                if ch.contains("direct") && self.direct_expanded {
                    self.direct_expanded = false;
                    return;
                }
                if ch.contains("groups") && self.groups_expanded {
                    self.groups_expanded = false;
                    return;
                }
                if ch.contains("system") && self.system_expanded {
                    self.system_expanded = false;
                    return;
                }
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
