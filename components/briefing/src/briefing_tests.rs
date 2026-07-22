#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use tempfile::tempdir;

#[test]
fn save_and_load_round_trips() {
    let dir = tempdir().unwrap();
    save_briefing_in(dir.path(), "dev", "hello briefing").unwrap();
    let loaded = load_briefing_in(dir.path(), "dev");
    assert_eq!(loaded, "hello briefing");
}

#[test]
fn missing_file_loads_as_empty_string() {
    let dir = tempdir().unwrap();
    let loaded = load_briefing_in(dir.path(), "nobody");
    assert_eq!(loaded, "");
}

#[test]
fn save_creates_parent_dir_if_missing() {
    let root = tempdir().unwrap();
    let nested = root.path().join("does").join("not").join("exist");
    save_briefing_in(&nested, "dev", "content").unwrap();
    assert!(nested.join("dev.md").exists());
}

#[test]
fn save_rejects_content_over_cap() {
    let dir = tempdir().unwrap();
    let big = "a".repeat(MAX_BRIEFING_BYTES + 1);
    let result = save_briefing_in(dir.path(), "dev", &big);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn save_accepts_content_at_cap() {
    let dir = tempdir().unwrap();
    let at_cap = "a".repeat(MAX_BRIEFING_BYTES);
    let result = save_briefing_in(dir.path(), "dev", &at_cap);
    assert!(result.is_ok());
}

#[test]
fn per_role_isolation() {
    let dir = tempdir().unwrap();
    save_briefing_in(dir.path(), "dev", "dev briefing").unwrap();
    save_briefing_in(dir.path(), "pm", "pm briefing").unwrap();
    assert_eq!(load_briefing_in(dir.path(), "dev"), "dev briefing");
    assert_eq!(load_briefing_in(dir.path(), "pm"), "pm briefing");
}

#[test]
fn role_name_sanitized_against_path_traversal() {
    let dir = tempdir().unwrap();
    save_briefing_in(dir.path(), "../escape", "pwned").unwrap();
    let path = briefing_path_in(dir.path(), "../escape");
    assert!(path.starts_with(dir.path()));
    let parent = dir
        .path()
        .parent()
        .expect("tempdir parent")
        .join("escape.md");
    assert!(
        !parent.exists(),
        "path traversal should not escape root; leaked to {parent:?}"
    );
}

#[test]
fn save_replaces_existing_content() {
    let dir = tempdir().unwrap();
    save_briefing_in(dir.path(), "dev", "first").unwrap();
    save_briefing_in(dir.path(), "dev", "second").unwrap();
    assert_eq!(load_briefing_in(dir.path(), "dev"), "second");
}
