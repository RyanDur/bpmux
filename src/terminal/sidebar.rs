use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::app::App;
use super::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_ORANGE, DRACULA_PURPLE,
    DRACULA_SELECTION,
};

// Section prefixes use centralized glyphs from icons.rs
fn section_prefix_collapsed() -> String {
    format!("{} ", super::icons::EXPAND)
}
fn section_prefix_expanded() -> String {
    format!("{} ", super::icons::COLLAPSE)
}
const SIDEBAR_NAME_WIDTH: usize = 16;

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) {
    let channels = app.sidebar_channels();

    // #191: topic mode uses a flat list with ● active markers.
    let items: Vec<Line> = if app.topic_mode() {
        render_topic_items(&channels, app, area)
    } else {
        render_channel_items(&channels, app, area)
    };

    let border_color = if app.sidebar.focused {
        DRACULA_PURPLE
    } else {
        DRACULA_SELECTION
    };

    // #191: title says "topics" when in topic mode.
    let title_text = if !app.sidebar.filter.is_empty() {
        format!(" filter: {} ", app.sidebar.filter)
    } else if app.topic_mode() {
        format!(" {} topics ", super::icons::COLLAPSE)
    } else {
        format!(" {} channels ", super::icons::COLLAPSE)
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

/// Render sidebar items in topic mode: flat list with ● active
/// marker, (N) unread counts, per-mockup Dracula colors.
fn render_topic_items(topics: &[String], app: &App, area: Rect) -> Vec<Line<'static>> {
    let inner_width = area.width.saturating_sub(2) as usize;
    topics
        .iter()
        .enumerate()
        .map(|(i, topic)| {
            let selected = i == app.sidebar.selected && app.sidebar.focused;
            let bg = if selected {
                DRACULA_SELECTION
            } else {
                DRACULA_BG
            };
            let is_active = *topic == app.topic.active;
            let unread = app.topic.unreads.get(topic).copied().unwrap_or(0);

            let marker = if is_active {
                format!("{} ", super::icons::ACTIVE)
            } else {
                "  ".to_string()
            };
            let badge = if unread > 0 {
                format!("  ({unread})")
            } else {
                String::new()
            };

            // Truncate topic name if needed
            let prefix_len = 2 + marker.len(); // select_marker + marker
            let available = inner_width.saturating_sub(prefix_len + badge.len());
            let display_name: String = if topic.len() > available {
                format!(
                    "{}…",
                    topic
                        .chars()
                        .take(available.saturating_sub(1))
                        .collect::<String>()
                )
            } else {
                topic.clone()
            };

            // Colors per design spec:
            // - Active: DRACULA_CYAN, bold
            // - Unreads: DRACULA_FG, bold (name) + DRACULA_ORANGE (count)
            // - Quiet: DRACULA_COMMENT (dim)
            let name_style = if is_active {
                Style::default()
                    .fg(DRACULA_CYAN)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else if unread > 0 {
                Style::default()
                    .fg(DRACULA_FG)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DRACULA_COMMENT).bg(bg)
            };

            let select_marker = if selected {
                format!("{} ", super::icons::EXPAND)
            } else {
                "  ".to_string()
            };
            let mut spans = vec![
                Span::styled(select_marker, Style::default().bg(bg).fg(DRACULA_PURPLE)),
                Span::styled(
                    marker,
                    Style::default()
                        .bg(bg)
                        .fg(if is_active { DRACULA_CYAN } else { DRACULA_BG }),
                ),
                Span::styled(display_name, name_style),
            ];
            if unread > 0 {
                spans.push(Span::styled(
                    badge,
                    Style::default()
                        .bg(bg)
                        .fg(DRACULA_ORANGE)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            Line::from(spans)
        })
        .collect()
}

/// Render sidebar items in channel mode: the existing channel tree
/// with sections (direct, groups, system).
fn render_channel_items(channels: &[String], app: &App, area: Rect) -> Vec<Line<'static>> {
    channels
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let selected = i == app.sidebar.selected && app.sidebar.focused;
            let bg = if selected {
                DRACULA_SELECTION
            } else {
                DRACULA_BG
            };

            // Index 0 = project root node
            if i == 0 {
                let arrow = if app.sidebar.project_expanded {
                    format!("{} ", super::icons::COLLAPSE)
                } else {
                    format!("{} ", super::icons::EXPAND)
                };
                let is_all = app.channels.current_view.is_none();
                let name_style = if is_all {
                    Style::default()
                        .fg(DRACULA_FG)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(DRACULA_FG).bg(bg)
                };
                let select_marker = if selected {
                    format!("{} ", super::icons::EXPAND)
                } else {
                    "  ".to_string()
                };
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
            let is_section = ch.starts_with(&section_prefix_collapsed())
                || ch.starts_with(&section_prefix_expanded());
            let is_current =
                !is_section && app.channels.current_view.as_deref() == Some(ch.as_str());
            let unread = if ch == "broadcast" || is_section {
                0
            } else {
                app.channels.unread.get(ch).copied().unwrap_or(0)
            };

            let select_marker = if selected {
                format!("{} ", super::icons::EXPAND)
            } else {
                "  ".to_string()
            };
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
        .collect()
}

fn is_system_channel(channel: &str) -> bool {
    channel.contains("bpmux") && channel != "broadcast"
}

fn is_group_channel(channel: &str) -> bool {
    channel.contains('+')
}

fn section_header(name: &str, count: usize, expanded: bool) -> String {
    let prefix = if expanded {
        section_prefix_expanded()
    } else {
        section_prefix_collapsed()
    };
    format!("{prefix}{name} ({count})")
}

fn is_section_header(entry: &str) -> bool {
    entry.starts_with(&section_prefix_collapsed()) || entry.starts_with(&section_prefix_expanded())
}

impl App {
    /// Whether the sidebar is in topic mode — topics exist beyond
    /// the default "general". When true, the sidebar shows topics
    /// instead of the channel tree.
    pub fn topic_mode(&self) -> bool {
        self.topic.topics.len() > 1
    }

    pub fn sidebar_channels(&self) -> Vec<String> {
        // #191: when topics exist, the sidebar shows topics instead
        // of the channel tree. Flat list, no sections.
        if self.topic_mode() {
            let filter = self.sidebar.filter.to_lowercase();
            return self
                .topic
                .topics
                .iter()
                .filter(|t| filter.is_empty() || t.to_lowercase().contains(&filter))
                .cloned()
                .collect();
        }

        let mut result = vec![self.identity.id.clone()];
        if !self.sidebar.project_expanded {
            return result;
        }

        let filter = self.sidebar.filter.to_lowercase();
        let matches_filter = |ch: &str| filter.is_empty() || ch.to_lowercase().contains(&filter);

        // Broadcast always at top level
        if self.channels.active.contains("broadcast") && matches_filter("broadcast") {
            result.push("broadcast".to_string());
        }

        // Build channels from connected peers, not message history
        let connected: std::collections::HashSet<&str> =
            self.peers.activity.keys().map(|s| s.as_str()).collect();

        // Direct: connected agents + channels with unread messages
        let mut direct_set: std::collections::HashSet<String> = connected
            .iter()
            .filter(|name| matches_filter(name))
            .map(|s| s.to_string())
            .collect();
        for (ch, &count) in &self.channels.unread {
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
            .channels
            .active
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
            .channels
            .active
            .iter()
            .filter(|ch| is_system_channel(ch) && matches_filter(ch))
            .cloned()
            .collect();

        direct.sort();
        groups.sort();
        system.sort();

        // Direct section
        if !direct.is_empty() {
            result.push(section_header(
                "direct",
                direct.len(),
                self.sidebar.direct_expanded,
            ));
            if self.sidebar.direct_expanded {
                result.extend(direct);
            }
        }

        // Groups section
        if !groups.is_empty() {
            result.push(section_header(
                "groups",
                groups.len(),
                self.sidebar.groups_expanded,
            ));
            if self.sidebar.groups_expanded {
                result.extend(groups);
            }
        }

        // System section
        if !system.is_empty() {
            result.push(section_header(
                "system",
                system.len(),
                self.sidebar.system_expanded,
            ));
            if self.sidebar.system_expanded {
                result.extend(system);
            }
        }

        result
    }

    pub fn toggle_sidebar(&mut self) {
        if !self.sidebar.open {
            self.sidebar.open = true;
            self.sidebar.focused = true;
            self.input_mode = super::app::InputMode::Sidebar;
        } else if !self.sidebar.focused {
            self.sidebar.focused = true;
            self.input_mode = super::app::InputMode::Sidebar;
        } else {
            self.sidebar.open = false;
            self.sidebar.focused = false;
            self.sidebar.filter.clear();
            self.input_mode = super::app::InputMode::Chat;
        }
    }

    pub fn sidebar_up(&mut self) {
        let count = self.sidebar_channels().len();
        if count == 0 {
            return;
        }
        self.sidebar.selected = if self.sidebar.selected == 0 {
            count - 1
        } else {
            self.sidebar.selected - 1
        };
    }

    pub fn sidebar_down(&mut self) {
        let count = self.sidebar_channels().len();
        if count == 0 {
            return;
        }
        self.sidebar.selected = (self.sidebar.selected + 1) % count;
    }

    pub fn sidebar_select(&mut self) {
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar.selected) {
            // #191: topic mode — selecting a topic switches active_topic.
            if self.topic_mode() {
                self.topic.active = ch.clone();
                self.mark_filters_dirty();
                self.topic.unreads.remove(ch);
                self.channels.current_view = None;
                self.scroll.offset = 0;
                self.sidebar.focused = false;
                self.sidebar.filter.clear();
                return;
            }

            // Toggle section headers
            if is_section_header(ch) {
                if ch.contains("direct") {
                    self.sidebar.direct_expanded = !self.sidebar.direct_expanded;
                } else if ch.contains("groups") {
                    self.sidebar.groups_expanded = !self.sidebar.groups_expanded;
                } else if ch.contains("system") {
                    self.sidebar.system_expanded = !self.sidebar.system_expanded;
                }
                return;
            }
            if self.sidebar.selected == 0 {
                self.channels.current_view = None;
                self.mark_filters_dirty();
                self.channels.send_target = None;
            } else {
                self.channels.current_view = Some(ch.clone());
                self.mark_filters_dirty();
                self.channels.send_target = Some(ch.clone());
                self.channels.unread.remove(ch);
            }
            self.scroll.offset = 0;
            self.sidebar.focused = false;
            self.sidebar.filter.clear();
        }
        self.sidebar.focused = false;
    }

    pub fn sidebar_collapse(&mut self) {
        // Topic mode has no sections to collapse.
        if self.topic_mode() {
            return;
        }
        // Collapse the section the selected item belongs to, not the whole tree
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar.selected) {
            if is_section_header(ch) {
                // On a header: if the section itself is expanded, collapse it.
                // Otherwise fall through to collapsing the project.
                if ch.contains("direct") && self.sidebar.direct_expanded {
                    self.sidebar.direct_expanded = false;
                    return;
                }
                if ch.contains("groups") && self.sidebar.groups_expanded {
                    self.sidebar.groups_expanded = false;
                    return;
                }
                if ch.contains("system") && self.sidebar.system_expanded {
                    self.sidebar.system_expanded = false;
                    return;
                }
                self.sidebar.project_expanded = false;
                self.sidebar.selected = 0;
                return;
            }
            // Find which section this item is in by looking backwards for a header
            for i in (0..self.sidebar.selected).rev() {
                if let Some(header) = channels.get(i)
                    && is_section_header(header)
                {
                    if header.contains("direct") {
                        self.sidebar.direct_expanded = false;
                    } else if header.contains("groups") {
                        self.sidebar.groups_expanded = false;
                    } else if header.contains("system") {
                        self.sidebar.system_expanded = false;
                    }
                    self.sidebar.selected = i;
                    return;
                }
            }
        }
        // Fallback: collapse project
        if self.sidebar.project_expanded {
            self.sidebar.project_expanded = false;
            self.sidebar.selected = 0;
        }
    }

    pub fn sidebar_expand(&mut self) {
        // Topic mode has no sections to expand.
        if self.topic_mode() {
            return;
        }
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar.selected)
            && is_section_header(ch)
        {
            if ch.contains("direct") {
                self.sidebar.direct_expanded = true;
            } else if ch.contains("groups") {
                self.sidebar.groups_expanded = true;
            } else if ch.contains("system") {
                self.sidebar.system_expanded = true;
            }
            return;
        }
        self.sidebar.project_expanded = true;
    }

    pub fn sidebar_close(&mut self) {
        self.sidebar.focused = false;
        self.sidebar.filter.clear();
        self.input_mode = super::app::InputMode::Chat;
    }

    pub fn sidebar_filter_char(&mut self, c: char) {
        self.sidebar.filter.push(c);
        self.sidebar.selected = if self.sidebar_channels().len() > 1 {
            1
        } else {
            0
        };
    }

    pub fn sidebar_filter_backspace(&mut self) {
        self.sidebar.filter.pop();
    }
}
