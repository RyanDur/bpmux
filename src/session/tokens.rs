//! Token-cost observability log — #179 dimension 3 of #175.
//!
//! Stores per-turn LLM-call token counts in an append-only JSONL log at
//! `~/.bpmux/tokens/log.jsonl`. Rows are written by **per-harness
//! adapters** (Claude Code, Cursor, etc.) that read their native
//! recording of their own LLM calls and emit a row in this common
//! schema. bpmux itself never observes LLM calls — that would violate
//! the "not subjective to you" governance property. Data is objective
//! because it comes from the harness's own ground truth, not from the
//! agent reporting on itself.
//!
//! See `docs/token-log-schema.md` for the adapter contract.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::session::LogError;

/// One row in the token log. Shape is the common schema all adapters
/// target. Adapters may enrich `metadata`-shaped fields by putting
/// extra data in `harness`-specific keys if we add a map field later,
/// but the first pass keeps the schema flat for simplicity and
/// aggregation-friendliness.
///
/// Field rationale:
/// - `ts`: RFC3339 string (not `DateTime<Utc>`) so adapters written in
///   any language can emit compatible rows by format convention alone.
/// - `role` / `harness`: the two dimensions `/tokens` groups by.
/// - `input_tokens` + the two cache_* fields + `output_tokens`: the
///   standard Anthropic usage shape. Other providers will have their
///   own shapes; adapters normalize on write.
/// - `tool_call_count`: a useful cost signal — a turn with 10 tool
///   calls is expensive in latency even if token counts look modest.
/// - `turn_duration_ms`: optional so adapters that can't derive it
///   (from transcript timestamps, for example) can still write rows.
/// - `transcript_line` / `session_id`: optional adapter-side debugging
///   aids, not required for aggregation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenRow {
    pub ts: String,
    pub role: String,
    pub harness: String,
    pub model: String,
    pub input_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
    pub output_tokens: u64,
    pub tool_call_count: u32,
    #[serde(default)]
    pub turn_duration_ms: Option<u64>,
    #[serde(default)]
    pub transcript_line: Option<u64>,
    #[serde(default)]
    pub session_id: Option<String>,
}

impl TokenRow {
    /// Total tokens counted against the prompt budget for this turn —
    /// input + cache creation + cache read + output. Useful as a
    /// single aggregate cost metric.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens
            .saturating_add(self.cache_creation_input_tokens)
            .saturating_add(self.cache_read_input_tokens)
            .saturating_add(self.output_tokens)
    }
}

/// Pure helper: resolve the log path from a home directory and an
/// optional env override. Split out so the resolution logic can be
/// unit-tested without touching the real env or home dir.
pub fn resolve_tokens_log_path(home: &Path, env_override: Option<&str>) -> PathBuf {
    if let Some(dir) = env_override {
        return PathBuf::from(dir).join("log.jsonl");
    }
    home.join(".bpmux").join("tokens").join("log.jsonl")
}

/// Default log path: `~/.bpmux/tokens/log.jsonl`. Sibling of
/// `~/.bpmux/sessions/` so tokens and session data share a base but
/// don't entangle. Cross-session (not scoped under a specific session
/// dir) because the intended view is trend-over-time across many
/// sessions of a role's work.
///
/// Respects the `BPMUX_TOKENS_DIR` env var for test isolation — same
/// convention the adapter uses — so the `/tokens` view can be driven
/// end-to-end in a tempdir without touching the real log.
pub fn default_tokens_log_path() -> Result<PathBuf, LogError> {
    let home = dirs::home_dir().ok_or_else(|| {
        LogError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "home directory not found",
        ))
    })?;
    let env = std::env::var("BPMUX_TOKENS_DIR").ok();
    Ok(resolve_tokens_log_path(&home, env.as_deref()))
}

/// Append one row to the token log as a single JSON line. Creates
/// parent directories if missing. Caller's responsibility to handle
/// concurrent writers — the append mode is atomic for single-write
/// `write_all` calls on POSIX file systems, which is sufficient for
/// per-turn adapter writes (one process, one append at a time).
pub fn append_token_row(path: &Path, row: &TokenRow) -> Result<(), LogError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut line = serde_json::to_string(row)?;
    line.push('\n');
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

/// Sum of `row.total_tokens()` grouped by `row.role`. Powers the
/// per-role column in the `/tokens` view. `BTreeMap` (not `HashMap`)
/// so iteration order is deterministic — the view renders roles in
/// alphabetical order, which is stable across repeated renders and
/// doesn't require a separate sort pass.
pub fn totals_by_role(rows: &[TokenRow]) -> BTreeMap<String, u64> {
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    for row in rows {
        let entry = totals.entry(row.role.clone()).or_insert(0);
        *entry = entry.saturating_add(row.total_tokens());
    }
    totals
}

/// Read all rows from the token log, skipping any malformed lines.
/// Returns an empty `Vec` if the file does not exist (first-use case,
/// not an error). Malformed lines are silently skipped so one bad
/// adapter write can't poison the rest of the log — availability
/// of valid data over visibility of the corruption. A future diag
/// hook could surface skip counts if it becomes a problem.
pub fn read_token_rows(path: &Path) -> Result<Vec<TokenRow>, LogError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(row) = serde_json::from_str::<TokenRow>(&line) {
            rows.push(row);
        }
    }
    Ok(rows)
}
