#![allow(clippy::unwrap_used, clippy::expect_used)]

// Unit tests for the `tokens` module — #179 dimension 3.
//
// **Scope**: these are UNIT TESTS on the `TokenRow` struct and the
// `append_token_row` / `read_token_rows` pure file-IO functions.
// They call the functions directly and assert on return values.
// They do NOT exercise the full adapter → log → `/tokens` view path
// — that's covered end-to-end by a future story60-style harness
// test once the TUI view lands.
//
// Naming follows `feedback_test_name_matches_content.md`: these tests
// describe what the functions return, not what a system does.

use super::tokens::*;
use tempfile::tempdir;

fn sample_row() -> TokenRow {
    TokenRow {
        ts: "2026-04-11T22:30:00+00:00".to_string(),
        role: "dev".to_string(),
        harness: "claude-code".to_string(),
        model: "claude-opus-4-6".to_string(),
        input_tokens: 120,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 8200,
        output_tokens: 450,
        tool_call_count: 3,
        turn_duration_ms: Some(8200),
        transcript_line: Some(16047),
        session_id: Some("abc123".to_string()),
    }
}

#[test]
fn append_token_row_writes_jsonl_line() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("log.jsonl");
    append_token_row(&path, &sample_row()).unwrap();
    let contents = std::fs::read_to_string(&path).unwrap();
    // One line, trailing newline.
    assert_eq!(contents.lines().count(), 1);
    assert!(contents.ends_with('\n'));
    // Line is valid JSON and round-trips.
    let line = contents.lines().next().unwrap();
    let parsed: TokenRow = serde_json::from_str(line).unwrap();
    assert_eq!(parsed.role, "dev");
    assert_eq!(parsed.input_tokens, 120);
    assert_eq!(parsed.output_tokens, 450);
}

#[test]
fn read_token_rows_returns_appended_rows_in_order() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("log.jsonl");
    let mut row_a = sample_row();
    row_a.role = "dev".to_string();
    row_a.output_tokens = 100;
    let mut row_b = sample_row();
    row_b.role = "pm".to_string();
    row_b.output_tokens = 200;
    append_token_row(&path, &row_a).unwrap();
    append_token_row(&path, &row_b).unwrap();
    let rows = read_token_rows(&path).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].role, "dev");
    assert_eq!(rows[0].output_tokens, 100);
    assert_eq!(rows[1].role, "pm");
    assert_eq!(rows[1].output_tokens, 200);
}

#[test]
fn read_token_rows_returns_empty_for_missing_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent.jsonl");
    let rows = read_token_rows(&path).unwrap();
    assert!(rows.is_empty());
}

#[test]
fn read_token_rows_skips_malformed_lines_but_keeps_valid_ones() {
    // If an adapter writes a corrupted line, it shouldn't poison the
    // rest of the log. Malformed lines are skipped silently (the
    // diagnostic is just that the row count is lower than expected —
    // a future diag hook could log this, but for now we favor
    // availability of valid data over visibility of the corruption).
    let dir = tempdir().unwrap();
    let path = dir.path().join("log.jsonl");
    append_token_row(&path, &sample_row()).unwrap();
    // Inject a malformed line directly into the file.
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    f.write_all(b"this is not json\n").unwrap();
    append_token_row(&path, &sample_row()).unwrap();
    let rows = read_token_rows(&path).unwrap();
    assert_eq!(
        rows.len(),
        2,
        "malformed line should be skipped, valid rows preserved"
    );
}

#[test]
fn resolve_tokens_log_path_uses_home_dir_when_no_env_override() {
    // Guards against a rename to e.g. `~/.claude/...` that would
    // silently move the log. Pure function, no env coupling.
    let home = std::path::PathBuf::from("/fake/home");
    let path = resolve_tokens_log_path(&home, None);
    let s = path.to_string_lossy();
    assert!(s.ends_with(".bpmux/tokens/log.jsonl"), "got {s}");
}

