#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::commitment_store::CommitmentStore;
use serde_json::json;

fn filter(pairs: &[(&str, serde_json::Value)]) -> serde_json::Map<String, serde_json::Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn a_committed_role_wakes_once_on_its_event_then_the_commitment_is_consumed() {
    let store = CommitmentStore::new();
    store.commit("dev", "ci_state_changed", None);

    assert_eq!(
        store.take_matching("ci_state_changed", &json!({ "conclusion": "green" })),
        vec!["dev".to_string()],
        "the committed role is woken when its event fires"
    );
    assert!(
        store
            .take_matching("ci_state_changed", &json!({ "conclusion": "green" }))
            .is_empty(),
        "one-shot: the commitment is a single obligation — once it fires it is consumed, so a second event does NOT re-wake the role (otherwise every later CI event re-wakes a dormant agent that already acted)"
    );
}

#[test]
fn a_filter_gates_which_payloads_wake_and_a_non_match_leaves_the_commitment_armed() {
    let store = CommitmentStore::new();
    store.commit(
        "dev",
        "ci_state_changed",
        Some(filter(&[("conclusion", json!("green"))])),
    );

    assert!(
        store
            .take_matching("ci_state_changed", &json!({ "conclusion": "red" }))
            .is_empty(),
        "a filtered commitment does NOT fire on a non-matching payload — a green-gate is not woken by a red CI result"
    );
    assert_eq!(
        store.take_matching("ci_state_changed", &json!({ "conclusion": "green" })),
        vec!["dev".to_string()],
        "the commitment stays armed through the non-match and fires on the matching payload — the intervening in_progress/failure events do not burn it"
    );
}

#[test]
fn a_different_event_class_wakes_no_committed_role() {
    let store = CommitmentStore::new();
    store.commit("dev", "ci_state_changed", None);

    assert!(
        store
            .take_matching("story_state_transition", &json!({ "story_id": 1753 }))
            .is_empty(),
        "a commitment is scoped to its event-class — an unrelated event fires no wake"
    );
}
