#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{or_register_user_id_from_pin_or_dir, registered_user_id_from_pin_or_dir};

#[test]
fn a_pinned_id_is_used_verbatim_and_never_touches_the_registry() {
    let base = tempfile::TempDir::new().unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let pinned = or_register_user_id_from_pin_or_dir(
        Some("agent-fixed-id".to_string()),
        base.path(),
        dir.path(),
    )
    .expect("pinned resolves");
    assert_eq!(
        pinned, "agent-fixed-id",
        "a pinned BPMUX_USER_ID is the agent's ONE id — it wins over the cwd-derived registry, so cd-ing never splinters the agent into a new id"
    );
    assert!(
        !base.path().join("users").join("by-dir").exists(),
        "when pinned, the registry is not consulted or written at all — the pin short-circuits before any dir lookup"
    );
    assert!(
        !dir.path().join(".bpmux-user-id").exists(),
        "and no marker is minted in the cwd either"
    );
}

#[test]
fn a_pinned_read_also_wins_verbatim_without_registering() {
    let base = tempfile::TempDir::new().unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let resolved = registered_user_id_from_pin_or_dir(
        Some("agent-fixed-id".to_string()),
        base.path(),
        dir.path(),
    );
    assert_eq!(
        resolved.as_deref(),
        Some("agent-fixed-id"),
        "the read path honors the pin verbatim — a pinned agent always resolves its id, even from a dir that was never registered"
    );
    assert!(
        !base.path().join("users").join("by-dir").exists(),
        "the pinned read touches no registry"
    );
}

#[test]
fn a_blank_pin_falls_back_to_the_dir_registry() {
    let base = tempfile::TempDir::new().unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let derived =
        or_register_user_id_from_pin_or_dir(Some("   ".to_string()), base.path(), dir.path())
            .expect("blank pin falls back to register");
    let again = or_register_user_id_from_pin_or_dir(None, base.path(), dir.path())
        .expect("none falls back to reclaim");
    assert_eq!(
        derived, again,
        "a blank pin is treated as unset → both resolve to the SAME stable per-dir registry id"
    );
    assert!(
        base.path().join("users").join("by-dir").exists(),
        "the unpinned register path writes the dir-user entry into the registry"
    );
}

#[test]
fn an_unpinned_read_of_an_unregistered_dir_resolves_nothing() {
    let base = tempfile::TempDir::new().unwrap();
    let dir = tempfile::TempDir::new().unwrap();
    let resolved = registered_user_id_from_pin_or_dir(None, base.path(), dir.path());
    assert!(
        resolved.is_none(),
        "an unpinned read from an unregistered dir resolves no id and mints nothing — send simply omits sender_id"
    );
    assert!(
        !base.path().join("users").join("by-dir").exists(),
        "and writes no registry entry"
    );
}
