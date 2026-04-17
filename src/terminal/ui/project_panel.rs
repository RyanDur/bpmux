use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::metrics::{fmt_opt_hours, fmt_opt_rate};
use crate::terminal::app::App;
use crate::terminal::theme::{
    DRACULA_BG, DRACULA_COMMENT, DRACULA_CYAN, DRACULA_FG, DRACULA_GREEN, DRACULA_ORANGE,
    DRACULA_PURPLE, DRACULA_SELECTION, DRACULA_YELLOW,
};

pub(super) fn render_project_panel(frame: &mut Frame, app: &App, area: Rect) {
    let (snapshot_stories, last_updated, last_error) = {
        let snap = app.board_snapshot.lock().ok();
        match snap {
            Some(s) => (s.stories.clone(), s.last_updated, s.last_error.clone()),
            None => (Vec::new(), None, None),
        }
    };
    let display_stories = if !snapshot_stories.is_empty() {
        snapshot_stories
    } else {
        app.board.stories.clone()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" project ")
        .title_style(
            Style::default()
                .fg(DRACULA_PURPLE)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(DRACULA_SELECTION))
        .style(Style::default().bg(DRACULA_BG).fg(DRACULA_FG));

    let mut lines: Vec<Line> = Vec::new();

    // When the poller can't authenticate (missing or rejected token),
    // hide velocity + DORA and point at the manual /board fallback.
    let auth_missing = matches!(
        last_error,
        Some(crate::terminal::board_poller::BoardFetchError::AuthInvalid)
    );

    if auth_missing {
        append_auth_missing_banner(&mut lines);
        lines.push(Line::from(""));
    } else {
        append_velocity_zone(&mut lines, app);
        lines.push(Line::from(""));
        append_dora_zone(&mut lines, app);
        lines.push(Line::from(""));
    }

    append_board_zone(
        &mut lines,
        &display_stories,
        last_error.as_ref(),
        area.width,
    );
    if auth_missing {
        lines.push(Line::from(Span::styled(
            "  (use /board to refresh)",
            Style::default().fg(DRACULA_COMMENT),
        )));
    }

    lines.push(Line::from(""));
    append_project_footer(&mut lines, last_updated, last_error.as_ref());

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn append_auth_missing_banner(lines: &mut Vec<Line<'_>>) {
    let orange = Style::default().fg(DRACULA_ORANGE);
    lines.push(Line::from(Span::styled("⚠ GITHUB_TOKEN not set", orange)));
    lines.push(Line::from(Span::styled(
        "   — limited visibility; falling back to",
        orange,
    )));
    lines.push(Line::from(Span::styled(
        "   manual board fetch via /board",
        orange,
    )));
}

/// Velocity zone — delivered/day, medians, rework rate. Pure render
/// over `App`'s message log via `session::velocity::compute`. Shows
/// "insufficient data" when below the sparse threshold. One metric
/// per row — narrow-width panels can't fit two-per-row alignment.
fn append_velocity_zone(lines: &mut Vec<Line<'_>>, app: &App) {
    use crate::session::velocity::{compute, is_feature_verdict};
    // Build a lookup from the board-poller snapshot so velocity
    // counts only business-value stories. If the snapshot is empty
    // (auth missing, first poll hasn't completed), every delivered
    // event falls through the `is_feature` check and the metric
    // shows "insufficient data" honestly — better than inflating
    // velocity with unclassified work.
    let snap = app.board_snapshot.lock().ok();
    let stories: Vec<crate::terminal::board::BoardStory> =
        snap.as_ref().map(|s| s.stories.clone()).unwrap_or_default();
    let is_feature = |number: u64| -> bool {
        stories
            .iter()
            .find(|s| s.number == number)
            .map(|s| is_feature_verdict(&s.labels, &s.title))
            .unwrap_or(false)
    };
    let metrics = compute(&app.msg.messages, chrono::Utc::now(), is_feature);
    lines.push(purple_bold_heading("★ velocity (14d)"));
    if metrics.sparse {
        lines.push(Line::from(Span::styled(
            format!(
                "  insufficient data ({}/3 delivered)",
                metrics.delivered_in_window
            ),
            Style::default().fg(DRACULA_COMMENT),
        )));
        return;
    }
    push_metric(
        lines,
        "delivered/day",
        &format!("{:.1}", metrics.delivered_per_day),
    );
    push_metric(
        lines,
        "pickup→deliver",
        &fmt_opt_hours(metrics.median_pickup_to_deliver_hours),
    );
    push_metric(
        lines,
        "deliver→accept",
        &fmt_opt_hours(metrics.median_deliver_to_accept_hours),
    );
    push_metric(lines, "rework rate", &fmt_opt_rate(metrics.rework_rate));
}

/// DORA zone — accepted-week, in-progress, rejection rate, cycle
/// time. Reads cached metrics from `app.project.dora`. One metric
/// per row, same as velocity.
fn append_dora_zone(lines: &mut Vec<Line<'_>>, app: &App) {
    lines.push(purple_bold_heading("★ DORA (30d)"));
    match &app.project.dora {
        Some(m) => {
            push_metric(lines, "shipped this week", &m.shipped_this_week.to_string());
            push_metric(lines, "in progress", &m.in_progress.to_string());
            push_metric(lines, "cycle time", &fmt_opt_hours(m.avg_cycle_hours));
            push_metric(lines, "rejection rate", &fmt_opt_rate(m.rejection_rate));
        }
        None => {
            // DORA cache empty — mirror the board zone's three-state
            // distinction (#219 re-work). Loading vs stuck is the
            // difference between "poller hasn't responded yet" and
            // "poller returned an error and we have no data to show."
            // last_error is surfaced by the footer; keep the cache
            // row short so the view doesn't double-announce.
            lines.push(Line::from(Span::styled(
                "  loading...",
                Style::default().fg(DRACULA_COMMENT),
            )));
        }
    }
}

fn purple_bold_heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default()
            .fg(DRACULA_PURPLE)
            .add_modifier(Modifier::BOLD),
    ))
}

