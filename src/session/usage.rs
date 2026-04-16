//! Feature-usage persistence (#195 extended by #230).
//!
//! Shared on-disk counter at `~/.bpmux/usage.json` that accumulates
//! invocation counts across sessions and surfaces (TUI slash commands +
//! MCP tool calls). Both the TUI process and bridge subprocesses call
//! `record(source, feature)` which loads, increments, and writes back.
//!
//! Keys are `"source:feature"` so readers can group by source without
//! a structural change. Values are `(count, last_used_rfc3339)`.
//!
//! Concurrent writes from separate processes race; last-writer-wins is
//! acceptable for this use case — the data is informational, not
//! authoritative. Under typical use (one TUI + a few bridges) the
//! collision window is ms-scale.
//!
//! Before #230 each caller started with an empty in-memory map and
//! overwrote the file on save, silently erasing prior counts. This
//! module's load-before-save eliminates that loss mode.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Source tag for a usage record. Keeps slash-command invocations
/// distinguishable from MCP tool invocations at read time without
/// a structural change to the persisted map.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Slash,
    Mcp,
}

impl Source {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Slash => "slash",
            Self::Mcp => "mcp",
        }
    }
}

/// Stored record — count + last invocation timestamp. Serialized as
/// a tuple to preserve wire compatibility with #195's initial format
/// (HashMap<String, (usize, String)>).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub count: usize,
    pub last_used: String,
}

impl Record {
    pub fn now(count: usize) -> Self {
        Self {
            count,
            last_used: Utc::now().to_rfc3339(),
        }
    }
}

/// Default on-disk path: `~/.bpmux/usage.json`. Overridable via
/// `BPMUX_USAGE_PATH` so tests (and anyone needing a sandboxed store)
/// can redirect writes into a tempdir without having to thread the
/// path through every call site.
pub fn default_usage_path() -> Option<PathBuf> {
    if let Ok(override_path) = std::env::var("BPMUX_USAGE_PATH") {
        return Some(PathBuf::from(override_path));
    }
    dirs::home_dir().map(|h| h.join(".bpmux").join("usage.json"))
}

/// Key format for a single invocation. Pure so we can test the
/// prefix convention without touching disk.
pub fn key_for(source: Source, feature: &str) -> String {
    format!("{}:{}", source.prefix(), feature)
}

/// Load existing usage from the default path. Returns empty on any
/// failure (missing file, parse error) — callers treat absent data
/// as "no prior counts" rather than propagating the error, since
/// this is an informational store.
pub fn load() -> HashMap<String, Record> {
    let Some(path) = default_usage_path() else {
        return HashMap::new();
    };
    load_from(&path)
}

pub fn load_from(path: &std::path::Path) -> HashMap<String, Record> {
    let Ok(contents) = std::fs::read_to_string(path) else {
        return HashMap::new();
    };
    // Parse the current format first; fall back to #195's legacy
    // tuple format so existing files don't lose data.
    if let Ok(map) = serde_json::from_str::<HashMap<String, Record>>(&contents) {
        return map;
    }
    if let Ok(legacy) = serde_json::from_str::<HashMap<String, (usize, String)>>(&contents) {
        return legacy
            .into_iter()
            .map(|(k, (c, t))| {
                (
                    k,
                    Record {
                        count: c,
                        last_used: t,
                    },
                )
            })
            .collect();
    }
    HashMap::new()
}

/// Save atomically: write to a temp file in the same directory, then
/// rename. Prevents readers from seeing a half-written file.
pub fn save(map: &HashMap<String, Record>) -> Result<(), std::io::Error> {
    let Some(path) = default_usage_path() else {
        return Ok(());
    };
    save_to(&path, map)
}

pub fn save_to(
    path: &std::path::Path,
    map: &HashMap<String, Record>,
) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(map)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Increment the counter for `(source, feature)`. Loads current
/// state, bumps the record, saves atomically. Best-effort on IO
/// failure — the session still works, counts just drift.
pub fn record(source: Source, feature: &str) {
    let Some(path) = default_usage_path() else {
        return;
    };
    let mut map = load_from(&path);
    let key = key_for(source, feature);
    let next = map.get(&key).map(|r| r.count + 1).unwrap_or(1);
    map.insert(key, Record::now(next));
    if let Err(e) = save_to(&path, &map) {
        crate::session::diag::log_diag(&format!("usage::record save failed: {e}"));
    }
}

/// Parse a stored key back into `(source, feature)`. Returns None on
/// keys without a recognized prefix (legacy entries from #195 that
/// pre-date source tagging).
pub fn parse_key(key: &str) -> Option<(Source, &str)> {
    let (prefix, rest) = key.split_once(':')?;
    let source = match prefix {
        "slash" => Source::Slash,
        "mcp" => Source::Mcp,
        _ => return None,
    };
    Some((source, rest))
}

/// Convert the record's RFC3339 timestamp back into a DateTime.
/// Used by UI surfaces that want to compute "X min ago."
pub fn last_used_datetime(record: &Record) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&record.last_used)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
