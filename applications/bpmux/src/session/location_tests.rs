#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::location::*;
use crate::session::Message;
use crate::session::log::{append_message, read_messages};
use chrono::Utc;
use tempfile::TempDir;

#[test]
fn creates_session_directory() {
    let base = TempDir::new().expect("temp base");
    let dir = create_session(base.path()).expect("create session");
    assert!(dir.exists());
    assert!(dir.is_dir());
    let current = base.path().join("current");
    assert!(current.is_symlink());
}

#[test]
fn resumes_existing_session() {
    let base = TempDir::new().expect("temp base");
    let dir1 = create_session(base.path()).expect("create session");
    assert!(dir1.exists());
    let msg = Message {
        from: "owner:human".to_string(),
        text: "persisted message".to_string(),
        ts: Utc::now(),
        ..Message::default()
    };
    append_message(&dir1, &msg).expect("append");
    let dir2 = resume_or_create_session(base.path()).expect("resume");
    assert_eq!(dir1, dir2);
    let messages = read_messages(&dir2).expect("read");
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].text, "persisted message");
}

#[test]
fn dir_session_id_is_held_in_the_dir_minted_once_then_reclaimed_via_a_lightweight_marker() {
    let parent = TempDir::new().expect("temp dir");
    let dir = parent.path().join("my-project");
    std::fs::create_dir_all(&dir).expect("mkdir");
    let first = read_or_create_dir_session(&dir).expect("mint");
    assert!(!first.id.is_empty());
    assert_eq!(
        first.name, "my-project",
        "the name defaults to the folder basename at mint — a self-describing label until renamed in the owner-conversation"
    );

    let second = read_or_create_dir_session(&dir).expect("reclaim");
    assert_eq!(
        first, second,
        "the {{id, name}} is HELD in the dir — a second derivation (restart in the same dir) reclaims the same identity, never mints a new one (owner's dir-anchor)"
    );

    assert!(
        dir.join(DIR_SESSION_ID_MARKER).is_file(),
        "the identity lives in a lightweight hidden marker — not a full session dir, so registering an agent never pollutes its workspace"
    );
    assert!(
        !dir.join("current").exists(),
        "no `current` symlink / session-dir scaffolding is written into the workspace (the pollution resume_or_create_session(cwd) would have caused)"
    );
}

#[test]
fn a_pre_id_name_marker_reads_back_compat_keeping_the_id_and_deriving_the_name() {
    let parent = TempDir::new().expect("temp dir");
    let dir = parent.path().join("legacy-dir");
    std::fs::create_dir_all(&dir).expect("mkdir");
    std::fs::write(dir.join(DIR_SESSION_ID_MARKER), "legacy-uuid-1234").expect("write old marker");

    let session = read_or_create_dir_session(&dir).expect("read back-compat");
    assert_eq!(
        session.id, "legacy-uuid-1234",
        "a bare-id (pre-{{id,name}}) marker keeps its id — the durable route-key never changes across the format upgrade"
    );
    assert_eq!(
        session.name, "legacy-dir",
        "the missing name is derived from the folder basename — an old marker stays routable with no migration step"
    );
}
