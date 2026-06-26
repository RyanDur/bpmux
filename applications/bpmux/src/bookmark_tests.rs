#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::bookmark::*;
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
