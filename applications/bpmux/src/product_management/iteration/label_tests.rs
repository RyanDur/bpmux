use super::label::*;
use crate::product_management::iteration::{Iteration, IterationId};
use chrono::{TimeZone, Utc};

#[test]
fn name_strips_iteration_prefix() {
    assert_eq!(iteration_name_from_title("Iteration Pegasus"), "Pegasus");
}

#[test]
fn name_hyphenates_internal_whitespace() {
    assert_eq!(
        iteration_name_from_title("Iteration Big Dipper"),
        "Big-Dipper"
    );
}

#[test]
fn name_returns_trimmed_title_when_prefix_absent() {
    assert_eq!(iteration_name_from_title("  Pegasus  "), "Pegasus");
}

fn iteration(id: u64, is_current: bool, complete: bool) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: vec![],
        review_closers: vec![],
        dev_ok_by: vec![],
        is_current,
        complete,
    }
}

#[test]
fn current_iteration_id_picks_the_open_current_and_ignores_complete_ones() {
    let iterations = vec![
        iteration(800, true, true),
        iteration(801, true, false),
        iteration(802, false, false),
    ];
    assert_eq!(
        current_iteration_id(&iterations),
        Some(IterationId::new(801))
    );
}
