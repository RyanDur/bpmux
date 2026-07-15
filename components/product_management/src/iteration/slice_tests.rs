use super::slice::*;
use crate::iteration::{Iteration, IterationId};
use chrono::{TimeZone, Utc};

fn iteration_of(id: u64) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: format!("iteration-{id}"),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: vec![],
        review_closers: vec![],
        dev_ok_by: vec![],
        is_current: false,
        complete: false,
    }
}

#[test]
fn get_hits_a_saved_iteration_and_misses_an_absent_id() {
    let state = IterationState {
        iterations: vec![iteration_of(1)],
    };

    assert_eq!(
        get(&state, IterationId::new(1)).map(|iteration| iteration.id.value()),
        Some(1)
    );
    assert_eq!(get(&state, IterationId::new(99)), None);
}
