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

pub(super) fn strip_link_urls(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
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

enum Segment {
    Text(String),
    Table(Vec<Vec<String>>),
}

fn split_into_segments(text: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut text_buf = String::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with('|')
            && trimmed.ends_with('|')
            && !super::table::is_table_separator(trimmed)
        {
            if !text_buf.is_empty() {
                segments.push(Segment::Text(std::mem::take(&mut text_buf)));
            }
            let mut table_rows = Vec::new();
            while i < lines.len() {
                let t = lines[i].trim();
                if t.starts_with('|') && t.ends_with('|') {
                    if !super::table::is_table_separator(t) {
                        table_rows.push(super::table::parse_table_row(t));
                    }
                    i += 1;
                } else {
                    break;
                }
            }
            segments.push(Segment::Table(table_rows));
        } else {
            text_buf.push_str(lines[i]);
            text_buf.push('\n');
            i += 1;
        }
    }
    if !text_buf.is_empty() {
        segments.push(Segment::Text(text_buf));
    }
    segments
}

pub(super) fn render_md_message<'a>(
    text: &'a str,
    prefix_spans: Vec<Span<'a>>,
    indent: Span<'a>,
    line_limit: Option<usize>,
    terminal_width: usize,
) -> Vec<Line<'a>> {
    let indent_width = indent.content.chars().count();
    let table_width = terminal_width.saturating_sub(indent_width);
    let segments = split_into_segments(text);
    let mut all_lines: Vec<Line<'a>> = Vec::new();

    for segment in segments {
        match segment {
            Segment::Table(rows) => {
                let table_text = crate::terminal::table::render_table(&rows, table_width);
                for table_line in table_text.lines() {
                    let mut spans = Vec::new();
                    if all_lines.is_empty() {
                        spans.extend(prefix_spans.iter().cloned());
                    } else {
                        spans.push(indent.clone());
                    }
                    spans.push(Span::raw(table_line.to_string()));
                    all_lines.push(Line::from(spans));
                }
            }
            Segment::Text(md_text) => {
                let simplified = strip_link_urls(&md_text);
                let chunks = chunk_at_paragraphs(&simplified, CHUNK_SIZE_LIMIT);
                for chunk in &chunks {
                    let options = tui_markdown::Options::new(DraculaStyleSheet);
                    let md = tui_markdown::from_str_with_options(chunk, &options);
                    let md_lines = merge_orphan_list_numbers(md.lines);
                    for mut md_line in md_lines {
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
            }
        }
        if let Some(limit) = line_limit
            && all_lines.len() >= limit
        {
            return all_lines;
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

/// tui-markdown sometimes puts list numbers ("1. ", "2. ") on their own line,
/// separate from the item content. This merges orphan number lines with the
/// following content line.
fn merge_orphan_list_numbers(lines: Vec<Line<'_>>) -> Vec<Line<'_>> {
    fn is_orphan_number(line: &Line<'_>) -> bool {
        if line.spans.len() != 1 {
            return false;
        }
        let text = line.spans[0].content.trim();
        text.ends_with('.')
            && text.len() <= 4
            && text[..text.len() - 1].chars().all(|c| c.is_ascii_digit())
    }

    let mut result = Vec::with_capacity(lines.len());
    let mut iter = lines.into_iter().peekable();

    while let Some(line) = iter.next() {
        if is_orphan_number(&line) && iter.peek().is_some() {
            let next = iter.next();
            if let Some(next_line) = next {
                let mut merged_spans = line.spans;
                merged_spans.extend(next_line.spans);
                result.push(Line::from(merged_spans));
            }
        } else {
            result.push(line);
        }
    }
    result
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
                let chunk = &remaining[..byte_pos];
                let break_at = chunk.rfind(' ').map(|i| i + 1).unwrap_or(byte_pos);
                let (chunk, rest) = remaining.split_at(break_at);
                let trimmed = chunk.trim_end();
                if !trimmed.is_empty() {
                    current_spans.push(Span::styled(trimmed.to_string(), style));
                    current_width += trimmed.chars().count();
                }
                remaining = rest.trim_start();
            }
            let content_width = current_width.saturating_sub(indent_width);
            if content_width < 10 && !remaining.is_empty() && available > 0 {
                continue;
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

#[cfg(test)]
mod table_tests {
    use super::*;
    #[test]
    fn table_renders_with_borders() {
        let rows = vec![
            vec!["Anti-feature".to_string(), "Reason".to_string()],
            vec![
                "Threading".to_string(),
                "Flat chat preserves shared awareness for small teams. Threading fragments it."
                    .to_string(),
            ],
            vec![
                "Real-time editing".to_string(),
                "bpmux is a communication channel, not a shared editor.".to_string(),
            ],
        ];
        let output = crate::terminal::table::render_table(&rows, 78);
        println!("\n{output}");
        assert!(output.contains("┌"));
        assert!(output.contains("┤"));
        assert!(output.contains("└"));
    }
}

#[cfg(test)]
mod table_width_tests {
    use super::*;
    #[test]
    fn table_fits_within_width_minus_indent() {
        let rows = vec![
            vec!["A".to_string(), "B".to_string()],
            vec![
                "short".to_string(),
                "Longer text that should wrap within the cell boundaries properly".to_string(),
            ],
        ];
        let table_width = 63; // 78 - 15 indent
        let output = crate::terminal::table::render_table(&rows, table_width);
        for line in output.lines() {
            let line_chars = line.chars().count();
            assert!(
                line_chars <= table_width,
                "Table line exceeds width {table_width}: {line_chars} chars: {line}"
            );
        }
    }
}
