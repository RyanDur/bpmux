#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::user_id_from_pin_or_dir;

#[test]
fn a_pinned_id_is_used_verbatim_ignoring_the_working_directory() {
    let dir = tempfile::TempDir::new().unwrap();
    let pinned = user_id_from_pin_or_dir(Some("agent-fixed-id".to_string()), dir.path())
        .expect("pinned resolves");
    assert_eq!(
        pinned, "agent-fixed-id",
        "a pinned BPMUX_USER_ID is the agent's ONE id — it wins over the cwd-derived marker, so cd-ing never splinters the agent into a new id"
    );
    assert!(
        !dir.path().join(".bpmux-user-id").exists(),
        "when pinned, no per-cwd marker is minted — the cwd is not consulted at all"
    );
}

#[test]
fn a_blank_pin_falls_back_to_the_dir_user_marker() {
    let dir = tempfile::TempDir::new().unwrap();
    let derived =
        user_id_from_pin_or_dir(Some("   ".to_string()), dir.path()).expect("blank pin falls back");
    let again = user_id_from_pin_or_dir(None, dir.path()).expect("none falls back");
    assert_eq!(
        derived, again,
        "a blank pin is treated as unset → both resolve to the SAME stable per-dir marker id (read_or_create reuses it)"
    );
    assert!(
        dir.path().join(".bpmux-user-id").exists(),
        "the unpinned path mints/reads the dir-user marker"
    );
}
