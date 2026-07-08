#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use serde_json::{Map, Value, json};

use super::{CommitmentsAction, CommitmentsState, matching_roles, reduce};

fn commit(
    state: &CommitmentsState,
    role: &str,
    event_class: &str,
    filter: Option<Map<String, Value>>,
) -> CommitmentsState {
    reduce(
        state,
        &CommitmentsAction::Committed {
            role: role.to_string(),
            event_class: event_class.to_string(),
            filter,
        },
    )
    .unwrap()
}

fn filter(pairs: &[(&str, Value)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn a_committed_role_wakes_once_on_its_event_then_the_commitment_is_consumed() {
    let armed = commit(
        &CommitmentsState::default(),
        "dev",
        "ci_state_changed",
        None,
    );
    let payload = json!({ "conclusion": "green" });

    assert_eq!(
        matching_roles(&armed, "ci_state_changed", &payload),
        vec!["dev".to_string()],
        "the committed role is woken when its event fires"
    );

    let consumed = reduce(
        &armed,
        &CommitmentsAction::CommitmentsMatched {
            roles: vec!["dev".to_string()],
            event_class: "ci_state_changed".to_string(),
            payload: payload.clone(),
        },
    )
    .unwrap();
    assert!(
        matching_roles(&consumed, "ci_state_changed", &payload).is_empty(),
        "one-shot: the commitment is a single obligation — once it fires it is consumed, so a second event does NOT re-wake the role"
    );
}

#[test]
fn a_filter_gates_which_payloads_wake_and_a_non_match_leaves_the_commitment_armed() {
    let armed = commit(
        &CommitmentsState::default(),
        "dev",
        "ci_state_changed",
        Some(filter(&[("conclusion", json!("green"))])),
    );

    assert!(
        matching_roles(&armed, "ci_state_changed", &json!({ "conclusion": "red" })).is_empty(),
        "a filtered commitment does NOT fire on a non-matching payload"
    );
    assert!(
        reduce(
            &armed,
            &CommitmentsAction::CommitmentsMatched {
                roles: vec!["dev".to_string()],
                event_class: "ci_state_changed".to_string(),
                payload: json!({ "conclusion": "red" }),
            },
        )
        .is_none(),
        "the non-matching event removes nothing — the reducer reports no change and the commitment stays armed"
    );
    assert_eq!(
        matching_roles(
            &armed,
            "ci_state_changed",
            &json!({ "conclusion": "green" })
        ),
        vec!["dev".to_string()],
        "the commitment stays armed through the non-match and fires on the matching payload"
    );
}

#[test]
fn a_different_event_class_wakes_no_committed_role() {
    let armed = commit(
        &CommitmentsState::default(),
        "dev",
        "ci_state_changed",
        None,
    );

    assert!(
        matching_roles(
            &armed,
            "story_state_transition",
            &json!({ "story_id": 1753 })
        )
        .is_empty(),
        "a commitment is scoped to its event-class — an unrelated event fires no wake"
    );
}

#[test]
fn the_removal_is_scoped_to_the_woken_set_so_a_concurrent_commit_is_not_silently_consumed() {
    let payload = json!({ "conclusion": "green" });
    let read = commit(&CommitmentsState::default(), "A", "ci_state_changed", None);
    let woken = matching_roles(&read, "ci_state_changed", &payload);

    let raced = commit(&read, "B", "ci_state_changed", None);
    let consumed = reduce(
        &raced,
        &CommitmentsAction::CommitmentsMatched {
            roles: woken,
            event_class: "ci_state_changed".to_string(),
            payload: payload.clone(),
        },
    )
    .unwrap();

    assert_eq!(
        matching_roles(&consumed, "ci_state_changed", &payload),
        vec!["B".to_string()],
        "B committed in the read-to-removal window is NOT in the woken set, so its commitment is retained and B still wakes — only A, the role actually read as woken, is consumed"
    );
}
