use super::*;

#[test]
fn reorder_inserts_at_position() {
    let mut index = PriorityIndex::default();
    reorder(&mut index, "ready", 100, 1);
    reorder(&mut index, "ready", 200, 1);
    assert_eq!(index.columns["ready"], vec![200, 100]);
}

#[test]
fn reorder_moves_existing_story() {
    let mut index = PriorityIndex::default();
    reorder(&mut index, "ready", 100, 1);
    reorder(&mut index, "ready", 200, 2);
    reorder(&mut index, "ready", 300, 3);
    reorder(&mut index, "ready", 300, 1);
    assert_eq!(index.columns["ready"], vec![300, 100, 200]);
}

#[test]
fn reorder_clamps_position() {
    let mut index = PriorityIndex::default();
    reorder(&mut index, "ready", 100, 1);
    reorder(&mut index, "ready", 200, 99);
    assert_eq!(index.columns["ready"], vec![100, 200]);
}

#[test]
fn sort_by_priority_orders_correctly() {
    let numbers = vec![100, 200, 300, 400];
    let priority = vec![300, 100];
    let sorted = sort_by_priority(&numbers, &priority);
    assert_eq!(sorted, vec![300, 100, 200, 400]);
}

#[test]
fn sort_by_priority_handles_empty_priority() {
    let numbers = vec![100, 200, 300];
    let sorted = sort_by_priority(&numbers, &[]);
    assert_eq!(sorted, vec![100, 200, 300]);
}

#[test]
fn save_and_load_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    let mut index = PriorityIndex::default();
    reorder(&mut index, "ready", 100, 1);
    reorder(&mut index, "ready", 200, 2);
    save_priority(&path, &index).unwrap();
    let loaded = load_priority(&path);
    assert_eq!(loaded.columns["ready"], vec![100, 200]);
}

