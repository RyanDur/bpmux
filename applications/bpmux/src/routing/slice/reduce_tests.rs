#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{
    RoutingAction, RoutingState, cursor, delivery_targets, display_for_terminal,
    missed_after_cursor, reduce, resolve, resolve_recipients, terminal_for,
};

fn apply(state: &RoutingState, action: RoutingAction) -> RoutingState {
    reduce(state, &action).unwrap_or_else(|| state.clone())
}

fn registered(state: &RoutingState, session_id: &str, display: &str) -> RoutingState {
    apply(
        state,
        RoutingAction::Registered {
            session_id: session_id.to_string(),
            display: display.to_string(),
        },
    )
}

fn linked(state: &RoutingState, session_id: &str, terminal_id: &str) -> RoutingState {
    apply(
        state,
        RoutingAction::TerminalLinked {
            session_id: session_id.to_string(),
            terminal_id: terminal_id.to_string(),
        },
    )
}

fn cursor_advanced(state: &RoutingState, session_id: &str, cursor: &str) -> RoutingState {
    apply(
        state,
        RoutingAction::CursorAdvanced {
            session_id: session_id.to_string(),
            cursor: cursor.to_string(),
        },
    )
}

#[test]
fn routing_survives_rename_session_id_is_the_key_display_is_a_mutable_attribute() {
    let state = registered(&RoutingState::default(), "sess-uuid-1", "dev");
    assert_eq!(resolve(&state, "dev"), vec!["sess-uuid-1".to_string()]);

    let renamed = apply(
        &state,
        RoutingAction::Renamed {
            session_id: "sess-uuid-1".to_string(),
            display: "dev-renamed".to_string(),
        },
    );
    assert_eq!(
        resolve(&renamed, "dev-renamed"),
        vec!["sess-uuid-1".to_string()],
        "rename moves the DISPLAY; the new display must still resolve to the same session-id"
    );
    assert!(
        resolve(&renamed, "dev").is_empty(),
        "the OLD display must no longer route — the route-key (session-id) never moved"
    );
}

#[test]
fn reconnecting_same_session_id_reclaims_its_cursor_does_not_reset_it() {
    let state = registered(&RoutingState::default(), "sess-1", "dev");
    let state = cursor_advanced(&state, "sess-1", "2026-06-11T20:00:00Z");

    let reconnected = registered(&state, "sess-1", "dev-renamed");

    assert_eq!(
        cursor(&reconnected, "sess-1"),
        Some("2026-06-11T20:00:00Z".to_string()),
        "a reconnecting same-session-id reclaims its existing cursor — Registered preserves the cursor while it updates the display"
    );
}

#[test]
fn drain_identification_returns_the_missed_set_strictly_after_the_reclaimed_cursor() {
    let state = registered(&RoutingState::default(), "sess-1", "dev");
    let state = cursor_advanced(&state, "sess-1", "2026-06-11T20:00:02Z");
    let events = vec![
        "2026-06-11T20:00:01Z".to_string(),
        "2026-06-11T20:00:02Z".to_string(),
        "2026-06-11T20:00:03Z".to_string(),
        "2026-06-11T20:00:04Z".to_string(),
    ];
    assert_eq!(
        missed_after_cursor(&state, "sess-1", &events),
        vec![
            "2026-06-11T20:00:03Z".to_string(),
            "2026-06-11T20:00:04Z".to_string()
        ],
        "the reclaimed cursor identifies the missed-events SET — strictly AFTER the cursor"
    );
}

#[test]
fn linking_a_terminal_lets_delivery_find_the_live_pty_and_relinks_on_reconnect() {
    let state = registered(&RoutingState::default(), "sess-1", "dev");
    assert_eq!(
        terminal_for(&state, "sess-1"),
        None,
        "a registered id with no live terminal yet links to NO pty — delivery to it routes to nobody"
    );

    let state = linked(&state, "sess-1", "term-A");
    assert_eq!(terminal_for(&state, "sess-1"), Some("term-A".to_string()));

    let state = linked(&state, "sess-1", "term-B");
    assert_eq!(
        terminal_for(&state, "sess-1"),
        Some("term-B".to_string()),
        "a reconnecting same-session-id re-links to its NEW live pty — delivery follows the agent across a restart"
    );
}

