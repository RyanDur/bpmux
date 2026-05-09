#![allow(clippy::unwrap_used)]

use crate::session::log::{BROADCAST_CHANNEL_CAPACITY, Session};
use crate::session::storage::FileStorage;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::broadcast;

fn fresh_session() -> Session {
    let dir = tempdir().unwrap().keep();
    let storage: Arc<dyn crate::session::storage::Storage> =
        Arc::new(FileStorage::new(dir.clone()));
    let (wake_tx, _wake_rx) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);
    Session {
        dir,
        id: "test-session".to_string(),
        owner_name: "ryan".to_string(),
        wake_tx,
        connected_peers: std::sync::atomic::AtomicUsize::new(0),
        storage,
        roster: std::sync::Mutex::new(std::collections::HashSet::new()),
    }
}

#[test]
fn fresh_session_has_empty_roster() {
    let session = fresh_session();
    let snapshot = session.roster_snapshot();
    assert!(snapshot.is_empty());
}

#[test]
fn roster_join_populates_roster() {
    let session = fresh_session();
    session.roster_join("dev:claude-code");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot, vec!["dev:claude-code".to_string()]);
}

#[test]
fn roster_leave_removes_entry() {
    let session = fresh_session();
    session.roster_join("dev:claude-code");
    session.roster_join("pm:claude-code");
    session.roster_leave("dev:claude-code");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot, vec!["pm:claude-code".to_string()]);
}

#[test]
fn cloned_session_starts_with_empty_roster() {
    let original = fresh_session();
    original.roster_join("dev:claude-code");
    original.roster_join("pm:claude-code");
    let cloned = original.clone();
    let snapshot = cloned.roster_snapshot();
    assert!(
        snapshot.is_empty(),
        "Clone is a snapshot of the log, not live peers — roster must start fresh per Session::clone semantics"
    );
}

#[test]
fn roster_join_is_idempotent() {
    let session = fresh_session();
    session.roster_join("dev:claude-code");
    session.roster_join("dev:claude-code");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot.len(), 1);
}

#[test]
fn roster_snapshot_is_sorted() {
    let session = fresh_session();
    session.roster_join("zeta");
    session.roster_join("alpha");
    session.roster_join("mu");
    let snapshot = session.roster_snapshot();
    assert_eq!(snapshot, vec!["alpha", "mu", "zeta"]);
}
