use super::*;

fn story(number: u64, state: &str) -> serde_json::Value {
    serde_json::json!({ "number": number, "state": state, "title": "s" })
}

fn ready_story_with_order(number: u64, position: u32) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "ready",
        "title": "s",
        "labels": [{"name": "ready"}, {"name": format!("ready-order.{position}")}],
    })
}

fn numbers_of(stories: &[serde_json::Value]) -> Vec<u64> {
    stories
        .iter()
        .filter_map(|s| s.get("number").and_then(|v| v.as_u64()))
        .collect()
}

#[test]
fn save_and_load_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("priority.json");
    let mut index = PriorityIndex::default();
    index.columns.insert("ready".to_string(), vec![100, 200]);
    save_priority(&path, &index).unwrap();
    let loaded = load_priority(&path);
    assert_eq!(loaded.columns["ready"], vec![100, 200]);
}

#[test]
fn load_missing_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("does-not-exist.json");
    let loaded = load_priority(&path);
    assert!(loaded.columns.is_empty());
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

#[test]
fn sort_board_ready_sorts_by_ready_order_label_when_present() {
    let stories = vec![
        ready_story_with_order(300, 300),
        ready_story_with_order(100, 100),
        ready_story_with_order(200, 200),
    ];
    let sorted = sort_board_by_ready_order_labels(stories);
    assert_eq!(numbers_of(&sorted), vec![100, 200, 300]);
}

#[test]
fn sort_board_ready_unlabeled_stories_fall_to_end_by_issue_number() {
    let stories = vec![
        story(300, "ready"),
        ready_story_with_order(100, 50),
        story(200, "ready"),
    ];
    let sorted = sort_board_by_ready_order_labels(stories);
    assert_eq!(
        numbers_of(&sorted),
        vec![100, 200, 300],
        "labeled first by position; unlabeled follow by issue number"
    );
}

#[test]
fn sort_board_leaves_non_ready_states_in_encounter_order() {
    let stories = vec![
        story(500, "delivered"),
        ready_story_with_order(100, 100),
        story(400, "delivered"),
    ];
    let sorted = sort_board_by_ready_order_labels(stories);
    assert_eq!(numbers_of(&sorted), vec![500, 400, 100]);
}

#[test]
fn sort_board_preserves_inter_state_encounter_order() {
    let stories = vec![
        story(500, "delivered"),
        ready_story_with_order(100, 100),
        story(600, "delivered"),
    ];
    let sorted = sort_board_by_ready_order_labels(stories);
    assert_eq!(numbers_of(&sorted), vec![500, 600, 100]);
}

#[test]
fn sort_board_empty_input_is_identity() {
    let sorted = sort_board_by_ready_order_labels(vec![]);
    assert!(sorted.is_empty());
}

#[test]
fn sort_board_no_ready_stories_returns_input_in_encounter_order() {
    let stories = vec![story(100, "delivered"), story(200, "in progress")];
    let sorted = sort_board_by_ready_order_labels(stories);
    assert_eq!(numbers_of(&sorted), vec![100, 200]);
}