#[test]
fn icebox_unified_serializes_with_kind_tag() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1234 });
    let json = serde_json::to_string(&index).unwrap();
    assert!(json.contains(r#""icebox_unified""#));
    assert!(json.contains(r#""kind":"marker""#));
    assert!(json.contains(r#""kind":"story""#));
    assert!(json.contains(r#""id":7"#));
    assert!(json.contains(r#""number":1234"#));
}

#[test]
fn icebox_unified_round_trips() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index
        .icebox_unified
        .push(IceboxItem::Story { number: 1234 });
    let json = serde_json::to_string(&index).unwrap();
    let parsed: PriorityIndex = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.icebox_unified.len(), 2);
    assert_eq!(parsed.icebox_unified[0], IceboxItem::Marker { id: 7 });
    assert_eq!(parsed.icebox_unified[1], IceboxItem::Story { number: 1234 });
}

#[test]
fn legacy_priority_json_loads_with_empty_icebox_unified() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    std::fs::write(&path, r#"{"ready":[100,200],"icebox":[300,400]}"#).unwrap();
    let loaded = load_priority(&path);
    assert_eq!(loaded.columns["ready"], vec![100, 200]);
    assert_eq!(loaded.columns["icebox"], vec![300, 400]);
    assert!(loaded.icebox_unified.is_empty());
}

#[test]
fn empty_icebox_unified_is_omitted_from_json() {
    let index = PriorityIndex::default();
    let json = serde_json::to_string(&index).unwrap();
    assert!(!json.contains("icebox_unified"));
}

fn marker(id: u64, position: u32) -> super::super::iteration_markers::IterationMarker {
    super::super::iteration_markers::IterationMarker {
        id,
        theme_name: format!("theme{id}"),
        position,
        created_at: "2026-05-10T00:00:00Z".to_string(),
        ..super::super::iteration_markers::IterationMarker::default()
    }
}

#[test]
fn migrate_populates_unified_with_stories_then_markers() {
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("icebox".to_string(), vec![100, 200, 300]);
    let markers = vec![marker(7, 0), marker(8, 1)];
    let migrated = migrate_icebox_unified(&mut index, &markers);
    assert!(migrated);
    assert_eq!(index.icebox_unified.len(), 5);
    assert_eq!(index.icebox_unified[0], IceboxItem::Story { number: 100 });
    assert_eq!(index.icebox_unified[1], IceboxItem::Story { number: 200 });
    assert_eq!(index.icebox_unified[2], IceboxItem::Story { number: 300 });
    assert_eq!(index.icebox_unified[3], IceboxItem::Marker { id: 7 });
    assert_eq!(index.icebox_unified[4], IceboxItem::Marker { id: 8 });
}

#[test]
fn migrate_is_idempotent_when_unified_already_populated() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 999 });
    index.columns.insert("icebox".to_string(), vec![100]);
    let markers = vec![marker(7, 0)];
    let migrated = migrate_icebox_unified(&mut index, &markers);
    assert!(!migrated);
    assert_eq!(index.icebox_unified.len(), 1);
    assert_eq!(index.icebox_unified[0], IceboxItem::Story { number: 999 });
}

#[test]
fn migrate_no_op_when_no_legacy_state() {
    let mut index = PriorityIndex::default();
    let migrated = migrate_icebox_unified(&mut index, &[]);
    assert!(!migrated);
    assert!(index.icebox_unified.is_empty());
}

#[test]
fn migrate_with_only_markers_no_icebox_stories() {
    let mut index = PriorityIndex::default();
    let markers = vec![marker(7, 0), marker(8, 1)];
    let migrated = migrate_icebox_unified(&mut index, &markers);
    assert!(migrated);
    assert_eq!(index.icebox_unified.len(), 2);
    assert_eq!(index.icebox_unified[0], IceboxItem::Marker { id: 7 });
    assert_eq!(index.icebox_unified[1], IceboxItem::Marker { id: 8 });
}

#[test]
fn migrate_with_only_icebox_stories_no_markers() {
    let mut index = PriorityIndex::default();
    index.columns.insert("icebox".to_string(), vec![100, 200]);
    let migrated = migrate_icebox_unified(&mut index, &[]);
    assert!(migrated);
    assert_eq!(index.icebox_unified.len(), 2);
    assert_eq!(index.icebox_unified[0], IceboxItem::Story { number: 100 });
    assert_eq!(index.icebox_unified[1], IceboxItem::Story { number: 200 });
}

#[test]
fn migrate_sorts_markers_by_position() {
    let mut index = PriorityIndex::default();
    let markers = vec![marker(8, 5), marker(7, 1), marker(9, 3)];
    let migrated = migrate_icebox_unified(&mut index, &markers);
    assert!(migrated);
    assert_eq!(index.icebox_unified[0], IceboxItem::Marker { id: 7 });
    assert_eq!(index.icebox_unified[1], IceboxItem::Marker { id: 9 });
    assert_eq!(index.icebox_unified[2], IceboxItem::Marker { id: 8 });
}

#[test]
fn reorder_icebox_unified_inserts_story_at_position() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    reorder_icebox_unified(&mut index, IceboxItem::Story { number: 100 }, 1);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 200 },
        ]
    );
}

#[test]
fn reorder_icebox_unified_moves_existing_story_across_marker() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    reorder_icebox_unified(&mut index, IceboxItem::Story { number: 100 }, 3);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 200 },
            IceboxItem::Story { number: 100 },
        ]
    );
}

#[test]
fn reorder_icebox_unified_moves_marker_across_stories() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    reorder_icebox_unified(&mut index, IceboxItem::Marker { id: 7 }, 1);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Marker { id: 7 },
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
        ]
    );
}

#[test]
fn reorder_icebox_unified_clamps_position() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    reorder_icebox_unified(&mut index, IceboxItem::Story { number: 200 }, 99);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 100 },
            IceboxItem::Story { number: 200 },
        ]
    );
}

// ── #1258 AC 22 + 30: rollback failure-mode tests ──

