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
fn routing_survives_rename_user_id_is_the_key_display_is_a_mutable_attribute() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-uuid-1", "dev", "dev");
    assert_eq!(identity.resolve("dev"), vec!["sess-uuid-1".to_string()]);

    identity.rename("sess-uuid-1", "dev-renamed");
    assert_eq!(
        identity.resolve("dev-renamed"),
        vec!["sess-uuid-1".to_string()],
        "rename moves the DISPLAY; the new display must still resolve to the same session-id"
    );
    assert!(
        identity.resolve("dev").is_empty(),
        "the OLD display must no longer route — the route-key (session-id) never moved"
    );
    assert!(identity.contains_id("sess-uuid-1"));
}

#[test]
fn reconnecting_same_user_id_reclaims_its_cursor_does_not_reset_it() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-1", "dev", "dev");
    identity.advance_cursor("sess-1", "2026-06-11T20:00:00Z");

    identity.register("sess-1", "dev-renamed", "dev");

    assert_eq!(
        identity.cursor("sess-1"),
        Some("2026-06-11T20:00:00Z".to_string()),
        "a reconnecting same-session-id reclaims its existing cursor (drains from where it left off) — register preserves the cursor while it updates the display"
    );
}

#[test]
fn drain_identification_returns_the_missed_set_strictly_after_the_reclaimed_cursor() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-1", "dev", "dev");
    identity.advance_cursor("sess-1", "2026-06-11T20:00:02Z");
    let events = vec![
        "2026-06-11T20:00:01Z".to_string(),
        "2026-06-11T20:00:02Z".to_string(),
        "2026-06-11T20:00:03Z".to_string(),
        "2026-06-11T20:00:04Z".to_string(),
    ];
    assert_eq!(
        identity.missed_after_cursor("sess-1", &events),
        vec![
            "2026-06-11T20:00:03Z".to_string(),
            "2026-06-11T20:00:04Z".to_string()
        ],
        "the reclaimed cursor identifies the missed-events SET — strictly AFTER the cursor (the cursor position was the last delivered, not missed)"
    );
}

#[test]
fn linking_a_terminal_lets_delivery_find_the_live_pty_and_relinks_on_reconnect() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-1", "dev", "dev");
    assert_eq!(
        identity.terminal_for("sess-1"),
        None,
        "a registered id with no live terminal yet (offline, or an orphaned id whose owner was deleted) links to NO pty — delivery to it routes to nobody, never mis-routes"
    );

    identity.link_terminal("sess-1", "term-A");
    assert_eq!(identity.terminal_for("sess-1"), Some("term-A".to_string()));

    identity.link_terminal("sess-1", "term-B");
    assert_eq!(
        identity.terminal_for("sess-1"),
        Some("term-B".to_string()),
        "a reconnecting same-session-id re-links to its NEW live pty — delivery follows the agent across a restart"
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

#[test]
fn resolve_recipients_broadcast_is_all_ids_scoped_is_resolved_ids_unknown_is_no_leak() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-1", "dev", "dev");
    identity.register("sess-2", "pm", "pm");

    let mut broadcast = identity.resolve_recipients(None);
    broadcast.sort();
    assert_eq!(
        broadcast,
        vec!["sess-1".to_string(), "sess-2".to_string()],
        "broadcast (to=None) routes to every registered instance"
    );

    assert_eq!(
        identity.resolve_recipients(Some(&["pm".to_string()])),
        vec!["sess-2".to_string()],
        "a scoped message resolves the display-name to its session-id"
    );

    assert!(
        identity
            .resolve_recipients(Some(&["ghost".to_string()]))
            .is_empty(),
        "a scoped message to an unknown display routes to NO instance — no mis-route/leak"
    );
}

#[test]
fn display_for_terminal_reverse_resolves_the_agent_bound_to_a_terminal() {
    let mut identity = RoutingIdentity::new();
    identity.register("sess-dev", "dev", "dev");
    identity.link_terminal("sess-dev", "term-1");
    assert_eq!(
        identity.display_for_terminal("term-1"),
        Some("dev".to_string()),
        "the reverse lookup finds the DISPLAY bound to a terminal — used on terminal-death to roster-leave the right agent so the roster tracks terminal liveness"
    );
    assert_eq!(
        identity.display_for_terminal("term-unbound"),
        None,
        "a terminal bound to no session resolves to no agent — death of an unregistered terminal leaves the roster untouched"
    );
}

#[test]
fn reap_removes_unclaimed_duplicates_of_a_live_display_but_never_an_offline_other_display() {
    let mut identity = RoutingIdentity::new();
    identity.register("dev-live", "dev", "dev");
    identity.link_terminal("dev-live", "term-dev");
    identity.register("dev-stale-a", "dev", "dev");
    identity.advance_cursor("dev-stale-a", "2026-06-13T00:00:00Z");
    identity.register("dev-stale-b", "dev", "dev");
    identity.register("pm-offline", "pm", "pm");
    identity.advance_cursor("pm-offline", "2026-06-13T00:00:00Z");

    let reaped = identity.reap_unclaimed_duplicates_of("dev");

    assert!(
        reaped.contains(&"dev-stale-a".to_string()) && reaped.contains(&"dev-stale-b".to_string()),
        "the unclaimed (no live terminal) dev duplicates — the splinters from the old per-cwd model — are reaped; got {reaped:?}"
    );
    assert_eq!(
        identity.resolve("dev"),
        vec!["dev-live".to_string()],
        "the LIVE (terminal-bound) dev is preserved — only the one true dev remains"
    );
    assert!(
        identity.cursor("dev-stale-a").is_none(),
        "a reaped id's durable cursor is removed too (it was dead history)"
    );
    assert_eq!(
        identity.resolve("pm"),
        vec!["pm-offline".to_string()],
        "an OFFLINE agent of a DIFFERENT display is untouched — reap is scoped to the display whose live terminal just bound, so a legitimately-offline agent (no live sibling) keeps its id"
    );
    assert_eq!(
        identity.cursor("pm-offline"),
        Some("2026-06-13T00:00:00Z".to_string()),
        "the offline agent's durable cursor survives — the discriminator never breaks offline-reconnect"
    );
}
