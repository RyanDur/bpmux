use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

pub fn default_usage_path() -> Option<PathBuf> {
    if let Ok(override_path) = std::env::var("BPMUX_USAGE_PATH") {
        return Some(PathBuf::from(override_path));
    }
    dirs::home_dir().map(|h| h.join(".bpmux").join("usage.json"))
}

pub fn key_for(source: Source, feature: &str) -> String {
    format!("{}:{}", source.prefix(), feature)
}

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
    let tmp = unique_tmp_path(path);
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn unique_tmp_path(path: &std::path::Path) -> PathBuf {
    static TMP_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = TMP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let base = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("usage.json");
    let tmp_name = format!("{base}.{}.{seq}.tmp", std::process::id());
    match path.parent() {
        Some(parent) => parent.join(tmp_name),
        None => PathBuf::from(tmp_name),
    }
}

pub fn record(source: Source, feature: &str) {
    let Some(path) = default_usage_path() else {
        return;
    };
    let mut map = load_from(&path);
    let key = key_for(source, feature);
    let next = map.get(&key).map(|r| r.count + 1).unwrap_or(1);
    map.insert(key, Record::now(next));
    if let Err(e) = save_to(&path, &map) {
        crate::diag::log_diag(&format!("usage::record save failed: {e}"));
    }
}

pub fn parse_key(key: &str) -> Option<(Source, &str)> {
    let (prefix, rest) = key.split_once(':')?;
    let source = match prefix {
        "slash" => Source::Slash,
        "mcp" => Source::Mcp,
        _ => return None,
    };
    Some((source, rest))
}

pub fn last_used_datetime(record: &Record) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&record.last_used)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
