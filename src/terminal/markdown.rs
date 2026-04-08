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

const CHUNK_SIZE_LIMIT: usize = 4000;

pub(super) fn render_md_message<'a>(
    text: &'a str,
    prefix_spans: Vec<Span<'a>>,
    indent: Span<'a>,
    line_limit: Option<usize>,
) -> Vec<Line<'a>> {
    let chunks = chunk_at_paragraphs(text, CHUNK_SIZE_LIMIT);
    let mut all_lines = Vec::new();

    for chunk in &chunks {
        let options = tui_markdown::Options::new(DraculaStyleSheet);
        let md_text = tui_markdown::from_str_with_options(chunk, &options);
        for mut md_line in md_text.lines {
            let mut spans = Vec::new();
            if all_lines.is_empty() {
                spans.extend(prefix_spans.iter().cloned());
            } else {
                spans.push(indent.clone());
            }
            let line_style = md_line.style;
            for span in &mut md_line.spans {
                span.style = line_style.patch(span.style);
                if span.style.fg == Some(Color::LightBlue) {
                    span.style.fg = Some(DRACULA_CYAN);
                } else if span.style.fg == Some(Color::Blue) {
                    span.style.fg = Some(DRACULA_PURPLE);
                }
            }
            strip_heading_hashes(&mut md_line.spans);
            spans.extend(md_line.spans);
            all_lines.push(Line::from(spans));

            if let Some(limit) = line_limit
                && all_lines.len() >= limit
            {
                return all_lines;
            }
        }
    }
    all_lines
}

fn chunk_at_paragraphs(text: &str, max_size: usize) -> Vec<&str> {
    if text.len() <= max_size {
        return vec![text];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        if start + max_size >= text.len() {
            chunks.push(&text[start..]);
            break;
        }
        // Find the last paragraph break (double newline) within the limit
        let window = &text[start..start + max_size];
        let break_at = window.rfind("\n\n").map(|i| i + 2).unwrap_or(max_size);
        chunks.push(&text[start..start + break_at]);
        start += break_at;
    }
    chunks
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

            if available > 0 {
                // Prefer breaking at a word boundary (last space before the limit)
                let chunk = &remaining[..byte_pos];
                let break_at = chunk
                    .rfind(' ')
                    .map(|i| i + 1) // include the space in the current line
                    .unwrap_or(byte_pos); // no space = fall back to char break
                let (chunk, rest) = remaining.split_at(break_at);
                let trimmed = chunk.trim_end();
                if !trimmed.is_empty() {
                    current_spans.push(Span::styled(trimmed.to_string(), style));
                }
                remaining = rest.trim_start();
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

    // Broadcasts belong in broadcast view, not channel views
    let Some(recipients) = &msg.to else {
        return false;
    };

    let is_involved = |name: &str| sender_short == name || recipients.iter().any(|r| r == name);

    // The owner must be involved AND every view participant must be involved
    let view_participants: Vec<&str> = view.split('+').collect();
    is_involved(owner) && view_participants.iter().all(|p| is_involved(p))
}
