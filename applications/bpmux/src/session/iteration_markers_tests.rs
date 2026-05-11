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
        ..IterationMarker::default()
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

// ── #1283: pure-function tests on `restore_missing_markers_to_icebox_unified` ──

#[test]
fn restore_appends_missing_markers_in_position_order() {
    use crate::session::iteration_markers::restore_missing_markers_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    let store = IterationMarkersStore {
        markers: vec![
            marker(2, "Aquila", 2),
            marker(5, "Aquila", 3),
            marker(6, "Pegasus", 4),
        ],
        next_id: 7,
    };
    let mut icebox_unified: Vec<IceboxItem> = Vec::new();
    let restored = restore_missing_markers_to_icebox_unified(&store, &mut icebox_unified);
    assert_eq!(restored, 3);
    assert_eq!(
        icebox_unified,
        vec![
            IceboxItem::Marker { id: 2 },
            IceboxItem::Marker { id: 5 },
            IceboxItem::Marker { id: 6 },
        ],
        "restored in marker-position order (2 → 3 → 4)"
    );
}

#[test]
fn restore_is_noop_when_all_markers_present() {
    use crate::session::iteration_markers::restore_missing_markers_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(2, "Aquila", 1)],
        next_id: 3,
    };
    let mut icebox_unified = vec![IceboxItem::Marker { id: 1 }, IceboxItem::Marker { id: 2 }];
    let before = icebox_unified.clone();
    let restored = restore_missing_markers_to_icebox_unified(&store, &mut icebox_unified);
    assert_eq!(restored, 0);
    assert_eq!(icebox_unified, before);
}

#[test]
fn restore_preserves_existing_stories_and_appends_missing_marker() {
    use crate::session::iteration_markers::restore_missing_markers_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(2, "Aquila", 1)],
        next_id: 3,
    };
    let mut icebox_unified = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Marker { id: 1 },
        IceboxItem::Story { number: 200 },
    ];
    let restored = restore_missing_markers_to_icebox_unified(&store, &mut icebox_unified);
    assert_eq!(restored, 1, "marker 2 restored; marker 1 already present");
    assert_eq!(
        icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Marker { id: 1 },
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker { id: 2 },
        ],
        "existing stories preserved; missing marker appended"
    );
}

#[test]
fn restore_noop_when_store_empty() {
    use crate::session::iteration_markers::restore_missing_markers_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    let store = IterationMarkersStore::default();
    let mut icebox_unified = vec![IceboxItem::Story { number: 100 }];
    let before = icebox_unified.clone();
    let restored = restore_missing_markers_to_icebox_unified(&store, &mut icebox_unified);
    assert_eq!(restored, 0);
    assert_eq!(icebox_unified, before);
}

#[test]
fn restore_idempotent_across_multiple_invocations() {
    use crate::session::iteration_markers::restore_missing_markers_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0)],
        next_id: 2,
    };
    let mut icebox_unified: Vec<IceboxItem> = Vec::new();
    let first = restore_missing_markers_to_icebox_unified(&store, &mut icebox_unified);
    let second = restore_missing_markers_to_icebox_unified(&store, &mut icebox_unified);
    let third = restore_missing_markers_to_icebox_unified(&store, &mut icebox_unified);
    assert_eq!(first, 1, "first run restores the missing marker");
    assert_eq!(second, 0, "second run no-op");
    assert_eq!(third, 0, "third run still no-op (multi-restart-survives)");
    assert_eq!(icebox_unified, vec![IceboxItem::Marker { id: 1 }]);
}

// ── #1283: boot-integration tests via `restore_marker_positions_to_icebox_unified_at_path` ──

