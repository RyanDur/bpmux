use ratatui::{
    style::Color,
    text::{Line, Span},
};

use crate::session;
use crate::terminal::theme::{DRACULA_CYAN, DRACULA_PURPLE, DraculaStyleSheet};

/// Strip heading hash prefixes (e.g. "## ", "### ") from the first span in a line.
/// tui-markdown keeps hashes in the output; we remove them for GitHub-style rendering.
pub(super) fn strip_heading_hashes(spans: &mut Vec<Span<'_>>) {
    if let Some(first) = spans.first_mut() {
        let trimmed = first.content.trim_start_matches('#');
        if trimmed.len() < first.content.len() {
            // Had leading hashes — strip them plus the trailing space
            let stripped = trimmed.trim_start();
            if stripped.is_empty() {
                spans.remove(0);
            } else {
                *first = Span::styled(stripped.to_string(), first.style);
            }
        }
    }
}

/// Render markdown message text into `Line`s, prepending prefix spans on the first
/// line and an indent on subsequent lines.  When `line_limit` is `Some(n)`, only
/// the first `n` markdown lines are emitted.
pub(super) fn render_md_message<'a>(
    text: &'a str,
    prefix_spans: Vec<Span<'a>>,
    indent: Span<'a>,
    line_limit: Option<usize>,
) -> Vec<Line<'a>> {
    let options = tui_markdown::Options::new(DraculaStyleSheet);
    let md_text = tui_markdown::from_str_with_options(text, &options);
    let iter: Box<dyn Iterator<Item = (usize, Line<'_>)>> = match line_limit {
        Some(n) => Box::new(md_text.lines.into_iter().take(n).enumerate()),
        None => Box::new(md_text.lines.into_iter().enumerate()),
    };
    let mut lines = Vec::new();
    for (li, mut md_line) in iter {
        let mut spans = Vec::new();
        if li == 0 {
            spans.extend(prefix_spans.iter().cloned());
        } else {
            spans.push(indent.clone());
        }
        // Patch line-level style into each span so it survives Line reconstruction
        let line_style = md_line.style;
        for span in &mut md_line.spans {
            span.style = line_style.patch(span.style);
            // Replace hardcoded ANSI blues from tui-markdown internals
            if span.style.fg == Some(Color::LightBlue) {
                span.style.fg = Some(DRACULA_CYAN);
            } else if span.style.fg == Some(Color::Blue) {
                span.style.fg = Some(DRACULA_PURPLE);
            }
        }
        // Strip heading hash prefixes (e.g. "## ") — render as styled text only
        strip_heading_hashes(&mut md_line.spans);
        spans.extend(md_line.spans);
        lines.push(Line::from(spans));
    }
    lines
}

/// Wrap a Line into multiple lines if it exceeds `max_width`, adding `indent`
/// as a prefix on continuation lines (hanging indent).
pub(crate) fn wrap_line_with_indent<'a>(
    line: Line<'a>,
    indent: &Span<'a>,
    max_width: usize,
) -> Vec<Line<'a>> {
    if max_width == 0 {
        return vec![line];
    }
    let total_width: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    if total_width <= max_width {
        return vec![line];
    }

    let indent_width = indent.content.chars().count();
    let mut result = Vec::new();
    let mut current_spans: Vec<Span<'a>> = Vec::new();
    let mut current_width = 0usize;

    for span in line.spans {
        let style = span.style;
        let mut remaining: &str = &span.content;

        while !remaining.is_empty() {
            let available = max_width.saturating_sub(current_width);

            // Single pass: walk char_indices to find both the split point
            // and whether we need to split at all.
            let mut split_byte = None;
            let mut char_count = 0;
            for (byte_pos, _) in remaining.char_indices() {
                if char_count == available {
                    split_byte = Some(byte_pos);
                    break;
                }
                char_count += 1;
            }

            let Some(byte_pos) = split_byte else {
                // Entire remaining fits in available width
                current_spans.push(Span::styled(remaining.to_string(), style));
                current_width += char_count;
                break;
            };

            // Take what fits, emit line, continue with indent
            if available > 0 {
                let (chunk, rest) = remaining.split_at(byte_pos);
                current_spans.push(Span::styled(chunk.to_string(), style));
                remaining = rest;
            }
            result.push(Line::from(current_spans));
            current_spans = vec![indent.clone()];
            current_width = indent_width;
        }
    }
    if !current_spans.is_empty() {
        result.push(Line::from(current_spans));
    }
    result
}

/// Check if a message involves all participants named in the view.
/// View "dev" matches any message where dev is sender or recipient.
/// View "dev+pm" matches messages where both dev and pm are involved.
pub(crate) fn message_involves_participants(
    msg: &session::Message,
    view: &str,
    owner: &str,
) -> bool {
    let sender_short = msg.from.split(':').next().unwrap_or(&msg.from);
    // Owner's own messages always appear in whatever view they're in
    if sender_short == owner {
        return true;
    }
    let view_participants: Vec<&str> = view.split('+').collect();

    view_participants.iter().all(|participant| {
        if sender_short == *participant {
            return true;
        }
        if *participant == owner {
            return true;
        }
        if let Some(recipients) = &msg.to {
            return recipients.iter().any(|r| r == participant);
        }
        false
    })
}
