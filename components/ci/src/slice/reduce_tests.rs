#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{
    CiAction, CiSnapshot, CiState, last_successful_poll_at, last_updated_at, polling, reduce,
    snapshot,
};

fn snap(run_id: u64, conclusion: &'static str) -> CiSnapshot {
    CiSnapshot {
        state: crate::CiState::Green,
        run_id: Some(run_id),
        head_sha: None,
        display_title: None,
        conclusion,
        workflows: Vec::new(),
    }
}

#[test]
fn polling_started_records_the_actual_spawn() {
    let state =
        reduce(&CiState::default(), &CiAction::PollingStarted).expect("the fact changes polling");
    assert!(polling(&state), "a reported spawn flips the flag true");
}

#[test]
fn polling_stopped_records_the_actual_cancel() {
    let state =
        reduce(&CiState::default(), &CiAction::PollingStarted).expect("the fact changes polling");
    let state = reduce(&state, &CiAction::PollingStopped).expect("the fact changes polling");
    assert!(!polling(&state), "a reported cancel flips the flag false");
}

#[test]
fn polling_facts_preserve_the_snapshot() {
    let seeded = reduce(&CiState::default(), &CiAction::Updated(snap(42, "success")))
        .expect("a first Updated lands the snapshot");
    let state = reduce(&seeded, &CiAction::PollingStarted).expect("the fact changes polling");
    assert_eq!(
        snapshot(&state)
            .expect("the snapshot survives the fact")
            .run_id,
        Some(42),
        "a liveness fact folds its flag without touching the stored snapshot"
    );
}

#[test]
fn an_updated_preserves_the_polling_flag() {
    let state =
        reduce(&CiState::default(), &CiAction::PollingStarted).expect("the fact changes polling");
    let state = reduce(&state, &CiAction::Updated(snap(42, "success")))
        .expect("a new key lands the snapshot");
    assert!(
        polling(&state),
        "a snapshot update folds its fields without touching the liveness flag"
    );
}

#[test]
fn start_polling_command_is_pure_intent_not_actuality() {
    assert!(
        reduce(&CiState::default(), &CiAction::StartPolling).is_none(),
        "the command alone reports no state change until the effector reports a fact"
    );
}

#[test]
fn stop_polling_command_is_pure_intent_not_actuality() {
    assert!(
        reduce(&CiState::default(), &CiAction::StopPolling).is_none(),
        "the command alone reports no state change until the effector reports a fact"
    );
}

#[test]
fn poll_succeeded_always_records_the_liveness_timestamp() {
    let state = reduce(
        &CiState::default(),
        &CiAction::PollSucceeded {
            at: "2026-07-16T00:00:00Z".to_string(),
            updated: false,
        },
    )
    .expect("a successful poll changes state");
    assert_eq!(
        last_successful_poll_at(&state),
        Some("2026-07-16T00:00:00Z".to_string()),
        "every successful poll, regardless of verdict change, records liveness"
    );
    assert_eq!(
        last_updated_at(&state),
        None,
        "an unchanged verdict leaves last_updated_at untouched"
    );
}

#[test]
fn poll_succeeded_records_the_verdict_timestamp_only_when_updated() {
    let state = reduce(
        &CiState::default(),
        &CiAction::PollSucceeded {
            at: "2026-07-16T00:00:00Z".to_string(),
            updated: true,
        },
    )
    .expect("a successful poll changes state");
    assert_eq!(
        last_updated_at(&state),
        Some("2026-07-16T00:00:00Z".to_string()),
        "a changed verdict records the update timestamp"
    );

    let unchanged = reduce(
        &state,
        &CiAction::PollSucceeded {
            at: "2026-07-16T00:00:10Z".to_string(),
            updated: false,
        },
    )
    .expect("a successful poll changes state");
    assert_eq!(
        last_successful_poll_at(&unchanged),
        Some("2026-07-16T00:00:10Z".to_string()),
        "liveness advances on every poll"
    );
    assert_eq!(
        last_updated_at(&unchanged),
        Some("2026-07-16T00:00:00Z".to_string()),
        "the prior verdict timestamp survives a poll with no change"
    );
}