#[tokio::test]
async fn boot_restore_repairs_priority_json_when_icebox_unified_missing_markers() {
    let (session, _rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    // Empirical repro: priority.json has no icebox_unified field
    // (skip_serializing_if = Vec::is_empty omitted it), markers-store
    // has 3 markers. Ryan's UI showed no markers post-restart.
    std::fs::write(&priority_path, r#"{"icebox":[],"ready":[]}"#).unwrap();
    let store = IterationMarkersStore {
        markers: vec![
            marker(2, "Aquila", 2),
            marker(5, "Aquila", 3),
            marker(6, "Pegasus", 4),
        ],
        next_id: 7,
    };
    save(&session.dir, &store).unwrap();

    crate::restore_marker_positions_to_icebox_unified_at_path(
        &priority_path,
        &session.dir,
        session.clone(),
    )
    .await;

    let index = crate::session::priority::load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified.len(),
        3,
        "all 3 markers restored to icebox_unified"
    );
    let ids: Vec<u64> = index
        .icebox_unified
        .iter()
        .filter_map(|i| match i {
            crate::session::priority::IceboxItem::Marker { id } => Some(*id),
            crate::session::priority::IceboxItem::Story { .. } => None,
        })
        .collect();
    assert_eq!(ids, vec![2, 5, 6]);
}

#[tokio::test]
async fn boot_restore_noop_when_already_consistent() {
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
    let before_mtime = std::fs::metadata(&priority_path)
        .unwrap()
        .modified()
        .unwrap();

    crate::restore_marker_positions_to_icebox_unified_at_path(
        &priority_path,
        &session.dir,
        session.clone(),
    )
    .await;

    let after_mtime = std::fs::metadata(&priority_path)
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(
        before_mtime, after_mtime,
        "consistent state = no save = no mtime change"
    );
}

#[tokio::test]
async fn boot_restore_broadcasts_state_delta_on_restore() {
    let (session, mut rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    std::fs::write(&priority_path, r#"{}"#).unwrap();
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0)],
        next_id: 2,
    };
    save(&session.dir, &store).unwrap();

    crate::restore_marker_positions_to_icebox_unified_at_path(
        &priority_path,
        &session.dir,
        session.clone(),
    )
    .await;

    let msg = rx.try_recv().expect("state-delta broadcast received");
    assert_eq!(msg.message_type, crate::session::MessageType::StateDelta);
    match msg.state_delta {
        Some(crate::session::message::StateDeltaPayload::IceboxMigration { entries }) => {
            assert_eq!(entries, 1, "icebox_unified now has 1 marker");
        }
        other => panic!("expected IceboxMigration payload; got {other:?}"),
    }
}

#[tokio::test]
async fn boot_restore_then_orphan_heal_preserves_restored_markers() {
    // Ordering invariant: #1283 restore MUST run before #1271 evict.
    // Otherwise #1271 would evict the very markers #1283 restored
    // (markers-store has them; icebox_unified DIDN'T pre-restore, so
    // running #1271 first would treat them as orphans).
    let (session, _rx, _dir) = fresh_session();
    let priority_path = session.dir.join("priority.json");
    std::fs::write(&priority_path, r#"{}"#).unwrap();
    let store = IterationMarkersStore {
        markers: vec![marker(1, "Andromeda", 0), marker(2, "Aquila", 1)],
        next_id: 3,
    };
    save(&session.dir, &store).unwrap();

    // Boot sequence step 1: #1283 restore
    crate::restore_marker_positions_to_icebox_unified_at_path(
        &priority_path,
        &session.dir,
        session.clone(),
    )
    .await;
    // Boot sequence step 2: #1271 evict
    crate::auto_heal_markers_store_orphans_at_path(&priority_path, &session.dir, session.clone())
        .await;

    let after = load(&session.dir);
    assert_eq!(
        after.markers.len(),
        2,
        "both markers survive the boot sequence — restore-then-evict preserves them"
    );
    let index = crate::session::priority::load_priority(&priority_path);
    assert_eq!(
        index.icebox_unified.len(),
        2,
        "icebox_unified has both markers post-sequence"
    );
}

// ── #1288: pure-function tests on compute_fold_above_marker + sync_marker_fold_snapshots + restore_story_fold_positions_to_icebox_unified ──

#[test]
fn compute_fold_above_marker_returns_stories_immediately_preceding() {
    use crate::session::iteration_markers::compute_fold_above_marker;
    use crate::session::priority::IceboxItem;
    let icebox = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Story { number: 101 },
        IceboxItem::Story { number: 102 },
        IceboxItem::Marker { id: 5 },
        IceboxItem::Story { number: 200 },
    ];
    let fold = compute_fold_above_marker(&icebox, 5);
    assert_eq!(fold, vec![100, 101, 102], "all stories preceding marker 5");
    let fold_absent = compute_fold_above_marker(&icebox, 99);
    assert!(
        fold_absent.is_empty(),
        "marker not in icebox_unified returns empty fold"
    );
}

#[test]
fn compute_fold_above_marker_bounded_by_previous_marker() {
    use crate::session::iteration_markers::compute_fold_above_marker;
    use crate::session::priority::IceboxItem;
    // Marker M1 at index 1, Marker M2 at index 4; M2's fold is only
    // stories 101 + 102 (NOT story 100 which is before M1).
    let icebox = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Marker { id: 1 },
        IceboxItem::Story { number: 101 },
        IceboxItem::Story { number: 102 },
        IceboxItem::Marker { id: 2 },
    ];
    let fold = compute_fold_above_marker(&icebox, 2);
    assert_eq!(
        fold,
        vec![101, 102],
        "fold for M2 bounded above by M1; story 100 belongs to M1"
    );
    let fold_m1 = compute_fold_above_marker(&icebox, 1);
    assert_eq!(fold_m1, vec![100], "fold for M1 = stories before it");
}

