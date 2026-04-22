//! Pure-classification tests for the CI watcher (#372). The
//! full-loop integration test belongs in Playwright (AC6) — here we
//! cover the transition-detection shape that decides when to
//! broadcast vs stay silent. Separating the pure logic from the
//! tokio task keeps the test fast + deterministic.

use super::ci_watcher::{Transition, classify_transition};

#[test]
fn first_observation_is_a_change() {
    // None → any conclusion means we haven't observed anything yet;
    // the first tick should count as a transition so the renderer
    // syncs immediately rather than waiting for a later change.
    let t = classify_transition(None, "success");
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn same_conclusion_as_prior_is_no_broadcast() {
    let t = classify_transition(Some("success"), "success");
    assert_eq!(t, Transition::Same);
}

#[test]
fn in_progress_to_success_is_a_change() {
    let t = classify_transition(Some("in_progress"), "success");
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn success_to_failure_is_a_change() {
    let t = classify_transition(Some("success"), "failure");
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "failure"
        }
    );
}

#[test]
fn failure_to_in_progress_is_a_change() {
    // A push after a red main starts a new run → in_progress.
    let t = classify_transition(Some("failure"), "in_progress");
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "in_progress"
        }
    );
}

#[test]
fn unknown_to_any_real_state_is_a_change() {
    // Startup often sees unknown briefly before the first successful
    // fetch classifies a concrete state — the transition should still
    // fire so the renderer repaints.
    let t = classify_transition(Some("unknown"), "success");
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}