#[test]
fn rollback_on_markers_save_failure_restores_priority() {
    let mut priority = PriorityIndex::default();
    priority
        .icebox_unified
        .push(IceboxItem::Story { number: 100 });
    priority.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let snapshot = priority.clone();
    let mut markers = vec![marker(7, 0)];

    use std::cell::RefCell;
    let priority_saves: RefCell<Vec<PriorityIndex>> = RefCell::new(Vec::new());

    let result = apply_close_iteration_with_rollback(
        &mut priority,
        &mut markers,
        &snapshot,
        |p: &PriorityIndex| {
            priority_saves.borrow_mut().push(p.clone());
            Ok(())
        },
        |_m: &[super::super::iteration_markers::IterationMarker]| Err("disk full".to_string()),
    );

    assert!(result.is_err(), "expected save-failure to surface as Err");
    let err = result.unwrap_err();
    assert!(
        err.contains("rolled back"),
        "expected rollback in error message: {err}"
    );
    let saves = priority_saves.borrow();
    assert_eq!(
        saves.len(),
        2,
        "priority saved twice (once with mutation, once for rollback)"
    );
    // Second save = snapshot (rollback).
    assert_eq!(saves[1].icebox_unified, snapshot.icebox_unified);
}

#[test]
fn rollback_on_markers_save_failure_rollback_save_also_fails_returns_double_error() {
    let mut priority = PriorityIndex::default();
    priority.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let snapshot = priority.clone();
    let mut markers = vec![marker(7, 0)];

    use std::cell::RefCell;
    let save_count: RefCell<u32> = RefCell::new(0);

    let result = apply_close_iteration_with_rollback(
        &mut priority,
        &mut markers,
        &snapshot,
        |_: &PriorityIndex| {
            let mut n = save_count.borrow_mut();
            *n += 1;
            if *n == 1 {
                Ok(())
            } else {
                Err("rollback save failed".to_string())
            }
        },
        |_: &[super::super::iteration_markers::IterationMarker]| {
            Err("markers save failed".to_string())
        },
    );

    let err = result.unwrap_err();
    assert!(
        err.contains("rollback also failed"),
        "expected double-fail message: {err}"
    );
    assert_eq!(*save_count.borrow(), 2);
}

#[test]
fn rollback_no_op_when_persistence_succeeds() {
    let mut priority = PriorityIndex::default();
    priority
        .icebox_unified
        .push(IceboxItem::Story { number: 100 });
    priority.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let snapshot = priority.clone();
    let mut markers = vec![marker(7, 0)];

    use std::cell::RefCell;
    let priority_saves: RefCell<u32> = RefCell::new(0);

    let result = apply_close_iteration_with_rollback(
        &mut priority,
        &mut markers,
        &snapshot,
        |_: &PriorityIndex| {
            *priority_saves.borrow_mut() += 1;
            Ok(())
        },
        |_: &[super::super::iteration_markers::IterationMarker]| Ok(()),
    );

    assert!(result.is_ok());
    let outcome = result.unwrap();
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(outcome.promoted, vec![100]);
    assert_eq!(
        *priority_saves.borrow(),
        1,
        "priority saved once (no rollback path)"
    );
}

#[test]
fn rollback_priority_save_failure_short_circuits_before_markers() {
    let mut priority = PriorityIndex::default();
    priority.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let snapshot = priority.clone();
    let mut markers = vec![marker(7, 0)];

    use std::cell::RefCell;
    let markers_save_called: RefCell<bool> = RefCell::new(false);

    let result = apply_close_iteration_with_rollback(
        &mut priority,
        &mut markers,
        &snapshot,
        |_: &PriorityIndex| Err("priority save failed".to_string()),
        |_: &[super::super::iteration_markers::IterationMarker]| {
            *markers_save_called.borrow_mut() = true;
            Ok(())
        },
    );

    assert!(result.is_err());
    assert!(
        !*markers_save_called.borrow(),
        "markers save should NOT be called when priority save fails first"
    );
}

