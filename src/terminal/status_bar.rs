//! Bottom status bar rendering + stall detection for peer status dots.
//!
//! Extracted from ui.rs to keep that file under the 400-line soft limit and
//! to let the stall-detection logic (the belt-and-suspenders layer for #146)
//! live next to the code that decides what dot color to paint.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::app::App;
use super::ci::{CiGlyphColor, render_indicator as render_ci};
use super::theme::{
    DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_GREEN, DRACULA_ORANGE, DRACULA_PINK,
    DRACULA_RED, DRACULA_SELECTION, DRACULA_YELLOW,
};

const MAX_STATUS_DESC: usize = 15;

/// If a peer has been marked `busy` or `help` for longer than this threshold
/// without emitting a new activity, we treat their status as `stalled` — the
/// plugin hook chain has probably broken (see #146). Belt-and-suspenders
/// layer: survives plugin uninstalls, hook failures, curl errors, port
/// mismatches. Presence shouldn't silently lie when the happy path breaks.
pub(super) const STALL_THRESHOLD: std::time::Duration = std::time::Duration::from_secs(30);

pub(super) fn parse_status_with_age(
    status: &str,
    age: std::time::Duration,
) -> (Color, Option<std::borrow::Cow<'_, str>>) {
    let (color, desc) = parse_status_raw(status);
    // Only stall-check the busy-like statuses. "available" never stalls —
    // an available peer is simply idle, which is the normal expected state
    // for long stretches.
    let is_busy_like = matches!(color, c if c == DRACULA_YELLOW || c == DRACULA_RED);
    if is_busy_like && age >= STALL_THRESHOLD {
        let secs = age.as_secs();
        let label = format!("stalled {secs}s");
        let label = if label.len() > MAX_STATUS_DESC {
            label[..MAX_STATUS_DESC].to_string()
        } else {
            label
        };
        return (DRACULA_ORANGE, Some(std::borrow::Cow::Owned(label)));
    }
    (color, desc.map(std::borrow::Cow::Borrowed))
}

fn ci_color_to_ratatui(color: CiGlyphColor) -> Color {
    match color {
        CiGlyphColor::Green => DRACULA_GREEN,
        CiGlyphColor::Red => DRACULA_RED,
        CiGlyphColor::Yellow => DRACULA_YELLOW,
    }
}

fn parse_status_raw(status: &str) -> (Color, Option<&str>) {
    if let Some(desc) = status.strip_prefix("busy:") {
        let desc = desc.trim();
        if desc.is_empty() {
            (DRACULA_YELLOW, None)
        } else if desc.len() > MAX_STATUS_DESC {
            (DRACULA_YELLOW, Some(&desc[..MAX_STATUS_DESC]))
        } else {
            (DRACULA_YELLOW, Some(desc))
        }
    } else if let Some(desc) = status.strip_prefix("help:") {
        let desc = desc.trim();
        if desc.is_empty() {
            (DRACULA_RED, None)
        } else if desc.len() > MAX_STATUS_DESC {
            (DRACULA_RED, Some(&desc[..MAX_STATUS_DESC]))
        } else {
            (DRACULA_RED, Some(desc))
        }
    } else {
        match status {
            "busy" => (DRACULA_YELLOW, None),
            "help" => (DRACULA_RED, None),
            _ => (DRACULA_GREEN, None),
        }
    }
}