#[test]
fn resolve_recipients_broadcast_is_all_ids_scoped_is_resolved_ids_unknown_is_no_leak() {
    let state = registered(&RoutingState::default(), "sess-1", "dev");
    let state = registered(&state, "sess-2", "pm");

    let mut broadcast = resolve_recipients(&state, None);
    broadcast.sort();
    assert_eq!(
        broadcast,
        vec!["sess-1".to_string(), "sess-2".to_string()],
        "broadcast (to=None) routes to every registered instance"
    );

    assert_eq!(
        resolve_recipients(&state, Some(&["pm".to_string()])),
        vec!["sess-2".to_string()],
        "a scoped message resolves the display-name to its session-id"
    );

    assert!(
        resolve_recipients(&state, Some(&["ghost".to_string()])).is_empty(),
        "a scoped message to an unknown display routes to NO instance — no mis-route/leak"
    );
}

#[test]
fn display_for_terminal_reverse_resolves_the_agent_bound_to_a_terminal() {
    let state = registered(&RoutingState::default(), "sess-dev", "dev");
    let state = linked(&state, "sess-dev", "term-1");
    assert_eq!(
        display_for_terminal(&state, "term-1"),
        Some("dev".to_string()),
        "the reverse lookup finds the DISPLAY bound to a terminal"
    );
    assert_eq!(
        display_for_terminal(&state, "term-unbound"),
        None,
        "a terminal bound to no session resolves to no agent"
    );
}

#[test]
fn reap_removes_unclaimed_duplicates_of_a_live_display_but_never_an_offline_other_display() {
    let state = registered(&RoutingState::default(), "dev-live", "dev");
    let state = linked(&state, "dev-live", "term-dev");
    let state = registered(&state, "dev-stale-a", "dev");
    let state = cursor_advanced(&state, "dev-stale-a", "2026-06-13T00:00:00Z");
    let state = registered(&state, "dev-stale-b", "dev");
    let state = registered(&state, "pm-offline", "pm");
    let state = cursor_advanced(&state, "pm-offline", "2026-06-13T00:00:00Z");

    let reaped = apply(
        &state,
        RoutingAction::Reaped {
            display: "dev".to_string(),
        },
    );

    assert_eq!(
        resolve(&reaped, "dev"),
        vec!["dev-live".to_string()],
        "the LIVE (terminal-bound) dev is preserved — only the one true dev remains"
    );
    assert!(
        cursor(&reaped, "dev-stale-a").is_none(),
        "a reaped id's durable cursor is removed too (it was dead history)"
    );
    assert_eq!(
        resolve(&reaped, "pm"),
        vec!["pm-offline".to_string()],
        "an OFFLINE agent of a DIFFERENT display is untouched"
    );
    assert_eq!(
        cursor(&reaped, "pm-offline"),
        Some("2026-06-13T00:00:00Z".to_string()),
        "the offline agent's durable cursor survives"
    );
}

#[test]
fn cursor_advance_is_monotonic_max_a_later_cursor_applies_an_earlier_one_is_a_no_op() {
    let state = registered(&RoutingState::default(), "sess-1", "dev");
    let state = cursor_advanced(&state, "sess-1", "2026-06-11T20:00:05Z");
    assert_eq!(
        cursor(&state, "sess-1"),
        Some("2026-06-11T20:00:05Z".to_string())
    );

    let earlier = RoutingAction::CursorAdvanced {
        session_id: "sess-1".to_string(),
        cursor: "2026-06-11T20:00:01Z".to_string(),
    };
    assert!(
        reduce(&state, &earlier).is_none(),
        "an earlier cursor is not later than the stored one — the reducer applies the monotonic-max guard and reports no change"
    );
    assert_eq!(
        cursor(
            &cursor_advanced(&state, "sess-1", "2026-06-11T20:00:01Z"),
            "sess-1"
        ),
        Some("2026-06-11T20:00:05Z".to_string()),
        "the stored cursor never moves backward — forward-only monotonicity lives in the reducer"
    );

    let advanced = cursor_advanced(&state, "sess-1", "2026-06-11T20:00:09Z");
    assert_eq!(
        cursor(&advanced, "sess-1"),
        Some("2026-06-11T20:00:09Z".to_string()),
        "a later cursor advances — the monotonic-max resolves to the highest"
    );
}

