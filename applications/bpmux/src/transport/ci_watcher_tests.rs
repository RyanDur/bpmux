//! Pure-classification tests for the CI watcher (#372, #395). The
//! full-loop integration test belongs in Playwright (AC6) — here we
//! cover the transition-detection shape that decides when to
//! broadcast vs stay silent. Separating the pure logic from the
//! tokio task keeps the test fast + deterministic.
//!
//! #395 widened the dedup key from conclusion-alone to
//! `(run_id, conclusion)` — so repeated reds across successive
//! pushes each produce a broadcast instead of only the first.

use super::ci_watcher::{Observation, Transition, classify_transition};

fn obs(run_id: u64, conclusion: &'static str) -> Observation {
    (Some(run_id), conclusion)
}

#[test]
fn first_observation_is_a_change() {
    // None → any observation means we haven't observed anything yet;
    // the first tick should count as a transition so the renderer
    // syncs immediately rather than waiting for a later change.
    let t = classify_transition(None, obs(1, "success"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn same_run_and_same_conclusion_is_no_broadcast() {
    // Same (run_id, conclusion) = same run already broadcast; the
    // tick was duplicated (e.g., back-to-back polls before GitHub
    // transitions), no re-broadcast warranted.
    let t = classify_transition(Some(obs(42, "success")), obs(42, "success"));
    assert_eq!(t, Transition::Same);
}

#[test]
fn in_progress_to_success_same_run_is_a_change() {
    // Same run_id, conclusion flips from in_progress → success as
    // the run completes. Broadcast so the renderer updates.
    let t = classify_transition(Some(obs(42, "in_progress")), obs(42, "success"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn success_to_failure_different_run_is_a_change() {
    // New run_id + conclusion flip — unambiguous broadcast.
    let t = classify_transition(Some(obs(42, "success")), obs(43, "failure"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "failure"
        }
    );
}

#[test]
fn failure_to_in_progress_different_run_is_a_change() {
    // A push after a red main starts a new run → in_progress.
    let t = classify_transition(Some(obs(42, "failure")), obs(43, "in_progress"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "in_progress"
        }
    );
}

#[test]
fn unknown_to_any_real_state_same_run_is_a_change() {
    // Startup often sees unknown briefly before the first successful
    // fetch classifies a concrete state — the transition should still
    // fire so the renderer repaints.
    let t = classify_transition(Some(obs(42, "unknown")), obs(42, "success"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

// ── #395 falsifier trio ────────────────────────────────────────────

#[test]
fn new_run_same_conclusion_is_a_change_not_same() {
    // #395 core case: two consecutive red pushes with distinct
    // run_ids. Pre-#395 this collapsed into `Same` and swallowed the
    // second broadcast. Now each new run_id triggers `Changed`
    // regardless of whether its conclusion matches the prior run.
    let t = classify_transition(Some(obs(42, "failure")), obs(43, "failure"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "failure"
        },
        "new run_id with same conclusion must broadcast — pre-#395 \
         bug was that successive reds silently fell through"
    );
}

#[test]
fn same_run_same_conclusion_dedups() {
    // #395 falsifier guard: re-observing the exact same run (e.g.,
    // a duplicate poll response, or a re-fetch within the same
    // tick) must NOT fire a second broadcast. Dedup is by tuple,
    // not by polled-at-least-once.
    let t = classify_transition(Some(obs(99, "failure")), obs(99, "failure"));
    assert_eq!(
        t,
        Transition::Same,
        "identical (run_id, conclusion) must dedup — otherwise \
         every tick re-emits and the channel fills with noise"
    );
}

#[test]
fn conclusion_flip_same_run_preserves_existing_broadcast_behavior() {
    // AC5: "conclusion-flip → Changed (existing behavior
    // preserved)." Covers the in-flight re-classification case:
    // a run transitions from in_progress to its final conclusion
    // under a single run_id.
    let t = classify_transition(Some(obs(7, "in_progress")), obs(7, "failure"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "failure"
        }
    );
}

#[test]
fn run_id_none_degrades_to_conclusion_only_dedup() {
    // Degenerate case: GitHub response omits database_id. Dedup
    // key is (None, conclusion) on both sides — if conclusion
    // matches, it's still Same. Documents behavior rather than
    // promising a specific policy on id-less runs.
    let t = classify_transition(Some((None, "success")), (None, "success"));
    assert_eq!(t, Transition::Same);
}