pub(super) fn render(frame: &mut Frame, app: &mut App, area: Rect) {
    let connection_status = if app.peer_activity.is_empty() {
        Span::styled(" solo", Style::default().fg(DRACULA_ORANGE))
    } else {
        Span::raw("")
    };
    // #193: board view label, #191: topic mode label.
    let view_label = if app.show_board {
        if app.board_detail.is_some() {
            let visible = super::board_view::visible_stories(&app.board_stories);
            let idx = app.board_detail.unwrap_or(0);
            let global_idx = visible.get(idx).copied().unwrap_or(0);
            app.board_stories
                .get(global_idx)
                .map(|s| format!(" #{} detail", s.number))
                .unwrap_or_else(|| " board".to_string())
        } else {
            " board view".to_string()
        }
    } else if app.topic_mode() {
        format!(" {}", app.active_topic)
    } else {
        match &app.current_view {
            None => " [all]".to_string(),
            Some(v) if v == "broadcast" => " [broadcast]".to_string(),
            Some(v) => format!(" [{v}]"),
        }
    };
    let mut status_spans = vec![
        Span::styled(view_label, Style::default().fg(DRACULA_CYAN)),
        Span::raw(" "),
        connection_status,
    ];
    // #191: cross-topic unread indicators. Show other topics
    // that have unread messages so the user doesn't miss activity.
    if app.topic_mode() {
        for topic in &app.topics {
            if *topic == app.active_topic {
                continue;
            }
            let unread = app.topic_unreads.get(topic).copied().unwrap_or(0);
            if unread > 0 {
                status_spans.push(Span::raw(" | "));
                status_spans.push(Span::styled(
                    format!("{topic}: {unread} new"),
                    Style::default().fg(DRACULA_ORANGE),
                ));
            }
        }
    }
    // #181: focus-mode indicator — shows the current focus set
    // alongside the view label when `/focus <roles>` is active.
    // Format: `[focus: dev,pm]`. Silent when the filter is off so
    // the bar stays quiet in the common case.
    if let Some(roles) = app.focused_roles.as_ref() {
        status_spans.push(Span::raw(" "));
        status_spans.push(Span::styled(
            format!("[focus: {}]", roles.join(",")),
            Style::default().fg(DRACULA_CYAN),
        ));
    }
    if app.scroll_offset > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("scrolled up {}", app.scroll_offset),
            Style::default().fg(DRACULA_COMMENT),
        ));
    }
    if app.unread_count > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("{} new", app.unread_count),
            Style::default().fg(DRACULA_PINK),
        ));
    }
    // #161: class-filter indicator. Shows "N hidden" when the filter is
    // on and has suppressed messages; "showing all" when the user has
    // toggled the filter off via `/all`. Silent when the filter is on
    // but nothing is currently hidden (avoids cluttering the status bar
    // during channels with no Routine messages).
    if app.show_all_classes {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            "showing all".to_string(),
            Style::default().fg(DRACULA_COMMENT),
        ));
    } else if app.hidden_class_count > 0 {
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("{} hidden", app.hidden_class_count),
            Style::default().fg(DRACULA_COMMENT),
        ));
    }
    let mut peers: Vec<_> = app.peer_activity.iter().collect();
    peers.sort_by_key(|(name, _)| (*name).clone());
    let now = std::time::Instant::now();
    for (peer, (status, last_seen)) in &peers {
        let age = now.saturating_duration_since(*last_seen);
        let (dot_color, description) = parse_status_with_age(status, age);
        status_spans.push(Span::raw(" "));
        status_spans.push(Span::styled("\u{25CF}", Style::default().fg(dot_color)));
        status_spans.push(Span::styled(
            format!(" {peer}"),
            Style::default().fg(DRACULA_FG),
        ));
        if let Some(desc) = description {
            status_spans.push(Span::styled(
                format!(": {desc}"),
                Style::default().fg(DRACULA_COMMENT),
            ));
        }
    }
    // DM unread badge with sender names
    if !app.dm_unread_from.is_empty() {
        let names = app.dm_unread_from.join(", ");
        status_spans.push(Span::raw(" | "));
        status_spans.push(Span::styled(
            format!("\u{2709} DM from {names}"),
            Style::default().fg(DRACULA_YELLOW),
        ));
    }
    // CI indicator (#151). Sits between the presence cluster and the
    // right-aligned keyboard hint — repository state is cognitively
    // separate from presence state, so the spec groups them that way.
    // Hidden when state is Unknown (no indicator at startup until the
    // first poll lands).
    let ci = render_ci(app.ci_state);
    // Width budget includes a leading " " separator to keep it visually
    // distinct from the presence cluster on its left.
    let ci_width = ci
        .as_ref()
        .map(|(label, _)| 1 + label.chars().count())
        .unwrap_or(0);

    // Right-aligned hint region. #183 splits the hint into two
    // visual groups:
    //   - clickable widgets (`?=help`) get cyan + underlined
    //   - keybind-only labels (`⌥1=sidebar`, `Enter=send`) stay in
    //     plain foreground so users can distinguish "click OR key"
    //     from "key only"
    let used: usize = status_spans.iter().map(|s| s.content.chars().count()).sum();
    let bar_width = area.width as usize;
    let (clickable_hint, keybind_hint) = if app.show_board {
        if app.board_detail.is_some() {
            ("", "Esc=back to board")
        } else {
            ("", "\u{2191}\u{2193}=nav Enter=detail Esc=close")
        }
    } else if app.sidebar.focused {
        // In sidebar-focus mode, nothing in the hint is clickable —
        // it's all keybind guidance. Render the whole string in plain
        // foreground.
        ("", "Enter=switch \u{2191}\u{2193}=nav Esc=close")
    } else if app.topic_mode() {
        ("?=help", " \u{2325}1=topics Enter=send")
    } else {
        ("?=help", " \u{2325}1=sidebar Enter=send")
    };
    let hint_len = clickable_hint.chars().count() + keybind_hint.chars().count();
    let right_margin = 1;
    // Reserve two spaces of breathing room between the CI indicator and
    // the help hint when both are shown, so they don't visually fuse.
    let ci_hint_gap = if ci_width > 0 { 2 } else { 0 };
    if used + ci_width + ci_hint_gap + hint_len + right_margin + 1 < bar_width {
        let pad = bar_width.saturating_sub(used + ci_width + ci_hint_gap + hint_len + right_margin);
        // Pad first, then CI indicator, then gap, then hint — the CI
        // glyph floats next to the hint rather than anchoring to the left
        // content. That's what the spec mockup shows.
        status_spans.push(Span::raw(" ".repeat(pad)));
        if let Some((label, color)) = ci {
            status_spans.push(Span::raw(" "));
            // #183: CI indicator is treated as clickable per designer's
            // spec (click handler will eventually open `gh run view`;
            // today it's passive but the affordance is forward-looking),
            // so it gets underlined to match the other interactive
            // status-bar widgets.
            status_spans.push(Span::styled(
                label,
                Style::default()
                    .fg(ci_color_to_ratatui(color))
                    .add_modifier(Modifier::UNDERLINED),
            ));
            status_spans.push(Span::raw(" ".repeat(ci_hint_gap.saturating_sub(1))));
        }
        app.help_hint_col = Some(area.x + (used + pad + ci_width + ci_hint_gap) as u16);
        if !clickable_hint.is_empty() {
            status_spans.push(Span::styled(
                clickable_hint,
                Style::default()
                    .fg(DRACULA_CYAN)
                    .add_modifier(Modifier::UNDERLINED),
            ));
        }
        if !keybind_hint.is_empty() {
            status_spans.push(Span::styled(keybind_hint, Style::default().fg(DRACULA_FG)));
        }
    } else {
        app.help_hint_col = None;
    }
    let status = Paragraph::new(Line::from(status_spans))
        .style(Style::default().bg(DRACULA_SELECTION).fg(DRACULA_FG));
    frame.render_widget(status, area);
}
