#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::panic)]

// Tests for #1271: boot-time auto-heal of orphan markers in the
// iteration-markers store. Sister of #1268 `auto_heal_icebox_at_boot`
// at markers-store-axis. Truth-source = icebox_unified (canonical
// positional substrate from #1258 + #1268).

use crate::session::iteration_markers::{
    IterationMarker, IterationMarkersStore, auto_heal_orphans, load, save,
};
use std::collections::HashSet;

fn marker(id: u64, theme: &str, position: u32) -> IterationMarker {
    IterationMarker {
        id,
        theme_name: theme.to_string(),
        position,
        created_at: "2026-05-10T22:00:00Z".to_string(),
    }
}

// ── Pure-function tests on `auto_heal_orphans` ──

#[test]
fn evicts_orphan_marker_absent_from_canonical_set() {
    let mut store = IterationMarkersStore {
        markers: vec![
            marker(1, "Andromeda", 0),
            marker(2, "Aquila", 1),
            marker(3, "Pegasus", 2),
        ],
        next_id: 4,
    };
    // Canonical icebox_unified only references markers 1 and 3 —
    // marker 2 (Aquila) is orphaned.
    let canonical: HashSet<u64> = [1, 3].into_iter().collect();
    let evicted = auto_heal_orphans(&mut store, &canonical);
    assert_eq!(evicted, 1);
    assert_eq!(store.markers.len(), 2);
    let ids: Vec<u64> = store.markers.iter().map(|m| m.id).collect();
    assert_eq!(ids, vec![1, 3]);
}

#[test]
fn preserves_all_markers_when_all_in_canonical_set() {
    let mut store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(2, "Aquila", 1)],
        next_id: 3,
    };
    let canonical: HashSet<u64> = [1, 2].into_iter().collect();
    let evicted = auto_heal_orphans(&mut store, &canonical);
    assert_eq!(evicted, 0, "no orphans means no eviction");
    assert_eq!(store.markers.len(), 2);
}

#[test]
fn evicts_all_when_canonical_set_empty() {
    let mut store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(2, "Aquila", 1)],
        next_id: 3,
    };
    let canonical: HashSet<u64> = HashSet::new();
    let evicted = auto_heal_orphans(&mut store, &canonical);
    assert_eq!(evicted, 2);
    assert!(store.markers.is_empty());
}

#[test]
fn noop_when_store_empty() {
    let mut store = IterationMarkersStore::default();
    let canonical: HashSet<u64> = [1, 2, 3].into_iter().collect();
    let evicted = auto_heal_orphans(&mut store, &canonical);
    assert_eq!(evicted, 0);
}

#[test]
fn idempotent_after_first_heal() {
    let mut store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(99, "ORPHAN", 1)],
        next_id: 100,
    };
    let canonical: HashSet<u64> = [1].into_iter().collect();
    let first = auto_heal_orphans(&mut store, &canonical);
    let second = auto_heal_orphans(&mut store, &canonical);
    assert_eq!(first, 1);
    assert_eq!(second, 0, "re-run on healed state is a no-op");
    assert_eq!(store.markers.len(), 1);
}

// ── Boot-integration tests via `auto_heal_markers_store_orphans_at_path` ──

fn fresh_session() -> (
    std::sync::Arc<crate::session::Session>,
    tokio::sync::broadcast::Receiver<crate::session::Message>,
    tempfile::TempDir,
) {
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = tokio::sync::broadcast::channel::<crate::session::Message>(
        crate::session::BROADCAST_CHANNEL_CAPACITY,
    );
    let session = std::sync::Arc::new(crate::session::Session::new(
        dir.path().to_path_buf(),
        "test-dev".to_string(),
        tx,
    ));
    (session, rx, dir)
}

