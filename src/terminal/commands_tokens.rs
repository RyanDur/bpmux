//! `/tokens` slash command — opens the token-cost overlay for #179.
//!
//! Reads `~/.bpmux/tokens/log.jsonl` (or the env-overridden path) via
//! the pure `session::tokens` reader, snapshots the rows onto
//! `App::tokens_view_rows`, and flips `show_tokens_view = true` so
//! `ui::render_tokens_overlay` draws them.
//!
//! Extracted into its own module to keep `commands.rs` under its 400-
//! line soft limit (the `commands_help.rs` extraction followed the
//! same pattern; see #164 rework for the precedent).

use super::app::App;
use crate::session::Message;
use crate::session::tokens::{default_tokens_log_path, read_token_rows};

impl App {
    /// `/tokens` — show the token-cost overlay. Snapshots the log
    /// rows onto `App::tokens_view_rows` so the overlay stays stable
    /// across frames until dismissed.
    pub(super) fn handle_tokens(&mut self) -> Option<Message> {
        let rows = match default_tokens_log_path().and_then(|p| read_token_rows(&p)) {
            Ok(rows) => rows,
            Err(e) => {
                self.system_message(format!("tokens: unable to read log: {e}"));
                self.clear_input();
                return None;
            }
        };
        self.tokens.rows = rows;
        self.tokens.show = true;
        self.input_mode = super::app::InputMode::Tokens;
        self.clear_input();
        None
    }
}

/// Build the text block the `/tokens` overlay renders. Pure function
/// over the snapshot so it can be unit-tested without a frame. The
/// overlay renderer (`ui::render_tokens_overlay`) wraps the returned
/// string in a bordered box.
///
/// Layout (text-only v1):
/// ```text
/// Total: 12.3K tokens across 4 rows
///
/// By role:
///   dev       8.1K
///   pm        2.4K
///   designer  1.8K
///
/// Recent:
///   22:30 dev       350
///   22:28 pm        210
///   ...
///
/// Press any key to close
/// ```
///
/// A future commit will upgrade this to a ratatui `Chart` widget for
/// the line-graph-over-time view Ryan asked for; the text form is the
/// first working slice so the feature loop closes today.
pub(super) fn format_tokens_overlay(rows: &[crate::session::tokens::TokenRow]) -> String {
    use crate::session::tokens::totals_by_role;
    let mut out = String::new();
    let row_count = rows.len();
    let grand_total: u64 = rows.iter().map(|r| r.total_tokens()).sum();
    out.push_str(&format!(
        "Total: {} tokens across {row_count} row{}\n\n",
        human(grand_total),
        if row_count == 1 { "" } else { "s" }
    ));

    let totals = totals_by_role(rows);
    if totals.is_empty() {
        out.push_str("No token rows yet.\n");
        out.push_str("Once an agent completes a turn with the adapter\n");
        out.push_str("hook registered, rows will appear here.\n\n");
    } else {
        out.push_str("By role:\n");
        let widest_name = totals.keys().map(|k| k.len()).max().unwrap_or(0);
        for (role, total) in &totals {
            out.push_str(&format!("  {role:<widest_name$}  {}\n", human(*total)));
        }
        out.push('\n');
    }

    // Show the last 5 rows as a recent-activity list. Reverse-chron
    // order (most recent on top) to match chat pane conventions.
    const RECENT_LIMIT: usize = 5;
    if !rows.is_empty() {
        out.push_str("Recent:\n");
        for row in rows.iter().rev().take(RECENT_LIMIT) {
            let ts_short = row.ts.split('T').nth(1).unwrap_or(&row.ts);
            let ts_hm: String = ts_short.chars().take(5).collect();
            out.push_str(&format!(
                "  {ts_hm} {role:<8} {total}\n",
                role = row.role,
                total = human(row.total_tokens())
            ));
        }
        out.push('\n');
    }

    out
}