use crate::board::domain::StoryState;

fn states_map(entries: &[(u64, StoryState)]) -> std::collections::HashMap<u64, StoryState> {
    entries.iter().copied().collect()
}

#[test]
fn auto_heal_evicts_stories_whose_workflow_state_is_not_icebox() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 300 });
    let states = states_map(&[
        (100, StoryState::Accepted),
        (200, StoryState::Icebox),
        (300, StoryState::Ready),
    ]);
    let evicted = auto_heal_icebox_unified_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 2);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker { id: 7 },
        ]
    );
}

#[test]
fn auto_heal_clean_state_is_no_op() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let states = states_map(&[(100, StoryState::Icebox)]);
    let evicted = auto_heal_icebox_unified_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 0);
    assert_eq!(index.icebox_unified.len(), 2);
}

#[test]
fn auto_heal_preserves_all_markers_unconditionally() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 999 });
    index.icebox_unified.push(IceboxItem::Marker { id: 8 });
    let states = states_map(&[(999, StoryState::Delivered)]);
    let evicted = auto_heal_icebox_unified_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 1);
    assert_eq!(
        index.icebox_unified,
        vec![IceboxItem::Marker { id: 7 }, IceboxItem::Marker { id: 8 }]
    );
}

#[test]
fn auto_heal_preserves_icebox_state_stories() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    let states = states_map(&[(100, StoryState::Icebox), (200, StoryState::Icebox)]);
    let evicted = auto_heal_icebox_unified_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 0);
    assert_eq!(index.icebox_unified.len(), 2);
}

#[test]
fn auto_heal_preserves_stories_with_unknown_state() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    let states = states_map(&[(100, StoryState::Icebox)]);
    let evicted = auto_heal_icebox_unified_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 0);
    assert_eq!(index.icebox_unified.len(), 2);
}

#[test]
fn auto_heal_idempotent_second_run_is_no_op() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let states = states_map(&[(100, StoryState::Accepted), (200, StoryState::Icebox)]);
    let first = auto_heal_icebox_unified_via_workflow_state(&mut index, &states);
    assert_eq!(first, 1);
    let second = auto_heal_icebox_unified_via_workflow_state(&mut index, &states);
    assert_eq!(second, 0);
    assert_eq!(
        index.icebox_unified,
        vec![
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker { id: 7 },
        ]
    );
}

// ── #1272: auto_heal_ready_column_via_workflow_state ──

#[test]
fn auto_heal_ready_evicts_non_ready_stories() {
    let mut index = PriorityIndex::default();
    // Today's empirical repro shape: accepted/closed stories stuck +
    // duplicates from repeated close_iteration appends.
    index.columns.insert(
        "ready".to_string(),
        vec![
            100, 1269, 1268, 1251, 1252, 1253, 1254, 1251, 1252, 1253, 1254,
        ],
    );
    let states = states_map(&[
        (100, StoryState::Accepted),
        (1269, StoryState::Accepted),
        (1268, StoryState::Accepted),
        (1251, StoryState::Ready),
        (1252, StoryState::Ready),
        (1253, StoryState::Ready),
        (1254, StoryState::Ready),
    ]);
    let evicted = auto_heal_ready_column_via_workflow_state(&mut index, &states);
    // 11 entries → 4 unique-ready; 7 evicted (3 accepted + 4 duplicates).
    assert_eq!(evicted, 7);
    assert_eq!(
        index.columns.get("ready").unwrap(),
        &vec![1251, 1252, 1253, 1254],
        "only canonical-Ready stories survive; duplicates removed"
    );
}

#[test]
fn auto_heal_ready_clean_state_no_op() {
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("ready".to_string(), vec![1251, 1252, 1253]);
    let states = states_map(&[
        (1251, StoryState::Ready),
        (1252, StoryState::Ready),
        (1253, StoryState::Ready),
    ]);
    let evicted = auto_heal_ready_column_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 0);
    assert_eq!(index.columns.get("ready").unwrap(), &vec![1251, 1252, 1253]);
}

