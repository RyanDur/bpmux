use super::register_agent_actions;
use crate::routing::slice::{RoutingAction, RoutingState, cursor, reduce, resolve, terminal_for};

fn apply(state: &RoutingState, action: RoutingAction) -> RoutingState {
    reduce(state, &action).unwrap_or_else(|| state.clone())
}

fn register_agent(
    state: &RoutingState,
    session_id: &str,
    display: &str,
    terminal_id: &str,
    job_start: &str,
) -> RoutingState {
    register_agent_actions(state, session_id, display, terminal_id, job_start)
        .into_iter()
        .fold(state.clone(), |acc, action| apply(&acc, action))
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

fn registered(state: &RoutingState, session_id: &str, display: &str) -> RoutingState {
    apply(
        state,
        RoutingAction::Registered {
            session_id: session_id.to_string(),
            display: display.to_string(),
        },
    )
}

#[test]
fn a_fresh_agent_floors_the_cursor_at_job_start_so_gate_window_messages_drain() {
    let state = register_agent(
        &RoutingState::default(),
        "sess-fresh",
        "dev",
        "term-A",
        "2026-06-12T10:00:00Z",
    );
    assert_eq!(
        cursor(&state, "sess-fresh").as_deref(),
        Some("2026-06-12T10:00:00Z"),
        "a FRESH agent's cursor floors at the agent-JOB-START — not empty (would flood all history) nor the register-instant (would lose the gate-window)"
    );
    assert_eq!(
        resolve(&state, "dev"),
        vec!["sess-fresh".to_string()],
        "the fresh agent registers display=role so it is routable"
    );
    assert_eq!(
        terminal_for(&state, "sess-fresh").as_deref(),
        Some("term-A"),
        "the fresh agent links its terminal in the same composite"
    );
}

#[test]
fn a_future_cursor_resets_to_job_start_so_a_poisoned_floor_self_heals_on_rebind() {
    let state = registered(&RoutingState::default(), "sess-designer", "designer");
    let state = cursor_advanced(&state, "sess-designer", "2099-01-01T00:00:00Z");
    assert_eq!(
        cursor(&state, "sess-designer").as_deref(),
        Some("2099-01-01T00:00:00Z"),
        "precondition: a poisoned far-future delivery cursor (the 2099 incident that muted the designer)"
    );

    let healed = register_agent(
        &state,
        "sess-designer",
        "designer",
        "term-designer",
        "2026-06-13T12:00:00Z",
    );

    assert_eq!(
        cursor(&healed, "sess-designer").as_deref(),
        Some("2026-06-13T12:00:00Z"),
        "register_agent anchors the cursor BACKWARD to job-start — the CursorAnchored fact resets a poisoned far-future floor, distinct from the monotonic CursorAdvanced which would keep 2099"
    );
}

#[test]
fn a_past_cursor_is_preserved_on_reconnect_keeping_delivery_forward_only() {
    let state = register_agent(
        &RoutingState::default(),
        "sess-dev",
        "dev",
        "term-a",
        "2026-06-13T10:00:00Z",
    );
    let state = cursor_advanced(&state, "sess-dev", "2026-06-13T10:05:00Z");

    let reconnected = register_agent(&state, "sess-dev", "dev", "term-b", "2026-06-13T11:00:00Z");

    assert_eq!(
        cursor(&reconnected, "sess-dev").as_deref(),
        Some("2026-06-13T10:05:00Z"),
        "a reconnecting agent whose cursor is PAST the new job-start keeps it — register_agent does not anchor forward, so a normal reconnect never re-drains ambient history"
    );
    assert_eq!(
        terminal_for(&reconnected, "sess-dev").as_deref(),
        Some("term-b"),
        "the reconnect re-links the new live terminal"
    );
}

#[test]
fn a_reclaim_preserves_a_renamed_display_not_clobbered_by_the_default_name() {
    let state = register_agent(
        &RoutingState::default(),
        "sess-1",
        "bpmux-dev",
        "term-A",
        "2026-06-12T10:00:00Z",
    );
    let state = apply(
        &state,
        RoutingAction::Renamed {
            session_id: "sess-1".to_string(),
            display: "dev".to_string(),
        },
    );

    let reclaimed = register_agent(
        &state,
        "sess-1",
        "bpmux-dev",
        "term-B",
        "2026-06-12T11:00:00Z",
    );

    assert_eq!(
        resolve(&reclaimed, "dev"),
        vec!["sess-1".to_string()],
        "a returning agent keeps the owner-forged name (dev) — re-detection re-supplies the folder-default (bpmux-dev) but must NOT clobber the rename back (no Registered fact fires on a non-fresh reclaim)"
    );
    assert!(
        resolve(&reclaimed, "bpmux-dev").is_empty(),
        "the folder-default name no longer resolves — the rename held across the reclaim"
    );
}

#[test]
fn a_reclaim_preserves_the_durable_cursor_not_reset_to_the_new_job_start() {
    let state = register_agent(
        &RoutingState::default(),
        "sess-ret",
        "dev",
        "term-A",
        "2026-06-12T09:00:00Z",
    );
    let state = cursor_advanced(&state, "sess-ret", "2026-06-12T12:00:00Z");

    let reclaimed = register_agent(&state, "sess-ret", "dev", "term-B", "2026-06-12T15:00:00Z");

    assert_eq!(
        cursor(&reclaimed, "sess-ret").as_deref(),
        Some("2026-06-12T12:00:00Z"),
        "a RETURNING agent preserves its durable drain-position (noon) — re-detecting it at a NEW job-start (15:00) must NOT reset the cursor, which would skip the offline 12:00-15:00 messages"
    );
}

#[test]
fn claiming_a_live_terminal_reaps_the_unclaimed_duplicate_of_the_display() {
    let state = registered(&RoutingState::default(), "dev-stale", "dev");
    let live = register_agent(
        &state,
        "dev-live",
        "dev",
        "term-live",
        "2026-06-13T10:00:00Z",
    );

    assert_eq!(
        resolve(&live, "dev"),
        vec!["dev-live".to_string()],
        "register_agent reaps the unclaimed stale 'dev' when the live 'dev' binds its terminal — only the one live id remains routable"
    );
}