#[test]
fn sync_marker_fold_snapshots_updates_every_marker() {
    use crate::session::iteration_markers::sync_marker_fold_snapshots;
    use crate::session::priority::IceboxItem;
    let mut store = IterationMarkersStore {
        markers: vec![marker(1, "M1", 0), marker(2, "M2", 1)],
        next_id: 3,
    };
    let icebox = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Marker { id: 1 },
        IceboxItem::Story { number: 200 },
        IceboxItem::Story { number: 201 },
        IceboxItem::Marker { id: 2 },
    ];
    sync_marker_fold_snapshots(&mut store, &icebox);
    let m1 = store.markers.iter().find(|m| m.id == 1).unwrap();
    let m2 = store.markers.iter().find(|m| m.id == 2).unwrap();
    assert_eq!(m1.stories, vec![100]);
    assert_eq!(m2.stories, vec![200, 201]);
}

#[test]
fn sync_clears_stories_for_markers_absent_from_icebox_unified() {
    use crate::session::iteration_markers::sync_marker_fold_snapshots;
    use crate::session::priority::IceboxItem;
    // Marker 1 in store but NOT in icebox_unified → its stories[]
    // should be cleared (live-state reflects positional truth).
    let mut store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 1,
            theme_name: "M1".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: vec![100, 101],
        }],
        next_id: 2,
    };
    let icebox: Vec<IceboxItem> = Vec::new();
    sync_marker_fold_snapshots(&mut store, &icebox);
    assert_eq!(
        store.markers[0].stories,
        Vec::<u64>::new(),
        "marker absent from icebox_unified ⇒ empty fold-snapshot"
    );
}

#[test]
fn restore_story_fold_positions_inserts_missing_stories_before_marker() {
    use crate::session::iteration_markers::restore_story_fold_positions_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    // Pre-restart snapshot: marker 5 had stories [100, 101, 102].
    // Post-restart icebox_unified has marker but no stories adjacent.
    let store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 5,
            theme_name: "Aquila".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: vec![100, 101, 102],
        }],
        next_id: 6,
    };
    let mut icebox = vec![IceboxItem::Marker { id: 5 }];
    let canonical: std::collections::HashSet<u64> = [100, 101, 102].into_iter().collect();
    let restored = restore_story_fold_positions_to_icebox_unified(&store, &mut icebox, &canonical);
    assert_eq!(restored, 3);
    assert_eq!(
        icebox,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 101 },
            IceboxItem::Story { number: 102 },
            IceboxItem::Marker { id: 5 },
        ],
        "stories restored in fold-order immediately before marker"
    );
}

#[test]
fn restore_story_fold_positions_skips_non_canonical_icebox_stories() {
    use crate::session::iteration_markers::restore_story_fold_positions_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    // Story 101 is in snapshot but no longer workflow_state=Icebox
    // (per refresh.all_stories() canonical filter). Don't reinsert.
    let store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 5,
            theme_name: "Aquila".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: vec![100, 101, 102],
        }],
        next_id: 6,
    };
    let mut icebox = vec![IceboxItem::Marker { id: 5 }];
    let canonical: std::collections::HashSet<u64> = [100, 102].into_iter().collect();
    let restored = restore_story_fold_positions_to_icebox_unified(&store, &mut icebox, &canonical);
    assert_eq!(restored, 2, "only 100 + 102 restored; 101 evicted upstream");
    assert!(
        icebox
            .iter()
            .all(|i| !matches!(i, IceboxItem::Story { number: 101 })),
        "story 101 not reinserted (not in canonical icebox set)"
    );
}

#[test]
fn restore_story_fold_positions_skips_marker_absent_from_icebox_unified() {
    use crate::session::iteration_markers::restore_story_fold_positions_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    // Marker 5 has fold-snapshot but isn't yet in icebox_unified;
    // can't restore stories without an anchor. #1283's marker-restore
    // runs first to add the marker; this fn then runs.
    let store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 5,
            theme_name: "Aquila".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: vec![100, 101],
        }],
        next_id: 6,
    };
    let mut icebox: Vec<IceboxItem> = Vec::new();
    let canonical: std::collections::HashSet<u64> = [100, 101].into_iter().collect();
    let restored = restore_story_fold_positions_to_icebox_unified(&store, &mut icebox, &canonical);
    assert_eq!(restored, 0, "no marker anchor ⇒ no insertion");
    assert!(icebox.is_empty());
}

