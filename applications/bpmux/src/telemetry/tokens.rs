use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};

use crate::session::LogError;

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
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens
            .saturating_add(self.cache_creation_input_tokens)
            .saturating_add(self.cache_read_input_tokens)
            .saturating_add(self.output_tokens)
    }
}

pub fn resolve_tokens_log_path(home: &Path, env_override: Option<&str>) -> PathBuf {
    if let Some(dir) = env_override {
        return PathBuf::from(dir).join("log.jsonl");
    }
    home.join(".bpmux").join("tokens").join("log.jsonl")
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenReport {
    pub total_tokens: u64,
    pub row_count: usize,
    pub by_role: BTreeMap<String, u64>,
    pub recent: Vec<TokenReportEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenReportEntry {
    pub ts: String,
    pub role: String,
    pub harness: String,
    pub model: String,
    pub total_tokens: u64,
    pub output_tokens: u64,
    pub tool_call_count: u32,
}

pub fn build_token_report(rows: &[TokenRow]) -> TokenReport {
    const RECENT_LIMIT: usize = 10;
    let total_tokens: u64 = rows.iter().map(|r| r.total_tokens()).sum();
    let by_role = totals_by_role(rows);
    let recent: Vec<TokenReportEntry> = rows
        .iter()
        .rev()
        .take(RECENT_LIMIT)
        .map(|r| TokenReportEntry {
            ts: r.ts.clone(),
            role: r.role.clone(),
            harness: r.harness.clone(),
            model: r.model.clone(),
            total_tokens: r.total_tokens(),
            output_tokens: r.output_tokens,
            tool_call_count: r.tool_call_count,
        })
        .collect();
    TokenReport {
        total_tokens,
        row_count: rows.len(),
        by_role,
        recent,
    }
}

pub fn totals_by_role(rows: &[TokenRow]) -> BTreeMap<String, u64> {
    let mut totals: BTreeMap<String, u64> = BTreeMap::new();
    for row in rows {
        let entry = totals.entry(row.role.clone()).or_insert(0);
        *entry = entry.saturating_add(row.total_tokens());
    }
    totals
}

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
