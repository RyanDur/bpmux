#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

#[test]
fn role_for_returns_the_assigned_role_independent_of_the_display() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-aria", "aria", "dev");
    assert_eq!(
        identity.role_for("sess-aria"),
        Some("dev".to_string()),
        "the role assignment is a distinct fact from the display — 'aria' carries no role prefix at all"
    );
}

#[test]
fn role_for_is_none_for_an_id_never_registered() {
    let identity = RoutingIdentity::new();
    assert_eq!(
        identity.role_for("sess-ghost"),
        None,
        "an id with no registration fact has no assignment — callers fall back to display-derivation"
    );
}

#[test]
fn rename_moves_the_display_but_never_touches_the_role_assignment() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-1", "dev", "dev");
    identity.rename("sess-1", "dev-renamed");
    assert_eq!(
        identity.role_for("sess-1"),
        Some("dev".to_string()),
        "a rename is a presentation change — it must never relaunder the role assignment"
    );
}

#[test]
fn unclaimed_is_the_registered_ids_with_no_live_terminal_the_reclaim_dropdown_set() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-live", "dev", "dev");
    identity.register("sess-orphan", "former-pm", "former-pm");
    identity.link_terminal("sess-live", "term-A");

    assert_eq!(
        identity.unclaimed(),
        vec![("sess-orphan".to_string(), "former-pm".to_string())],
        "unclaimed = a registered id whose owner is gone (no live terminal) — the set a user picks from to reclaim a slice of the conversation; a claimed (live-linked) id is NOT offered"
    );

    identity.link_terminal("sess-orphan", "term-B");
    assert!(
        identity.unclaimed().is_empty(),
        "claiming the orphan (linking a terminal) removes it from the unclaimed set — nothing unclaimed → no dropdown"
    );
}
