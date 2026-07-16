#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{CiBroadcastListener, CiSnapshot, CiStateStoreWriter};
use crate::{CiState, CiStateStore};

fn quota_exhausted_snapshot() -> CiSnapshot {
    CiSnapshot {
        state: CiState::QuotaExhausted,
        run_id: Some(42),
        head_sha: Some("abc123".into()),
        display_title: Some("nightly".into()),
        conclusion: "quota_exhausted",
        workflows: vec![],
    }
}

fn green_snapshot() -> CiSnapshot {
    CiSnapshot {
        state: CiState::Green,
        run_id: Some(7),
        head_sha: Some("def456".into()),
        display_title: Some("build".into()),
        conclusion: "success",
        workflows: vec![],
    }
}

#[test]
fn quota_exhausted_snapshot_is_stored_as_quota_exhausted_not_failure() {
    let store = CiStateStore::new();
    let writer = CiStateStoreWriter::new(store.clone());
    let snap = quota_exhausted_snapshot();

    writer.on_changed(&snap);

    let report = store.read().expect("writer stored a report");
    assert_eq!(report.conclusion, "quota_exhausted");
    assert_eq!(report.head_sha, snap.head_sha);
    assert_eq!(report.last_run_id, snap.run_id);
}

#[test]
fn green_snapshot_still_round_trips_as_success() {
    let store = CiStateStore::new();
    let writer = CiStateStoreWriter::new(store.clone());
    let snap = green_snapshot();

    writer.on_changed(&snap);

    let report = store.read().expect("writer stored a report");
    assert_eq!(report.conclusion, "success");
    assert_eq!(report.head_sha, snap.head_sha);
    assert_eq!(report.last_run_id, snap.run_id);
}
