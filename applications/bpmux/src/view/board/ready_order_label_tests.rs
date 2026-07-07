#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::ready_order_label::*;

fn ready_json(id: u64) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "state": "ready",
    })
}

fn ready_numbers_in_order(sorted: &[serde_json::Value]) -> Vec<u64> {
    sorted
        .iter()
        .filter(|s| s.get("state").and_then(|v| v.as_str()) == Some("ready"))
        .filter_map(|s| s.get("id").and_then(|v| v.as_u64()))
        .collect()
}

#[test]
fn ready_stories_sort_by_current_iteration_stories_order() {
    let stories = vec![ready_json(5001), ready_json(5002), ready_json(5003)];
    let current_order = [5002, 5003, 5001];
    let sorted = sort_board_ready_by_membership(stories, &current_order);
    assert_eq!(
        ready_numbers_in_order(&sorted),
        vec![5002, 5003, 5001],
        "ready stories order by their index in the current iteration's stories list"
    );
}

#[test]
fn wire_ready_rows_absent_from_the_membership_order_sort_last_by_issue_number() {
    let stories = vec![
        ready_json(5050),
        ready_json(5001),
        ready_json(5040),
        ready_json(5002),
    ];
    let current_order = [5002, 5001];
    let sorted = sort_board_ready_by_membership(stories, &current_order);
    assert_eq!(
        ready_numbers_in_order(&sorted),
        vec![5002, 5001, 5040, 5050],
        "rows found in the membership order come first in that order; rows absent from it fall back to issue-number"
    );
}

#[test]
fn a_wire_ready_row_absent_from_the_membership_order_sorts_after_members() {
    let stories = vec![ready_json(5001), ready_json(5002)];
    let current_order = [5002];
    let sorted = sort_board_ready_by_membership(stories, &current_order);
    assert_eq!(
        ready_numbers_in_order(&sorted),
        vec![5002, 5001],
        "a row the membership order does not name sorts after the ordered members"
    );
}