/// #195: format the feature usage table appended to the tokens overlay.
pub(super) fn format_usage_table(
    usage: &std::collections::HashMap<String, (usize, chrono::DateTime<chrono::Utc>)>,
) -> String {
    let mut out = String::new();
    if usage.is_empty() {
        return out;
    }
    out.push_str("Feature usage:\n");

    let mut entries: Vec<(&str, usize, &chrono::DateTime<chrono::Utc>)> = usage
        .iter()
        .map(|(k, (c, t))| (k.as_str(), *c, t))
        .collect();
    // Sort by count descending
    entries.sort_by(|a, b| b.1.cmp(&a.1));

    let now = chrono::Utc::now();
    let widest_name = entries.iter().map(|(n, _, _)| n.len()).max().unwrap_or(0);
    for (name, count, last) in &entries {
        let age = now.signed_duration_since(**last);
        let ago = if age.num_seconds() < 60 {
            "now".to_string()
        } else if age.num_minutes() < 60 {
            format!("{} min ago", age.num_minutes())
        } else {
            format!("{}h ago", age.num_hours())
        };
        let count_style = if *count > 0 {
            format!("{count:>4}")
        } else {
            "   0".to_string()
        };
        out.push_str(&format!("  {name:<widest_name$}  {count_style}   {ago}\n"));
    }
    out.push('\n');
    out
}

/// Format a token count as a short human string: `1234` → `1.2K`,
/// `2500000` → `2.5M`. Used by the overlay to keep rows narrow in the
/// 52-char overlay width budget. Single-decimal-place to match
/// ratatui Chart axis conventions.
///
/// Exposed to `super` because `ui::render_tokens_overlay` also uses
/// it for axis labels and the breakdown table in the chart layout.
pub(super) fn human(n: u64) -> String {
    if n < 1_000 {
        n.to_string()
    } else if n < 1_000_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::tokens::TokenRow;

    fn row(role: &str, ts_hm: &str, total_out: u64) -> TokenRow {
        TokenRow {
            ts: format!("2026-04-11T{ts_hm}:00+00:00"),
            role: role.to_string(),
            harness: "claude-code".to_string(),
            model: "claude-opus-4-6".to_string(),
            input_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            output_tokens: total_out,
            tool_call_count: 0,
            turn_duration_ms: None,
            transcript_line: None,
            session_id: None,
        }
    }

    #[test]
    fn format_tokens_overlay_shows_role_totals_and_grand_total() {
        let rows = vec![
            row("dev", "22:30:00", 100),
            row("pm", "22:31:00", 200),
            row("dev", "22:32:00", 300),
        ];
        let text = format_tokens_overlay(&rows);
        // Grand total line
        assert!(text.contains("Total: 600 tokens"), "got: {text}");
        // Role breakdown
        assert!(text.contains("By role:"));
        assert!(text.contains("dev"));
        assert!(text.contains("pm"));
        assert!(text.contains("400"), "dev should sum to 400, got: {text}");
        assert!(text.contains("200"), "pm should total 200, got: {text}");
        // Footer moved to renderer (#195)
    }

    #[test]
    fn format_tokens_overlay_shows_placeholder_when_no_rows() {
        let text = format_tokens_overlay(&[]);
        assert!(text.contains("No token rows yet"));
        assert!(text.contains("Total: 0 tokens"));
    }

    #[test]
    fn format_tokens_overlay_recent_section_lists_last_five_most_recent_first() {
        let rows: Vec<TokenRow> = (0..8)
            .map(|i| row("dev", &format!("22:{:02}:00", 20 + i), (i + 1) as u64 * 10))
            .collect();
        let text = format_tokens_overlay(&rows);
        assert!(text.contains("Recent:"));
        // Most recent row (output=80) should appear; earliest (output=10) should be cut off.
        assert!(text.contains("80"), "most recent row missing: {text}");
        // Count "Recent:" lines — should be 5 indented entries.
        let recent_section = text.split("Recent:\n").nth(1).unwrap();
        let recent_lines: Vec<&str> = recent_section
            .lines()
            .take_while(|l| l.starts_with("  "))
            .collect();
        assert_eq!(
            recent_lines.len(),
            5,
            "expected 5 recent rows, got {}",
            recent_lines.len()
        );
    }

    #[test]
    fn human_formats_small_medium_and_large_counts() {
        assert_eq!(human(0), "0");
        assert_eq!(human(999), "999");
        assert_eq!(human(1_234), "1.2K");
        assert_eq!(human(12_345), "12.3K");
        assert_eq!(human(1_500_000), "1.5M");
    }
}
