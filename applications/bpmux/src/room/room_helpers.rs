#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn resolve_falls_back_to_home_dotbpmux_rooms_when_env_unset() {
    let dir = resolve_rooms_dir(None, Some("/home/agent".into()));
    assert_eq!(dir, PathBuf::from("/home/agent/.bpmux/rooms"));
}

#[test]
fn resolve_honours_env_override() {
    let dir = resolve_rooms_dir(
        Some("/tmp/bpmux-rooms-test".into()),
        Some("/ignored".into()),
    );
    assert_eq!(dir, PathBuf::from("/tmp/bpmux-rooms-test"));
}

#[test]
fn resolve_falls_back_to_tmp_when_both_unset() {
    let dir = resolve_rooms_dir(None, None);
    assert_eq!(dir, PathBuf::from("/tmp/.bpmux/rooms"));
}

#[test]
fn strip_ansi_removes_color_escape_sequences() {
    let raw = "\x1b[31mred\x1b[0m plain";
    assert_eq!(strip_ansi(raw), "red plain");
    assert!(!strip_ansi(raw).contains('\u{1b}'));
}

#[test]
fn strip_ansi_leaves_clean_text_untouched() {
    assert_eq!(strip_ansi("just text\nline two"), "just text\nline two");
}

#[test]
fn room_round_trips_through_json() {
    let room = Room {
        role: "pm".to_string(),
        agent: "claude-code".to_string(),
        pid: 12345,
        created_at: chrono::Utc::now(),
    };
    let json = serde_json::to_string(&room).unwrap();
    let parsed: Room = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, room);
}