#[test]
fn auto_heal_ready_preserves_unknown_state_stories() {
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![1251, 9999]);
    let states = states_map(&[(1251, StoryState::Ready)]);
    let evicted = auto_heal_ready_column_via_workflow_state(&mut index, &states);
    assert_eq!(
        evicted, 0,
        "unknown-state stories preserved (safer default)"
    );
    assert_eq!(index.columns.get("ready").unwrap(), &vec![1251, 9999]);
}

#[test]
fn auto_heal_ready_dedups_duplicate_entries_with_ready_state() {
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("ready".to_string(), vec![1251, 1252, 1251, 1253, 1252]);
    let states = states_map(&[
        (1251, StoryState::Ready),
        (1252, StoryState::Ready),
        (1253, StoryState::Ready),
    ]);
    let evicted = auto_heal_ready_column_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 2, "2 duplicates evicted");
    assert_eq!(
        index.columns.get("ready").unwrap(),
        &vec![1251, 1252, 1253],
        "first occurrence wins; later duplicates dropped"
    );
}

#[test]
fn auto_heal_ready_noop_when_column_absent() {
    let mut index = PriorityIndex::default();
    let states = states_map(&[]);
    let evicted = auto_heal_ready_column_via_workflow_state(&mut index, &states);
    assert_eq!(evicted, 0);
    assert!(!index.columns.contains_key("ready"));
}

#[test]
fn auto_heal_ready_idempotent_second_run_no_op() {
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("ready".to_string(), vec![100, 1251, 100]);
    let states = states_map(&[(100, StoryState::Accepted), (1251, StoryState::Ready)]);
    let first = auto_heal_ready_column_via_workflow_state(&mut index, &states);
    let second = auto_heal_ready_column_via_workflow_state(&mut index, &states);
    assert_eq!(first, 2);
    assert_eq!(second, 0);
    assert_eq!(index.columns.get("ready").unwrap(), &vec![1251]);
}

// ── #1272: close_iteration_atomic dedup before append ──

#[test]
fn close_iteration_promote_batch_dedups_existing_ready_entries() {
    let mut index = PriorityIndex::default();
    // Pre-existing ready column with #100. close_iteration promotes
    // #100 again — must dedup so the array stays unique.
    index.columns.insert("ready".to_string(), vec![100]);
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(outcome.promoted, vec![100]);
    assert_eq!(
        index.columns.get("ready").unwrap(),
        &vec![100],
        "no duplicate — remove-then-append idempotent"
    );
}

#[test]
fn close_iteration_promote_batch_running_twice_does_not_duplicate() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 300 });
    index.icebox_unified.push(IceboxItem::Marker { id: 8 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];

    // First close_iteration: marker 7 + stories 100/200
    let first = close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(first.promoted, vec![100, 200]);

    // Repeat (idempotency check; in production this'd be a second
    // close_iteration on the next marker).
    let second = close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(second.promoted, vec![300]);

    assert_eq!(
        index.columns.get("ready").unwrap(),
        &vec![100, 200, 300],
        "no duplicates across two close_iteration fires"
    );
}

