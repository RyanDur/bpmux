#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use super::{SyncHealthAction, SyncHealthState, reduce};

fn at(secs: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 8, 0, 0, secs).unwrap()
}

#[test]
fn ingest_succeeded_sets_last_ingest_at_and_clears_last_error() {
    let state = SyncHealthState {
        last_error: Some("prior".to_string()),
        ..SyncHealthState::default()
    };
    let next = reduce(&state, &SyncHealthAction::IngestSucceeded { at: at(10) }).unwrap();
    assert_eq!(next.last_ingest_at, Some(at(10)));
    assert!(next.last_error.is_none());
}

#[test]
fn error_recorded_then_ingest_succeeded_clears_last_error() {
    let with_error = reduce(
        &SyncHealthState::default(),
        &SyncHealthAction::ErrorRecorded {
            msg: "ingest: down".to_string(),
        },
    )
    .unwrap();
    assert_eq!(with_error.last_error.as_deref(), Some("ingest: down"));
    let recovered = reduce(
        &with_error,
        &SyncHealthAction::IngestSucceeded { at: at(5) },
    )
    .unwrap();
    assert!(
        recovered.last_error.is_none(),
        "a successful ingest clears the error the shared field carried"
    );
}

#[test]
fn push_succeeded_sets_last_push_at_without_touching_ingest() {
    let next = reduce(
        &SyncHealthState::default(),
        &SyncHealthAction::PushSucceeded { at: at(20) },
    )
    .unwrap();
    assert_eq!(next.last_push_at, Some(at(20)));
    assert!(next.last_ingest_at.is_none());
}

#[test]
fn event_dropped_grows_dropped_events() {
    let next = reduce(
        &SyncHealthState::default(),
        &SyncHealthAction::EventDropped {
            story_id: 42,
            error: "permanent".to_string(),
            ts: at(30),
        },
    )
    .unwrap();
    assert_eq!(next.dropped_events.len(), 1);
    assert_eq!(next.dropped_events[0].story_id, 42);
}

#[test]
fn dropped_acknowledged_clears_dropped_events() {
    let dropped = reduce(
        &SyncHealthState::default(),
        &SyncHealthAction::EventDropped {
            story_id: 7,
            error: "permanent".to_string(),
            ts: at(40),
        },
    )
    .unwrap();
    let cleared = reduce(&dropped, &SyncHealthAction::DroppedAcknowledged).unwrap();
    assert!(cleared.dropped_events.is_empty());
}

#[test]
fn dropped_acknowledged_over_empty_is_a_noop() {
    assert!(
        reduce(
            &SyncHealthState::default(),
            &SyncHealthAction::DroppedAcknowledged
        )
        .is_none(),
        "acknowledging with nothing dropped changes no state"
    );
}