// ── #1294: backfill_legacy_marker_stories ──

#[test]
fn backfill_populates_legacy_marker_with_adjacent_icebox_stories() {
    use crate::session::iteration_markers::backfill_legacy_marker_stories;
    use crate::session::priority::IceboxItem;
    let mut store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 5,
            theme_name: "Aquila".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: Vec::new(),
        }],
        next_id: 6,
    };
    let icebox = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Story { number: 101 },
        IceboxItem::Marker { id: 5 },
    ];
    let backfilled = backfill_legacy_marker_stories(&mut store, &icebox);
    assert_eq!(backfilled, 1);
    assert_eq!(store.markers[0].stories, vec![100, 101]);
}

#[test]
fn backfill_idempotent_skips_markers_with_populated_stories() {
    use crate::session::iteration_markers::backfill_legacy_marker_stories;
    use crate::session::priority::IceboxItem;
    let mut store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 5,
            theme_name: "Aquila".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: vec![200, 201],
        }],
        next_id: 6,
    };
    let icebox = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Marker { id: 5 },
    ];
    let backfilled = backfill_legacy_marker_stories(&mut store, &icebox);
    assert_eq!(backfilled, 0, "non-empty stories[] preserved");
    assert_eq!(
        store.markers[0].stories,
        vec![200, 201],
        "existing forward-heal'd values not overwritten by backfill"
    );
}

#[test]
fn backfill_no_op_when_marker_absent_from_icebox_unified() {
    use crate::session::iteration_markers::backfill_legacy_marker_stories;
    use crate::session::priority::IceboxItem;
    let mut store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 99,
            theme_name: "GhostMarker".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: Vec::new(),
        }],
        next_id: 100,
    };
    let icebox = vec![IceboxItem::Story { number: 100 }];
    let backfilled = backfill_legacy_marker_stories(&mut store, &icebox);
    assert_eq!(backfilled, 0);
    assert!(store.markers[0].stories.is_empty());
}

#[test]
fn backfill_today_three_legacy_markers_scenario() {
    // Today's exact shape: Aquila id=2, Aquila id=5, Pegasus id=6 — all
    // legacy (empty stories[]) — with adjacent stories in icebox_unified.
    use crate::session::iteration_markers::backfill_legacy_marker_stories;
    use crate::session::priority::IceboxItem;
    let mut store = IterationMarkersStore {
        markers: vec![
            IterationMarker {
                id: 2,
                theme_name: "Aquila".to_string(),
                position: 2,
                created_at: "ts".to_string(),
                stories: Vec::new(),
            },
            IterationMarker {
                id: 5,
                theme_name: "Aquila".to_string(),
                position: 3,
                created_at: "ts".to_string(),
                stories: Vec::new(),
            },
            IterationMarker {
                id: 6,
                theme_name: "Pegasus".to_string(),
                position: 4,
                created_at: "ts".to_string(),
                stories: Vec::new(),
            },
        ],
        next_id: 7,
    };
    let icebox = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Marker { id: 2 },
        IceboxItem::Story { number: 200 },
        IceboxItem::Story { number: 201 },
        IceboxItem::Marker { id: 5 },
        IceboxItem::Story { number: 300 },
        IceboxItem::Marker { id: 6 },
    ];
    let backfilled = backfill_legacy_marker_stories(&mut store, &icebox);
    assert_eq!(backfilled, 3);
    assert_eq!(store.markers[0].stories, vec![100]);
    assert_eq!(store.markers[1].stories, vec![200, 201]);
    assert_eq!(store.markers[2].stories, vec![300]);
}

#[test]
fn restore_story_fold_positions_idempotent_when_stories_already_present() {
    use crate::session::iteration_markers::restore_story_fold_positions_to_icebox_unified;
    use crate::session::priority::IceboxItem;
    let store = IterationMarkersStore {
        markers: vec![IterationMarker {
            id: 5,
            theme_name: "Aquila".to_string(),
            position: 0,
            created_at: "ts".to_string(),
            stories: vec![100, 101],
        }],
        next_id: 6,
    };
    let mut icebox = vec![
        IceboxItem::Story { number: 100 },
        IceboxItem::Story { number: 101 },
        IceboxItem::Marker { id: 5 },
    ];
    let canonical: std::collections::HashSet<u64> = [100, 101].into_iter().collect();
    let first = restore_story_fold_positions_to_icebox_unified(&store, &mut icebox, &canonical);
    let second = restore_story_fold_positions_to_icebox_unified(&store, &mut icebox, &canonical);
    assert_eq!(first, 0, "stories already present ⇒ no restore");
    assert_eq!(second, 0, "second run still no-op");
}
