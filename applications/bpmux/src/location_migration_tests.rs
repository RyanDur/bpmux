#![allow(clippy::unwrap_used, clippy::expect_used)]

use crate::location::*;
use tempfile::TempDir;

#[test]
fn migrate_legacy_sessions_base_renames_sessions_to_data_and_repoints_current() {
    let parent = TempDir::new().expect("tmp parent");
    let sessions = parent.path().join("sessions");
    let agent_dir = sessions.join("agent-uuid");
    std::fs::create_dir_all(&agent_dir).expect("mkdir agent dir");
    std::fs::write(agent_dir.join("messages-current.jsonl"), "content").expect("seed content");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&agent_dir, sessions.join("current")).expect("symlink current");

    migrate_legacy_sessions_base(parent.path()).expect("migrate");

    let data = parent.path().join("data");
    assert!(data.is_dir(), "data/ now holds the migrated content");
    assert!(
        !sessions.exists(),
        "sessions/ is gone after the atomic rename"
    );
    let new_current = data.join("current");
    assert!(
        new_current.is_symlink(),
        "the current symlink survives the rename"
    );
    let target = std::fs::read_link(&new_current).expect("read relinked current");
    assert_eq!(
        target,
        data.join("agent-uuid"),
        "the ABSOLUTE current symlink is re-pointed from the old sessions-prefixed target to the same basename under data/"
    );
    assert_eq!(
        std::fs::read_to_string(new_current.join("messages-current.jsonl"))
            .expect("read migrated content"),
        "content"
    );
}

#[test]
fn migrate_legacy_sessions_base_second_call_is_a_no_op() {
    let parent = TempDir::new().expect("tmp parent");
    let sessions = parent.path().join("sessions");
    let agent_dir = sessions.join("agent-uuid");
    std::fs::create_dir_all(&agent_dir).expect("mkdir agent dir");
    std::fs::write(agent_dir.join("messages-current.jsonl"), "content").expect("seed content");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&agent_dir, sessions.join("current")).expect("symlink current");

    migrate_legacy_sessions_base(parent.path()).expect("first migrate");
    let data = parent.path().join("data");
    let before = std::fs::read_to_string(data.join("agent-uuid").join("messages-current.jsonl"))
        .expect("read once");

    migrate_legacy_sessions_base(parent.path()).expect("second migrate is a no-op");

    let after = std::fs::read_to_string(data.join("agent-uuid").join("messages-current.jsonl"))
        .expect("read again");
    assert_eq!(
        before, after,
        "second call no-ops — contents byte-identical"
    );
    assert!(
        !parent.path().join("sessions").exists(),
        "sessions stays gone; the no-op path never resurrects it"
    );
}

#[test]
fn migrate_legacy_sessions_base_leaves_both_untouched_when_data_already_exists() {
    let parent = TempDir::new().expect("tmp parent");
    let data = parent.path().join("data");
    std::fs::create_dir_all(&data).expect("mkdir data");
    std::fs::write(data.join("marker"), "authoritative").expect("seed data content");

    let sessions = parent.path().join("sessions");
    std::fs::create_dir_all(&sessions).expect("mkdir sessions leftover");
    std::fs::write(sessions.join("marker"), "leftover").expect("seed leftover content");

    migrate_legacy_sessions_base(parent.path()).expect("no-op with both present");

    assert_eq!(
        std::fs::read_to_string(data.join("marker")).expect("data untouched"),
        "authoritative",
        "data/ wins — never merged with or overwritten by the leftover"
    );
    assert_eq!(
        std::fs::read_to_string(sessions.join("marker")).expect("sessions untouched"),
        "leftover",
        "the leftover sessions/ is neither merged nor deleted"
    );
}

#[test]
fn migrate_legacy_sessions_base_is_a_no_op_when_neither_dir_exists() {
    let parent = TempDir::new().expect("tmp parent");
    migrate_legacy_sessions_base(parent.path()).expect("fresh install no-op");
    assert!(
        !parent.path().join("data").exists(),
        "no data/ is conjured on a fresh install"
    );
    assert!(
        !parent.path().join("sessions").exists(),
        "no sessions/ is conjured either"
    );
}

#[test]
fn resolve_base_with_migration_returns_the_override_and_leaves_the_parent_untouched() {
    let parent = TempDir::new().expect("tmp parent");
    let sessions = parent.path().join("sessions");
    std::fs::create_dir_all(&sessions).expect("mkdir sessions leftover");
    std::fs::write(sessions.join("marker"), "leftover").expect("seed leftover content");

    let override_dir = TempDir::new().expect("tmp override");
    let resolved =
        resolve_base_with_migration(Some(override_dir.path().to_path_buf()), parent.path())
            .expect("resolve override");

    assert_eq!(resolved, override_dir.path());
    assert!(
        sessions.join("marker").is_file(),
        "the sessions/ sibling under the (unused) default parent stays exactly as seeded — an explicit override never migrates it"
    );
    assert!(
        !parent.path().join("data").exists(),
        "no data/ is conjured under the default parent when an override is in play"
    );
}