#[tokio::test]
async fn boot_heal_evicts_orphan_and_persists_store() {
    let (session, _rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    // icebox_unified references marker 1 only; markers-store has
    // markers 1 + 99 (99 is orphan).
    std::fs::write(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":1}]}"#,
    )
    .unwrap();
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(99, "ORPHAN", 1)],
        next_id: 100,
    };
    save(&session.dir, &store).unwrap();

    crate::auto_heal_markers_store_orphans_at_path(&priority_path, &session.dir, session.clone())
        .await;

    let after = load(&session.dir);
    assert_eq!(after.markers.len(), 1);
    assert_eq!(after.markers[0].id, 1);
    assert_eq!(after.markers[0].theme_name, "Andromeda");
}

#[tokio::test]
async fn boot_heal_noop_when_store_already_clean() {
    let (session, _rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    std::fs::write(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":1},{"kind":"marker","id":2}]}"#,
    )
    .unwrap();
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(2, "Aquila", 1)],
        next_id: 3,
    };
    save(&session.dir, &store).unwrap();
    let before_mtime = std::fs::metadata(
        crate::session::iteration_markers::iteration_markers_path(&session.dir),
    )
    .unwrap()
    .modified()
    .unwrap();

    crate::auto_heal_markers_store_orphans_at_path(&priority_path, &session.dir, session.clone())
        .await;

    let after_mtime = std::fs::metadata(crate::session::iteration_markers::iteration_markers_path(
        &session.dir,
    ))
    .unwrap()
    .modified()
    .unwrap();
    assert_eq!(
        before_mtime, after_mtime,
        "no orphans = no save = no mtime change"
    );
    let after = load(&session.dir);
    assert_eq!(after.markers.len(), 2);
}

#[tokio::test]
async fn boot_heal_broadcasts_state_delta_on_eviction() {
    let (session, mut rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    std::fs::write(
        &priority_path,
        r#"{"icebox_unified":[{"kind":"marker","id":1}]}"#,
    )
    .unwrap();
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(99, "ORPHAN", 1)],
        next_id: 100,
    };
    save(&session.dir, &store).unwrap();

    crate::auto_heal_markers_store_orphans_at_path(&priority_path, &session.dir, session.clone())
        .await;

    let msg = rx.try_recv().expect("state-delta broadcast received");
    assert_eq!(msg.message_type, crate::session::MessageType::StateDelta);
    match msg.state_delta {
        Some(crate::session::message::StateDeltaPayload::IceboxMigration { entries }) => {
            assert_eq!(entries, 1, "icebox_unified retained marker 1");
        }
        other => panic!("expected IceboxMigration payload; got {other:?}"),
    }
}

#[tokio::test]
async fn boot_heal_preserves_canonical_marker_even_when_others_orphan() {
    let (session, _rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    std::fs::write(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":100},
            {"kind":"marker","id":5}
        ]}"#,
    )
    .unwrap();
    let store = IterationMarkersStore {
        markers: vec![
            marker(1, "ORPHAN-A", 0),
            marker(5, "Cassiopeia", 1),
            marker(99, "ORPHAN-B", 2),
        ],
        next_id: 100,
    };
    save(&session.dir, &store).unwrap();

    crate::auto_heal_markers_store_orphans_at_path(&priority_path, &session.dir, session.clone())
        .await;

    let after = load(&session.dir);
    assert_eq!(after.markers.len(), 1);
    assert_eq!(after.markers[0].id, 5);
    assert_eq!(after.markers[0].theme_name, "Cassiopeia");
}

#[tokio::test]
async fn boot_heal_ignores_story_entries_in_icebox_unified() {
    // story entries don't contribute marker-ids to the canonical
    // set — only marker entries do. Ensure we don't accidentally
    // preserve markers whose id collides with a story number.
    let (session, _rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    std::fs::write(
        &priority_path,
        r#"{"icebox_unified":[
            {"kind":"story","number":7},
            {"kind":"marker","id":1}
        ]}"#,
    )
    .unwrap();
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Canonical", 0), marker(7, "ORPHAN-id-7", 1)],
        next_id: 8,
    };
    save(&session.dir, &store).unwrap();

    crate::auto_heal_markers_store_orphans_at_path(&priority_path, &session.dir, session.clone())
        .await;

    let after = load(&session.dir);
    assert_eq!(after.markers.len(), 1);
    assert_eq!(after.markers[0].id, 1);
}