/// Emit a `  label:   value` row with label padded to a consistent
/// column so values line up across a zone. Shared by velocity + DORA.
fn push_metric(lines: &mut Vec<Line<'_>>, label: &str, value: &str) {
    const LABEL_WIDTH: usize = 18;
    let mut prefix = format!("  {label}:");
    while prefix.chars().count() < LABEL_WIDTH + 2 {
        prefix.push(' ');
    }
    lines.push(Line::from(vec![
        Span::styled(prefix, Style::default().fg(DRACULA_FG)),
        Span::styled(value.to_string(), Style::default().fg(DRACULA_CYAN)),
    ]));
}

/// Board zone — story groups with counts + titles. Three visually
/// distinguishable empty-state variants (#219 re-work): fetch in
/// flight ("loading..."), fetch failed ("board unavailable: …" with
/// actionable error), and the success path (story groups).
fn append_board_zone(
    lines: &mut Vec<Line<'_>>,
    stories: &[crate::terminal::board::BoardStory],
    last_error: Option<&crate::terminal::board_poller::BoardFetchError>,
    width: u16,
) {
    use crate::terminal::board::{StoryState, stories_by_state};
    if stories.is_empty() {
        lines.push(purple_bold_heading("★ board"));
        if let Some(err) = last_error {
            lines.push(Line::from(Span::styled(
                format!("  board unavailable: {}", err.user_message()),
                Style::default().fg(DRACULA_ORANGE),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "  loading...",
                Style::default().fg(DRACULA_COMMENT),
            )));
        }
        return;
    }
    let groups = stories_by_state(stories);
    lines.push(purple_bold_heading("★ board"));
    for (state, group_stories) in &groups {
        let color = match state {
            StoryState::Ready => DRACULA_GREEN,
            StoryState::InProgress => DRACULA_ORANGE,
            StoryState::Delivered => DRACULA_YELLOW,
            _ => DRACULA_COMMENT,
        };
        // Every section (including Accepted + Icebox) surfaces its
        // top 5 stories so recent accepts stay visible without a
        // click-to-expand affordance (#219 rework defect 2). Accepted
        // stories are ordered by priority within the section but get
        // re-sorted here to newest-number-first so recent accepts
        // surface to the top; number is a good proxy for recency
        // since issue numbers are monotonic.
        let mut display: Vec<&crate::terminal::board::BoardStory> = group_stories.clone();
        if matches!(state, StoryState::Accepted | StoryState::Icebox) {
            display.sort_by_key(|s| std::cmp::Reverse(s.number));
        }
        lines.push(Line::from(Span::styled(
            format!("  {} ({})", state.label(), group_stories.len()),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )));
        for story in display.iter().take(5) {
            lines.push(Line::from(Span::styled(
                format!("    {}", story.short_name(width.saturating_sub(6) as usize)),
                Style::default().fg(DRACULA_FG),
            )));
        }
        if group_stories.len() > 5 {
            lines.push(Line::from(Span::styled(
                format!("    +{} more", group_stories.len() - 5),
                Style::default().fg(DRACULA_COMMENT),
            )));
        }
    }
}

/// Footer line — freshness timestamp + any fetch error from the
/// board poller's last attempt.
fn append_project_footer(
    lines: &mut Vec<Line<'_>>,
    last_updated: Option<chrono::DateTime<chrono::Utc>>,
    last_error: Option<&crate::terminal::board_poller::BoardFetchError>,
) {
    let now = chrono::Utc::now();
    let freshness = match last_updated {
        Some(ts) => crate::terminal::board_poller::format_last_updated(ts, now),
        None => "loading...".to_string(),
    };
    let footer = match last_error {
        Some(err) => format!("⚠ {} — {}", err.user_message(), freshness),
        None => format!("⏱ {freshness}"),
    };
    let color = if last_error.is_some() {
        DRACULA_ORANGE
    } else {
        DRACULA_COMMENT
    };
    lines.push(Line::from(Span::styled(footer, Style::default().fg(color))));
}
