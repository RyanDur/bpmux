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

fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|')
        && trimmed.ends_with('|')
        && trimmed
            .chars()
            .all(|c| c == '|' || c == '-' || c == ' ' || c == ':')
}

fn parse_table_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

fn render_table(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; col_count];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                widths[i] = widths[i].max(cell.chars().count());
            }
        }
    }

    let separator = |left: char, mid: char, right: char| {
        let mut s = String::new();
        s.push(left);
        for (i, &w) in widths.iter().enumerate() {
            for _ in 0..w + 2 {
                s.push('─');
            }
            s.push(if i + 1 < col_count { mid } else { right });
        }
        s.push('\n');
        s
    };

    let mut out = separator('┌', '┬', '┐');
    for (ri, row) in rows.iter().enumerate() {
        out.push('│');
        for (i, w) in widths.iter().enumerate() {
            let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
            let pad = w.saturating_sub(cell.chars().count());
            out.push(' ');
            out.push_str(cell);
            for _ in 0..pad {
                out.push(' ');
            }
            out.push(' ');
            out.push('│');
        }
        out.push('\n');
        if ri == 0 {
            out.push_str(&separator('├', '┼', '┤'));
        }
    }
    out.push_str(&separator('└', '┴', '┘'));
    out
}

fn simplify_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with('|') && trimmed.ends_with('|') && !is_table_separator(trimmed) {
            let mut table_rows = Vec::new();
            while i < lines.len() {
                let t = lines[i].trim();
                if t.starts_with('|') && t.ends_with('|') {
                    if !is_table_separator(t) {
                        table_rows.push(parse_table_row(t));
                    }
                    i += 1;
                } else {
                    break;
                }
            }
            result.push_str(&render_table(&table_rows));
        } else {
            result.push_str(lines[i]);
            result.push('\n');
            i += 1;
        }
    }
    // Strip link URLs: [text](url) → text
    let mut output = String::with_capacity(result.len());
    let mut chars = result.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' {
            let mut link_text = String::new();
            let mut found_close = false;
            for inner in chars.by_ref() {
                if inner == ']' {
                    found_close = true;
                    break;
                }
                link_text.push(inner);
            }
            if found_close && chars.peek() == Some(&'(') {
                chars.next();
                let mut depth = 1;
                for inner in chars.by_ref() {
                    if inner == '(' {
                        depth += 1;
                    } else if inner == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                }
                output.push_str(&link_text);
            } else {
                output.push('[');
                output.push_str(&link_text);
                if found_close {
                    output.push(']');
                }
            }
        } else {
            output.push(c);
        }
    }
    output
}

pub(super) fn render_md_message<'a>(
    text: &'a str,
    prefix_spans: Vec<Span<'a>>,
    indent: Span<'a>,
    line_limit: Option<usize>,
) -> Vec<Line<'a>> {
    let simplified = simplify_markdown(text);
    let chunks = chunk_at_paragraphs(&simplified, CHUNK_SIZE_LIMIT);
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
            for span in md_line.spans {
                spans.push(Span::styled(span.content.to_string(), span.style));
            }
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

fn chunk_at_paragraphs(text: &str, max_size: usize) -> Vec<String> {
    if text.len() <= max_size {
        return vec![text.to_string()];
    }
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < text.len() {
        if start + max_size >= text.len() {
            chunks.push(text[start..].to_string());
            break;
        }
        let mut end = start + max_size;
        while !text.is_char_boundary(end) && end > start {
            end -= 1;
        }
        let window = &text[start..end];
        let break_at = window.rfind("\n\n").map(|i| i + 2).unwrap_or(end - start);
        chunks.push(text[start..start + break_at].to_string());
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

pub(crate) fn message_matches_channel(msg: &session::Message, view: &str, owner: &str) -> bool {
    if msg.to.is_none() {
        return false;
    }
    crate::terminal::app::channel_name(msg, owner) == view
}
