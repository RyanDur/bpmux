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