#[tokio::test]
async fn auto_heal_boot_integration_evicts_stale_writes_clean_priority() {
    use crate::board::RefreshDriver;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::orchestrator::SyncOrchestrator;
    use crate::board::store::BoardStore;
    use crate::session::Session;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let dir = tempfile::tempdir().unwrap();
    let priority_path = dir.path().join("priority.json");
    let mut dirty = PriorityIndex::default();
    dirty.icebox_unified.push(IceboxItem::Story { number: 100 });
    dirty.icebox_unified.push(IceboxItem::Story { number: 200 });
    dirty.icebox_unified.push(IceboxItem::Marker { id: 7 });
    dirty.icebox_unified.push(IceboxItem::Story { number: 300 });
    save_priority(&priority_path, &dirty).unwrap();

    let session_dir = dir.path().join("session");
    std::fs::create_dir_all(&session_dir).unwrap();
    let (wake_tx, _wake_rx) = tokio::sync::broadcast::channel(16);
    let session = Arc::new(Session::new(session_dir, "test".to_string(), wake_tx));

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
    adapter.seed_stories(vec![
        super::super::super::board::domain::Story::new_minimal(
            100,
            "s100".to_string(),
            StoryState::Accepted,
            ts,
            "pm".to_string(),
        ),
        super::super::super::board::domain::Story::new_minimal(
            200,
            "s200".to_string(),
            StoryState::Icebox,
            ts,
            "pm".to_string(),
        ),
        super::super::super::board::domain::Story::new_minimal(
            300,
            "s300".to_string(),
            StoryState::Ready,
            ts,
            "pm".to_string(),
        ),
    ]);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));

    crate::auto_heal_icebox_at_path(&priority_path, session, refresh).await;

    let healed = load_priority(&priority_path);
    assert_eq!(
        healed.icebox_unified,
        vec![
            IceboxItem::Story { number: 200 },
            IceboxItem::Marker { id: 7 },
        ],
        "stale accepted/ready stories evicted; icebox-state + marker preserved"
    );
}

#[test]
fn migrate_re_run_is_no_op() {
    let mut index = PriorityIndex::default();
    index.columns.insert("icebox".to_string(), vec![100, 200]);
    let markers = vec![marker(7, 0)];
    let first = migrate_icebox_unified(&mut index, &markers);
    assert!(first);
    let snapshot = index.icebox_unified.clone();
    let second = migrate_icebox_unified(&mut index, &markers);
    assert!(!second);
    assert_eq!(index.icebox_unified, snapshot);
}

fn story(number: u64, state: &str) -> serde_json::Value {
    serde_json::json!({ "number": number, "state": state, "title": "s" })
}

fn numbers_of(stories: &[serde_json::Value]) -> Vec<u64> {
    stories
        .iter()
        .filter_map(|s| s.get("number").and_then(|v| v.as_u64()))
        .collect()
}

#[test]
fn apply_priority_reorders_within_ready_column() {
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![300, 100]);
    let stories = vec![
        story(100, "ready"),
        story(200, "ready"),
        story(300, "ready"),
    ];
    let sorted = apply_priority_to_board(stories, &index);
    assert_eq!(numbers_of(&sorted), vec![300, 100, 200]);
}

#[test]
fn apply_priority_leaves_other_states_untouched() {
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![200, 100]);
    let stories = vec![
        story(100, "ready"),
        story(200, "ready"),
        story(500, "delivered"),
        story(400, "delivered"),
    ];
    let sorted = apply_priority_to_board(stories, &index);
    assert_eq!(numbers_of(&sorted), vec![200, 100, 500, 400]);
}

#[test]
fn apply_priority_preserves_inter_state_encounter_order() {
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![100]);
    let stories = vec![
        story(500, "delivered"),
        story(100, "ready"),
        story(600, "delivered"),
    ];
    let sorted = apply_priority_to_board(stories, &index);
    assert_eq!(numbers_of(&sorted), vec![500, 600, 100]);
}

#[test]
fn apply_priority_with_empty_index_is_identity() {
    let index = PriorityIndex::default();
    let stories = vec![
        story(100, "ready"),
        story(200, "ready"),
        story(300, "ready"),
    ];
    let sorted = apply_priority_to_board(stories.clone(), &index);
    assert_eq!(numbers_of(&sorted), numbers_of(&stories));
}

#[test]
fn apply_priority_entries_for_absent_numbers_ignored() {
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("ready".to_string(), vec![999, 200, 100]);
    let stories = vec![
        story(100, "ready"),
        story(200, "ready"),
        story(300, "ready"),
    ];
    let sorted = apply_priority_to_board(stories, &index);
    assert_eq!(numbers_of(&sorted), vec![200, 100, 300]);
}
