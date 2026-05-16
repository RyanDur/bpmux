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
fn legacy_priority_json_loads_columns_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    std::fs::write(&path, r#"{"ready":[100,200],"icebox":[300,400]}"#).unwrap();
    let loaded = load_priority(&path);
    assert_eq!(loaded.columns["ready"], vec![100, 200]);
    assert_eq!(loaded.columns["icebox"], vec![300, 400]);
}

use crate::board::domain::StoryState;

fn states_map(entries: &[(u64, StoryState)]) -> std::collections::HashMap<u64, StoryState> {
    entries.iter().copied().collect()
}

#[test]
fn auto_heal_ready_evicts_non_ready_stories() {
    let mut index = PriorityIndex::default();
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
