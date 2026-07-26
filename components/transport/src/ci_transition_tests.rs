use super::ci_transition::{Observation, Transition, classify_transition};

fn obs(run_id: u64, conclusion: &'static str) -> Observation {
    (Some(run_id), conclusion)
}

#[test]
fn first_observation_is_a_change() {
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
    let t = classify_transition(Some(obs(42, "success")), obs(42, "success"));
    assert_eq!(t, Transition::Same);
}

#[test]
fn in_progress_to_success_same_run_is_a_change() {
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
    let t = classify_transition(Some(obs(42, "unknown")), obs(42, "success"));
    assert_eq!(
        t,
        Transition::Changed {
            new_conclusion: "success"
        }
    );
}

#[test]
fn new_run_same_conclusion_is_a_change_not_same() {
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
    let t = classify_transition(Some((None, "success")), (None, "success"));
    assert_eq!(t, Transition::Same);
}