#[test]
fn resolve_tokens_log_path_honors_env_override_when_set() {
    // Guards the env var path — tests and adapters use it to redirect
    // writes to a tempdir.
    let home = std::path::PathBuf::from("/fake/home");
    let path = resolve_tokens_log_path(&home, Some("/tmp/custom-tokens"));
    assert_eq!(
        path,
        std::path::PathBuf::from("/tmp/custom-tokens/log.jsonl")
    );
}

// ─────────────────────────────────────────────────────────────────────
// Aggregation helper tests — pure functions over `&[TokenRow]`. These
// power the `/tokens` view's rollups. Unit-named per the test-name
// discipline: they describe what the function returns, not a system
// behavior.
// ─────────────────────────────────────────────────────────────────────

fn row_for(role: &str, total_in: u64, out: u64) -> TokenRow {
    TokenRow {
        ts: "2026-04-11T22:30:00+00:00".to_string(),
        role: role.to_string(),
        harness: "claude-code".to_string(),
        model: "claude-opus-4-6".to_string(),
        input_tokens: total_in,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        output_tokens: out,
        tool_call_count: 1,
        turn_duration_ms: None,
        transcript_line: None,
        session_id: None,
    }
}

#[test]
fn totals_by_role_returns_sum_of_total_tokens_per_role() {
    let rows = vec![
        row_for("dev", 100, 50),
        row_for("pm", 200, 75),
        row_for("dev", 300, 100),
        row_for("designer", 150, 25),
    ];
    let totals = totals_by_role(&rows);
    // dev: (100+50) + (300+100) = 550
    // pm: 200+75 = 275
    // designer: 150+25 = 175
    assert_eq!(totals.get("dev").copied().unwrap_or(0), 550);
    assert_eq!(totals.get("pm").copied().unwrap_or(0), 275);
    assert_eq!(totals.get("designer").copied().unwrap_or(0), 175);
}

#[test]
fn totals_by_role_returns_empty_for_no_rows() {
    let rows: Vec<TokenRow> = vec![];
    let totals = totals_by_role(&rows);
    assert!(totals.is_empty());
}

#[test]
fn build_token_report_summarizes_rows_into_grand_total_and_role_breakdown() {
    let rows = vec![
        row_for("dev", 100, 50),
        row_for("pm", 200, 75),
        row_for("dev", 300, 100),
    ];
    let report = build_token_report(&rows);
    assert_eq!(report.total_tokens, 825); // 150 + 275 + 400
    assert_eq!(report.row_count, 3);
    assert_eq!(report.by_role.get("dev").copied(), Some(550));
    assert_eq!(report.by_role.get("pm").copied(), Some(275));
    // Recent: newest-first order.
    assert_eq!(report.recent.len(), 3);
    assert_eq!(report.recent[0].role, "dev"); // last appended
    assert_eq!(report.recent[0].total_tokens, 400);
}

#[test]
fn build_token_report_caps_recent_at_ten_rows() {
    // 15 rows — recent should contain 10 newest.
    let rows: Vec<TokenRow> = (0..15)
        .map(|i| row_for("dev", (i + 1) as u64 * 10, 0))
        .collect();
    let report = build_token_report(&rows);
    assert_eq!(report.row_count, 15);
    assert_eq!(report.recent.len(), 10);
    // Newest first — recent[0] should correspond to i=14 (total = 150).
    assert_eq!(report.recent[0].total_tokens, 150);
    // Oldest kept — recent[9] should correspond to i=5 (total = 60).
    assert_eq!(report.recent[9].total_tokens, 60);
}

#[test]
fn total_tokens_sums_all_four_usage_fields() {
    let row = TokenRow {
        ts: "2026-04-11T22:30:00+00:00".to_string(),
        role: "dev".to_string(),
        harness: "claude-code".to_string(),
        model: "claude-opus-4-6".to_string(),
        input_tokens: 10,
        cache_creation_input_tokens: 20,
        cache_read_input_tokens: 30,
        output_tokens: 40,
        tool_call_count: 0,
        turn_duration_ms: None,
        transcript_line: None,
        session_id: None,
    };
    assert_eq!(row.total_tokens(), 100);
}
