#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;

use bookmark::{Bookmark, bookmark_path, load_bookmark, save_bookmark};

const PORT_WITH_NO_LISTENER: &str = "1";

fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn pre_compact_hook() -> PathBuf {
    repo_root()
        .join("claude-plugin")
        .join("hooks")
        .join("pre-compact.sh")
}

fn fire_hook(role: &str, sessions_base: &Path) -> std::process::Output {
    let mut cmd = Command::new("bash");
    cmd.arg(pre_compact_hook());
    cmd.env("BPMUX_ROLE", role);
    cmd.env("BPMUX_DATA_BASE", sessions_base);
    cmd.env("BPMUX_PORT", PORT_WITH_NO_LISTENER);
    cmd.env("PATH", std::env::var("PATH").unwrap_or_default());
    cmd.output().expect("run pre-compact hook")
}

#[test]
fn hook_creates_default_bookmark_when_none_exists() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sessions_base = tmp.path();
    let data_base_dir = sessions_base.join("session-001");
    std::fs::create_dir_all(&data_base_dir).expect("mkdir session");
    let current = sessions_base.join("current");
    std::fs::create_dir_all(&current).expect("mkdir current");

    let role = "dev";
    assert!(
        load_bookmark(&current, role) == Bookmark::default(),
        "precondition: no bookmark exists for role"
    );

    let out = fire_hook(role, sessions_base);
    assert!(
        out.status.success(),
        "hook must exit 0 (compaction-non-blocking); stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bookmark = load_bookmark(&current, role);
    assert_ne!(
        bookmark,
        Bookmark::default(),
        "hook must have written a bookmark file load_bookmark can read (round-trip per falsifier-c)"
    );
    assert!(
        !bookmark.last_updated.is_empty(),
        "stub bookmark must have last_updated timestamp"
    );
    assert!(
        bookmark.notes.contains("PreCompact") || !bookmark.notes.is_empty(),
        "stub bookmark must contain marker note; got: {:?}",
        bookmark.notes
    );
}

#[test]
fn hook_preserves_existing_bookmark_content_only_touches_timestamp() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sessions_base = tmp.path();
    let current = sessions_base.join("current");
    std::fs::create_dir_all(&current).expect("mkdir current");

    let role = "dev";
    let prior = Bookmark {
        current_task: "implementing #279 hook wiring".to_string(),
        decisions: vec!["use python3 for JSON merge".to_string()],
        open_questions: vec!["how to handle missing session-dir".to_string()],
        next_step: "write e2e test".to_string(),
        notes: "agent-saved earlier this session".to_string(),
        last_updated: "2026-05-05T10:00:00Z".to_string(),
    };
    save_bookmark(&current, role, &prior).expect("seed prior bookmark");

    let out = fire_hook(role, sessions_base);
    assert!(
        out.status.success(),
        "hook must exit 0; stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bookmark = load_bookmark(&current, role);
    assert_eq!(
        bookmark.current_task, prior.current_task,
        "existing current_task must be preserved (falsifier-b: no schema-drift / state-loss)"
    );
    assert_eq!(
        bookmark.decisions, prior.decisions,
        "existing decisions preserved"
    );
    assert_eq!(
        bookmark.open_questions, prior.open_questions,
        "existing open_questions preserved"
    );
    assert_eq!(
        bookmark.next_step, prior.next_step,
        "existing next_step preserved"
    );
    assert_eq!(bookmark.notes, prior.notes, "existing notes preserved");
    assert_ne!(
        bookmark.last_updated, prior.last_updated,
        "last_updated must be refreshed by hook so the file mtime is current"
    );
}

#[test]
fn hook_round_trips_through_bookmark_module_load_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sessions_base = tmp.path();
    let current = sessions_base.join("current");
    std::fs::create_dir_all(&current).expect("mkdir current");

    let role = "designer";
    let _ = fire_hook(role, sessions_base);

    let expected_path = bookmark_path(&current, role);
    assert!(
        expected_path.exists(),
        "hook must write to bookmark_path() ({}); falsifier-c file-shape match",
        expected_path.display()
    );

    let raw = std::fs::read_to_string(&expected_path).expect("read written bookmark");
    let parsed: Bookmark =
        serde_json::from_str(&raw).expect("hook output must parse as #168 Bookmark schema");
    assert!(!parsed.last_updated.is_empty());
}

#[test]
fn hook_role_with_unsafe_chars_sanitizes_to_alphanumerics() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sessions_base = tmp.path();
    let current = sessions_base.join("current");
    std::fs::create_dir_all(&current).expect("mkdir current");

    let dangerous = "../etc/passwd";
    let out = fire_hook(dangerous, sessions_base);
    assert!(out.status.success());

    let escape_target = sessions_base.join("etc").join("passwd");
    assert!(
        !escape_target.exists(),
        "role-traversal must not escape session-dir"
    );
}

#[test]
fn hook_idempotent_when_fired_twice_in_quick_succession() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let sessions_base = tmp.path();
    let current = sessions_base.join("current");
    std::fs::create_dir_all(&current).expect("mkdir current");

    let role = "pm";
    let _ = fire_hook(role, sessions_base);
    let bookmark1 = load_bookmark(&current, role);
    assert!(!bookmark1.last_updated.is_empty());

    std::thread::sleep(std::time::Duration::from_millis(1100));
    let _ = fire_hook(role, sessions_base);
    let bookmark2 = load_bookmark(&current, role);

    assert_eq!(
        bookmark2.notes, bookmark1.notes,
        "second fire preserves prior notes (since prior bookmark exists)"
    );
    assert_ne!(
        bookmark2.last_updated, bookmark1.last_updated,
        "second fire bumps last_updated"
    );
}
