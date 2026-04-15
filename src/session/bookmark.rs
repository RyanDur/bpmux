//! Session bookmark / notes — #168.
//!
//! Structured session state that survives compaction. Each agent can
//! save a "where am I, what's happening, what's next" snapshot at
//! any time. The PreCompact hook (#166) triggers an auto-save. Post-
//! compaction, the agent reads the bookmark to resume with high
//! fidelity instead of reconstructing state from chat history.
//!
//! Storage: `{session-dir}/bookmark-{role}.json` — per-role, per-
//! session. Overwrites in place (current state, not history).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Structured session state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Bookmark {
    /// What the agent is currently working on.
    #[serde(default)]
    pub current_task: String,

    /// Key decisions made this session that matter for context.
    #[serde(default)]
    pub decisions: Vec<String>,

    /// Open questions or unresolved threads.
    #[serde(default)]
    pub open_questions: Vec<String>,

    /// What the agent expects to do next.
    #[serde(default)]
    pub next_step: String,

    /// Free-text notes for anything not structured.
    #[serde(default)]
    pub notes: String,

    /// When the bookmark was last updated.
    #[serde(default)]
    pub last_updated: String,
}

/// Path to a role's bookmark within a session directory.
pub fn bookmark_path(session_dir: &Path, role: &str) -> PathBuf {
    let safe_role = super::identity::sanitize_name(role);
    session_dir.join(format!("bookmark-{safe_role}.json"))
}

/// Load a bookmark. Returns empty bookmark if file doesn't exist.
pub fn load_bookmark(session_dir: &Path, role: &str) -> Bookmark {
    let path = bookmark_path(session_dir, role);
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => Bookmark::default(),
    }
}

/// Save a bookmark. Creates parent directories if needed.
pub fn save_bookmark(
    session_dir: &Path,
    role: &str,
    bookmark: &Bookmark,
) -> Result<(), std::io::Error> {
    std::fs::create_dir_all(session_dir)?;
    let path = bookmark_path(session_dir, role);
    let json = serde_json::to_string_pretty(bookmark)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn save_and_load_bookmark_roundtrips() {
        let dir = tempdir().unwrap();
        let bookmark = Bookmark {
            current_task: "implementing #190 identity layer".to_string(),
            decisions: vec![
                "two-layer memory: identity + lessons".to_string(),
                "identity is portable, lessons are session-scoped".to_string(),
            ],
            open_questions: vec!["curation mechanism for identity traits".to_string()],
            next_step: "wire MCP tool for identity queries".to_string(),
            notes: "scroll fix parked for headless harness".to_string(),
            last_updated: "2026-04-12T19:00:00Z".to_string(),
        };
        save_bookmark(dir.path(), "dev", &bookmark).unwrap();
        let loaded = load_bookmark(dir.path(), "dev");
        assert_eq!(loaded, bookmark);
    }

    #[test]
    fn load_bookmark_returns_empty_for_missing_file() {
        let dir = tempdir().unwrap();
        let loaded = load_bookmark(dir.path(), "dev");
        assert!(loaded.current_task.is_empty());
        assert!(loaded.decisions.is_empty());
    }

    #[test]
    fn per_role_bookmark_isolation() {
        let dir = tempdir().unwrap();
        let dev = Bookmark {
            current_task: "scroll fix".to_string(),
            ..Default::default()
        };
        let pm = Bookmark {
            current_task: "backlog grooming".to_string(),
            ..Default::default()
        };
        save_bookmark(dir.path(), "dev", &dev).unwrap();
        save_bookmark(dir.path(), "pm", &pm).unwrap();
        assert_eq!(load_bookmark(dir.path(), "dev").current_task, "scroll fix");
        assert_eq!(
            load_bookmark(dir.path(), "pm").current_task,
            "backlog grooming"
        );
    }
}
