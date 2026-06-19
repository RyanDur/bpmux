use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Serialize, Deserialize)]
pub(super) struct PersistedSubscription {
    pub(super) id: String,
    pub(super) event_class: String,
    pub(super) filter: Option<Map<String, Value>>,
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn subscriptions_path(name: &str) -> Option<PathBuf> {
    let dir = crate::session::log::default_sessions_base()
        .ok()?
        .join("bridge-subscriptions");
    Some(dir.join(format!("{}.json", sanitize_name(name))))
}

pub(super) fn cursor_path(name: &str) -> Option<PathBuf> {
    let dir = crate::session::log::default_sessions_base()
        .ok()?
        .join("bridge-subscriptions");
    Some(dir.join(format!("{}.cursor", sanitize_name(name))))
}

pub(super) fn load_cursor(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub(super) fn write_cursor(path: &Path, cursor: &str) {
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let tmp = path.with_extension("cursor.tmp");
    if std::fs::write(&tmp, cursor).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

pub(super) fn load_persisted(path: &Path) -> Vec<PersistedSubscription> {
    match std::fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

pub(super) fn write_persisted(path: &Path, records: &[PersistedSubscription]) {
    let Some(parent) = path.parent() else { return };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(body) = serde_json::to_string_pretty(records) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, body).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}
