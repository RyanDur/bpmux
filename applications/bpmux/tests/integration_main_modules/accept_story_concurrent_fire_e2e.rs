//! E2E for #1406 / #1385 Scenario 9 — concurrent-fire race-window
//! exercise. Sister-fix to `accept_story_substrate_enforce_e2e` which
//! ships the substrate-validator (check_target_state_valid) but cannot
//! exercise the race at e2e-axis because InMemorySourceAdapter
//! serializes mutation→re-ingest (lock-then-broadcast).
//!
//! Scenario 9 (canonical from #1385): two concurrent accept_story
//! fires on the same delivered+evidence-met story → first returns 200
//! (story → accepted); second returns 409
//! workflow-target-state-invalid with current_state=accepted +
//! expected_state=delivered cite. Production single-orchestrator
//! serialization preserved; test-harness gets a controllable
//! race-window per #1406's substrate-fix shape.
//!
//! Initial state (pre-fix): test is filed-as-failing (TDD red).
//! #1406's substrate-fix will introduce the test-only race-window
//! affordance + make this go green.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

#[test]
#[ignore = "#1406 substrate-fix pending — InMemorySourceAdapter serializes the race-window"]
fn accept_story_concurrent_fire_returns_target_state_invalid_on_second() {
    panic!(
        "#1406 not yet shipped: test-harness race-window affordance \
         missing. When the substrate-fix lands, this test should fire \
         two concurrent accept_story calls on the same delivered+\
         evidence-met story and observe first→200 (accepted) + \
         second→409 workflow-target-state-invalid with \
         current_state=accepted, expected_state=delivered."
    );
}
