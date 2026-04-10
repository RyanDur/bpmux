//! Bottom status bar rendering + stall detection for peer status dots.
//!
//! Extracted from ui.rs to keep that file under the 400-line soft limit and
//! to let the stall-detection logic (the belt-and-suspenders layer for #146)
//! live next to the code that decides what dot color to paint.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::app::App;
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
    let view_label = match &app.current_view {
        None => " [all]".to_string(),
        Some(v) if v == "broadcast" => " [broadcast]".to_string(),
        Some(v) => format!(" [{v}]"),
    };
    let mut status_spans = vec![
        Span::styled(view_label, Style::default().fg(DRACULA_CYAN)),
        Span::raw(" "),
        connection_status,
    ];
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
    // Right-aligned "? help" hint
    let used: usize = status_spans.iter().map(|s| s.content.chars().count()).sum();
    let bar_width = area.width as usize;
    let hint = if app.sidebar_focused {
        "Enter=switch \u{2191}\u{2193}=nav Esc=close"
    } else {
        "?=help \u{2325}1=sidebar Enter=send"
    };
    let right_margin = 1;
    if used + hint.chars().count() + right_margin + 1 < bar_width {
        let pad = bar_width.saturating_sub(used + hint.chars().count() + right_margin);
        app.help_hint_col = Some(area.x + (used + pad) as u16);
        status_spans.push(Span::raw(" ".repeat(pad)));
        status_spans.push(Span::styled(hint, Style::default().fg(DRACULA_CYAN)));
    } else {
        app.help_hint_col = None;
    }
    let status = Paragraph::new(Line::from(status_spans))
        .style(Style::default().bg(DRACULA_SELECTION).fg(DRACULA_FG));
    frame.render_widget(status, area);
}
