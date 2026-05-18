//! E2E for #1406 / #1385 Scenario 9 — concurrent-fire race-window
//! exercise. Sister-fix to `accept_story_substrate_enforce_e2e` which
//! ships the substrate-validator (`check_target_state_valid`) but
//! cannot exercise the race at e2e-axis because tokio's single-thread
//! executor serialized concurrent fires before either reached the
//! validator boundary.
//!
//! #1406 ships the `RaceWindowBarrier` rendezvous primitive (env
//! `BPMUX_TEST_RACE_WINDOW_REQUESTS=N` opt-in; production no-op) and
//! the InMemorySourceAdapter StateTransitioned handler that closes
//! adapter-axis state-tracking. Those two substrates are unit-tested
//! in `bridge::sync_orchestrator_tests` (race-window leader/follower
//! semantics) + `board::adapters::in_memory_tests` (StateTransitioned
//! mutates adapter story.state).
//!
//! The full e2e Sc 9 exercise stays gated on #1421 — the drain-axis
//! substrate-pivot. Empirical: with rendezvous + adapter-fix in
//! place, the follower still reads regressed state because
//! mutate_sync schedules a background `StartUpdateGithub` →
//! drain_one_by_id task that, on classify_conflict divergence,
//! triggers middleware_mutation_rolled_back →
//! refresh.apply_server_rollback → store.seed_all(server_story). The
//! seed_all races with the follower's BoardStore read and regresses
//! the leader's mutation. Closing that race-class is the #1421
//! axis-of-fix; once it lands, this test gets unignored.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
#[ignore = "#1406 ships rendezvous + adapter substrate; e2e Sc 9 exercise blocked on #1421 drain-classifier/middleware-rollback race vs follower-read"]
fn accept_story_concurrent_fire_returns_target_state_invalid_on_second() {
    panic!(
        "#1421 unblocker pending: with RaceWindowBarrier + adapter \
         StateTransitioned in place, the follower's post-rendezvous \
         BoardStore read can still race the middleware's drain-time \
         apply_server_rollback (server_story seed_all). When #1421 \
         lands, replace this panic with the two-fire tokio::join! \
         exercise asserting first→200 (accepted) + second→409 \
         workflow-target-state-invalid with current_state=accepted, \
         expected_state=delivered."
    );
}
