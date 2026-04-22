//! Table rendering — grid and card layouts for markdown tables.

use super::markdown::strip_link_urls;

pub(super) fn is_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|')
        && trimmed.ends_with('|')
        && trimmed
            .chars()
            .all(|c| c == '|' || c == '-' || c == ' ' || c == ':')
}

fn clean_cell(text: &str) -> String {
    let stripped = strip_link_urls(text);
    stripped.replace("**", "")
}

pub(super) fn parse_table_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| clean_cell(c.trim()))
        .collect()
}

const MIN_COLUMN_WIDTH: usize = 20;

fn render_table_as_cards(rows: &[Vec<String>]) -> String {
    if rows.len() < 2 {
        return String::new();
    }
    let headers = &rows[0];
    let mut out = String::new();
    for row in &rows[1..] {
        for (i, cell) in row.iter().enumerate() {
            if cell.is_empty() {
                continue;
            }
            let label = headers.get(i).map(|s| s.as_str()).unwrap_or("?");
            if i == 0 {
                out.push_str(cell);
                out.push('\n');
            } else {
                out.push_str(&format!("  {label}: {cell}\n"));
            }
        }
        out.push('\n');
    }
    out
}

fn render_table_as_grid(rows: &[Vec<String>], col_widths: &[usize]) -> String {
    let col_count = col_widths.len();

    let separator = |left: char, mid: char, right: char| {
        let mut s = String::new();
        s.push(left);
        for (i, &w) in col_widths.iter().enumerate() {
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
        let wrapped: Vec<Vec<String>> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let w = col_widths.get(i).copied().unwrap_or(10);
                wrap_text(cell, w)
            })
            .collect();
        let max_lines = wrapped.iter().map(|w| w.len()).max().unwrap_or(1);
        for line_idx in 0..max_lines {
            out.push('│');
            for (i, w) in col_widths.iter().enumerate() {
                let text = wrapped
                    .get(i)
                    .and_then(|lines| lines.get(line_idx))
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let pad = w.saturating_sub(text.chars().count());
                out.push(' ');
                out.push_str(text);
                for _ in 0..pad {
                    out.push(' ');
                }
                out.push(' ');
                out.push('│');
            }
            out.push('\n');
        }
        if ri == 0 {
            out.push_str(&separator('├', '┼', '┤'));
        }
    }
    out.push_str(&separator('└', '┴', '┘'));
    out
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 || text.chars().count() <= width {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut remaining = text;
    while remaining.chars().count() > width {
        let byte_at = remaining
            .char_indices()
            .nth(width)
            .map_or(remaining.len(), |(i, _)| i);
        let chunk = &remaining[..byte_at];
        let break_at = chunk.rfind(' ').map(|i| i + 1).unwrap_or(byte_at);
        lines.push(remaining[..break_at].trim_end().to_string());
        remaining = remaining[break_at..].trim_start();
    }
    if !remaining.is_empty() {
        lines.push(remaining.to_string());
    }
    lines
}

pub(super) fn render_table(rows: &[Vec<String>], terminal_width: usize) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if col_count == 0 {
        return String::new();
    }
    let border_overhead = col_count + 1 + col_count * 2;
    let available = terminal_width.saturating_sub(border_overhead);

    if available / col_count < MIN_COLUMN_WIDTH {
        return render_table_as_cards(rows);
    }

    let mut max_content: Vec<usize> = vec![0; col_count];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                max_content[i] = max_content[i].max(cell.chars().count());
            }
        }
    }
    let total_content: usize = max_content.iter().sum::<usize>().max(1);
    let mut widths: Vec<usize> = max_content
        .iter()
        .map(|&c| ((c * available) / total_content).max(MIN_COLUMN_WIDTH))
        .collect();
    // Clamp total to available width
    let total: usize = widths.iter().sum();
    if total > available {
        let excess = total - available;
        if let Some(largest) = widths.iter_mut().max() {
            *largest = largest.saturating_sub(excess);
        }
    }
    render_table_as_grid(rows, &widths)
}
