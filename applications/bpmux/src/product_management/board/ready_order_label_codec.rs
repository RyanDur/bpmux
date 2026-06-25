#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::ready_order_label::*;

fn ready_json(number: u64, membership: Option<(u64, u32)>) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "ready",
        "iteration_membership": membership.map(|(it, pos)| serde_json::json!({
            "iteration_number": it,
            "position": pos,
        })),
        "is_current": false,
    })
}

fn current_iteration_json(number: u64) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "state": "icebox",
        "iteration_membership": serde_json::Value::Null,
        "is_current": true,
    })
}

fn ready_numbers_in_order(sorted: &[serde_json::Value]) -> Vec<u64> {
    sorted
        .iter()
        .filter(|s| s.get("state").and_then(|v| v.as_str()) == Some("ready"))
        .filter_map(|s| s.get("number").and_then(|v| v.as_u64()))
        .collect()
}

#[test]
fn ready_stories_sort_by_current_iteration_membership_position() {
    let stories = vec![
        current_iteration_json(7000),
        ready_json(5001, Some((7000, 300))),
        ready_json(5002, Some((7000, 100))),
        ready_json(5003, Some((7000, 200))),
    ];
    let sorted = sort_board_ready_by_membership(stories);
    assert_eq!(
        ready_numbers_in_order(&sorted),
        vec![5002, 5003, 5001],
        "ready stories must order by current-iteration membership-position (100,200,300)"
    );
}

#[test]
fn loose_ready_non_members_sort_after_members_by_issue_number() {
    let stories = vec![
        current_iteration_json(7000),
        ready_json(5050, None),
        ready_json(5001, Some((7000, 200))),
        ready_json(5040, None),
        ready_json(5002, Some((7000, 100))),
    ];
    let sorted = sort_board_ready_by_membership(stories);
    assert_eq!(
        ready_numbers_in_order(&sorted),
        vec![5002, 5001, 5040, 5050],
        "current-iteration members first by position, then loose-ready by issue-number"
    );
}

#[test]
fn membership_for_a_different_iteration_is_treated_as_loose() {
    let stories = vec![
        current_iteration_json(7000),
        ready_json(5001, Some((9999, 100))),
        ready_json(5002, Some((7000, 100))),
    ];
    let sorted = sort_board_ready_by_membership(stories);
    assert_eq!(
        ready_numbers_in_order(&sorted),
        vec![5002, 5001],
        "a story member of a non-current iteration is loose (sorts after current members)"
    );
}