#[test]
fn cursor_advance_parses_rather_than_lexically_compares_so_subsecond_precision_wins() {
    let state = registered(&RoutingState::default(), "sess-1", "dev");
    let state = cursor_advanced(&state, "sess-1", "2026-06-11T20:00:01Z");

    let advanced = cursor_advanced(&state, "sess-1", "2026-06-11T20:00:01.5Z");
    assert_eq!(
        cursor(&advanced, "sess-1"),
        Some("2026-06-11T20:00:01.5Z".to_string()),
        "the later instant wins by PARSED time — a lexical compare would wrongly keep the no-fraction form and lose half a second of events"
    );
}

#[test]
fn delivery_targets_routes_by_id_excluding_the_sender_and_offline_ids() {
    let state = registered(&RoutingState::default(), "sess-dev", "dev");
    let state = linked(&state, "sess-dev", "term-dev");
    let state = registered(&state, "sess-pm", "pm");
    let state = linked(&state, "sess-pm", "term-pm");
    let state = registered(&state, "sess-offline", "designer");

    let terminals = |targets: Vec<(String, String)>| {
        let mut t: Vec<String> = targets.into_iter().map(|(_, terminal)| terminal).collect();
        t.sort();
        t
    };

    let broadcast = terminals(delivery_targets(&state, None, "owner"));
    assert_eq!(
        broadcast,
        vec!["term-dev".to_string(), "term-pm".to_string()],
        "a broadcast routes by id to every LIVE-linked terminal; an offline id (registered, no terminal) is excluded"
    );

    assert_eq!(
        terminals(delivery_targets(
            &state,
            Some(&["all".to_string()]),
            "owner"
        )),
        broadcast,
        "an explicit BROADCAST_ALL to-list routes identically to to=None"
    );

    assert_eq!(
        delivery_targets(&state, Some(&["dev".to_string()]), "pm"),
        vec![("sess-dev".to_string(), "term-dev".to_string())],
        "a scoped to:[dev] resolves dev's DISPLAY to its id to its terminal and reaches ONLY it"
    );

    assert_eq!(
        delivery_targets(&state, None, "dev:claude-code"),
        vec![("sess-pm".to_string(), "term-pm".to_string())],
        "the SENDER's own terminal is excluded (no self-echo) and the sender is matched by short-name across a full identity"
    );
}

#[test]
fn delivery_targets_excludes_the_senders_own_terminal_even_under_the_unresolved_bridge_name() {
    use crate::routing::bridge_agent_name;

    let dev_terminal = "ccb4298d-1111-2222-3333-444455556666";
    let state = registered(&RoutingState::default(), "sess-dev", "dev");
    let state = linked(&state, "sess-dev", dev_terminal);
    let state = registered(&state, "sess-pm", "pm");
    let state = linked(&state, "sess-pm", "term-pm");

    let sender = bridge_agent_name(dev_terminal);
    let targets = delivery_targets(&state, None, &sender);

    assert!(
        targets.iter().any(|(_, t)| t == "term-pm"),
        "non-vacuous: a broadcast still reaches a DIFFERENT agent's terminal; got {targets:?}"
    );
    assert!(
        targets.iter().all(|(_, t)| t != dev_terminal),
        "a broadcast sent under the bridge name agent-{{terminal[..8]}} must NOT route back into the sender's OWN terminal; got {targets:?}"
    );
}
