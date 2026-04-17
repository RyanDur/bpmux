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

// Section prefixes use centralized glyphs from icons.rs
fn section_prefix_collapsed() -> String {
    format!("{} ", super::icons::EXPAND)
}
fn section_prefix_expanded() -> String {
    format!("{} ", super::icons::COLLAPSE)
}
const SIDEBAR_NAME_WIDTH: usize = 16;
/// Trailing chevron rendered on direct peer rows (#224). Click on
/// this column switches view to that peer's DM channel; click on
/// the name toggles focus. Single-chevron for compactness + the
/// widely-learned "enter" convention.
const SIDEBAR_CHEVRON: &str = "›";
/// Prefix that encodes a thread id in sidebar_channels entries
/// (#275). Strips to the raw id when we need to look up the Thread.
/// Uses `:` so it doesn't collide with role names or `+`-group
/// channels in sidebar_channels.
const THREAD_ROW_PREFIX: &str = "thread:";
/// Visual glyph for thread rows — distinguishes them from direct/
/// group channels at a glance (designer UX call).
const SIDEBAR_THREAD_GLYPH: &str = "🧵";

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) {
    let channels = app.sidebar_channels();
    let items: Vec<Line> = render_channel_items(&channels, app, area);

    let border_color = if app.sidebar.focused {
        DRACULA_PURPLE
    } else {
        DRACULA_SELECTION
    };

    let title_text = if !app.sidebar.filter.is_empty() {
        format!(" filter: {} ", app.sidebar.filter)
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
            // Thread rows encode the thread id after `thread:` —
            // resolve to the Thread for display + unread (#275).
            let thread_for_row = ch
                .strip_prefix(THREAD_ROW_PREFIX)
                .and_then(|id| app.threads.known.get(id));
            let is_current =
                !is_section && app.channels.current_view.as_deref() == Some(ch.as_str());
            let unread = if ch == "broadcast" || is_section {
                0
            } else if let Some(t) = thread_for_row {
                t.unread
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
            // Thread rows substitute the thread name for the id and
            // prepend a distinguishing glyph (#275); other rows
            // render the channel string directly.
            let raw_name = match thread_for_row {
                Some(t) => format!("{SIDEBAR_THREAD_GLYPH} {}", t.name),
                None => ch.clone(),
            };
            let display_name: String = if raw_name.chars().count() > name_budget {
                format!(
                    "{}…",
                    raw_name
                        .chars()
                        .take(name_budget.saturating_sub(1))
                        .collect::<String>()
                )
            } else {
                raw_name
            };

            // Focus (#224): direct peer rows render focused-peers in
            // cyan-bold. The same style matches the status-bar
            // `[focus: ...]` indicator so the two surfaces read as one
            // filter state.
            let is_direct_peer = is_direct_peer_row(ch);
            let is_focused = is_direct_peer
                && app
                    .focused_roles
                    .as_deref()
                    .is_some_and(|roles| roles.iter().any(|r| r == ch));

            let name_style = if is_focused {
                Style::default()
                    .fg(DRACULA_CYAN)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else if is_section {
                Style::default().fg(DRACULA_CYAN).bg(bg)
            } else if is_current || unread > 0 {
                Style::default()
                    .fg(DRACULA_FG)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(DRACULA_COMMENT).bg(bg)
            };

            let name_len = display_name.len();
            let badge_len = badge.len();
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
            // Chevron click-target (#224) — always visible on direct
            // peer rows so "switch to this peer's DM view" stays
            // accessible regardless of unread state. Mouse click on
            // the chevron column routes to `switch_view_to`; clicks
            // elsewhere on the row toggle focus.
            if is_direct_peer {
                let used = prefix_len + name_len + badge_len;
                let pad = inner_width.saturating_sub(used).saturating_sub(1);
                if pad > 0 {
                    spans.push(Span::styled(" ".repeat(pad), Style::default().bg(bg)));
                }
                spans.push(Span::styled(
                    SIDEBAR_CHEVRON,
                    Style::default().bg(bg).fg(DRACULA_COMMENT),
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

/// A direct-peer sidebar row (#224) — a single-role entry, not a
/// section header, not a group, not a system channel, not broadcast,
/// not a thread row (#275), not the self-root. These rows carry the
/// focus-toggle + chevron affordances; everything else keeps the
/// view-switch behavior.
fn is_direct_peer_row(entry: &str) -> bool {
    !is_section_header(entry)
        && entry != "broadcast"
        && !is_group_channel(entry)
        && !is_system_channel(entry)
        && !entry.starts_with(THREAD_ROW_PREFIX)
}

impl App {
    pub fn sidebar_channels(&self) -> Vec<String> {
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

        // Threads where self is a participant (#275). Rows encode
        // the thread id with a `thread:` prefix so sidebar_select
        // can reverse-lookup without colliding on channel names
        // (colons don't appear in role names or `+`-separated
        // group channels). Freshest activity first.
        let mut threads: Vec<&super::app::Thread> = self
            .threads
            .known
            .values()
            .filter(|t| t.participants.iter().any(|p| p == &self.identity.name))
            .collect();
        threads.sort_by_key(|t| std::cmp::Reverse(t.last_activity));
        let thread_rows: Vec<String> = threads
            .into_iter()
            .filter(|t| matches_filter(&t.name))
            .map(|t| format!("{THREAD_ROW_PREFIX}{}", t.id))
            .collect();

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

        // Threads section (#275)
        if !thread_rows.is_empty() {
            result.push(section_header(
                "threads",
                thread_rows.len(),
                self.sidebar.threads_expanded,
            ));
            if self.sidebar.threads_expanded {
                result.extend(thread_rows);
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
        self.sidebar_select_inner(false);
    }

    /// Mouse-click variant: `on_chevron` signals that the click
    /// landed on the trailing `›` column. For direct peer rows, the
    /// chevron triggers the view-switch behavior; the rest of the
    /// row toggles focus (#224). Non-peer rows (section headers,
    /// self, broadcast, groups, system) ignore `on_chevron` and
    /// behave identically to `sidebar_select`.
    pub fn sidebar_select_at(&mut self, on_chevron: bool) {
        self.sidebar_select_inner(on_chevron);
    }

    fn sidebar_select_inner(&mut self, on_chevron: bool) {
        let channels = self.sidebar_channels();
        if let Some(ch) = channels.get(self.sidebar.selected) {
            // Toggle section headers
            if is_section_header(ch) {
                if ch.contains("direct") {
                    self.sidebar.direct_expanded = !self.sidebar.direct_expanded;
                } else if ch.contains("groups") {
                    self.sidebar.groups_expanded = !self.sidebar.groups_expanded;
                } else if ch.contains("threads") {
                    self.sidebar.threads_expanded = !self.sidebar.threads_expanded;
                } else if ch.contains("system") {
                    self.sidebar.system_expanded = !self.sidebar.system_expanded;
                }
                return;
            }
            // Thread rows (#275): click switches view into the thread.
            // The id is encoded after `thread:`; look up the thread
            // and flip `threads.current`. Unread resets on entry.
            if let Some(id) = ch.strip_prefix(THREAD_ROW_PREFIX) {
                let id = id.to_string();
                if self.threads.known.contains_key(&id) {
                    self.threads.current = Some(id.clone());
                    self.threads.broadcast_unread_while_threaded = 0;
                    if let Some(t) = self.threads.known.get_mut(&id) {
                        t.unread = 0;
                    }
                    self.mark_filters_dirty();
                }
                self.sidebar.focused = false;
                self.sidebar.filter.clear();
                return;
            }
            // Direct peer rows have two click zones (#224):
            // name → toggle focus, chevron → switch view. Keyboard
            // Enter hits the name path (the primary action per spec).
            if self.sidebar.selected != 0 && is_direct_peer_row(ch) {
                let ch = ch.clone();
                if on_chevron {
                    self.switch_view_to(Some(ch.clone()));
                    self.channels.send_target = Some(ch.clone());
                    self.channels.unread.remove(&ch);
                } else {
                    self.toggle_focus_on_peer(&ch);
                }
                self.sidebar.focused = false;
                self.sidebar.filter.clear();
                return;
            }
            // Route the view switch through the shared
            // `switch_view_to` helper so click and typed `/view`
            // produce identical filter state. Context additions
            // (send_target, unread) stay here.
            if self.sidebar.selected == 0 {
                self.switch_view_to(None);
                self.channels.send_target = None;
            } else {
                self.switch_view_to(Some(ch.clone()));
                self.channels.send_target = Some(ch.clone());
                self.channels.unread.remove(ch);
            }
            self.sidebar.focused = false;
            self.sidebar.filter.clear();
        }
        self.sidebar.focused = false;
    }

    /// Toggle `peer` in `focused_roles` (#224). If present, remove
    /// it; if absent, add it. When removing empties the list,
    /// collapses `focused_roles` to `None` so downstream filters
    /// match `/focus off` semantics. Mirrors the observable effect
    /// of typing `/focus <peer>` so keyboard and click paths stay
    /// equivalent.
    fn toggle_focus_on_peer(&mut self, peer: &str) {
        let mut roles = self.focused_roles.take().unwrap_or_default();
        if let Some(idx) = roles.iter().position(|r| r == peer) {
            roles.remove(idx);
        } else {
            roles.push(peer.to_string());
        }
        let message = if roles.is_empty() {
            "Focus cleared.".to_string()
        } else {
            format!("Focus: {}.", roles.join(", "))
        };
        self.focused_roles = if roles.is_empty() { None } else { Some(roles) };
        self.mark_filters_dirty();
        self.system_message(message);
    }

    pub fn sidebar_collapse(&mut self) {
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
