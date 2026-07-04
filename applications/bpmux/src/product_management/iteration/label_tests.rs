use super::label::*;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
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

#[test]
fn parse_label_simple_position() {
    assert_eq!(
        parse_iteration_label(&GhLabelCodec, "iteration-Pegasus.100"),
        Some(IterationLabelParts {
            name: "Pegasus".to_string(),
            position: 100
        })
    );
}

#[test]
fn parse_label_hyphenated_name() {
    assert_eq!(
        parse_iteration_label(&GhLabelCodec, "iteration-Big-Dipper.200"),
        Some(IterationLabelParts {
            name: "Big-Dipper".to_string(),
            position: 200
        })
    );
}

#[test]
fn parse_label_rejects_legacy_pegasus_format() {
    assert_eq!(
        parse_iteration_label(&GhLabelCodec, "iteration-#1400"),
        None
    );
}

#[test]
fn parse_label_rejects_nondigit_position() {
    assert_eq!(
        parse_iteration_label(&GhLabelCodec, "iteration-Pegasus.foo"),
        None
    );
}

#[test]
fn parse_label_rejects_missing_position() {
    assert_eq!(
        parse_iteration_label(&GhLabelCodec, "iteration-Pegasus"),
        None
    );
}

#[test]
fn compute_insert_position_empty_container_returns_default_first() {
    assert_eq!(compute_insert_position(0, &[]), DEFAULT_FIRST_POSITION);
    assert_eq!(compute_insert_position(5, &[]), DEFAULT_FIRST_POSITION);
}

#[test]
fn compute_insert_position_at_head_returns_midpoint_below_first() {
    assert_eq!(compute_insert_position(0, &[100, 200, 300]), 50);
    assert_eq!(compute_insert_position(0, &[200]), 100);
}

#[test]
fn compute_insert_position_at_tail_appends_default_step() {
    assert_eq!(
        compute_insert_position(3, &[100, 200, 300]),
        300 + DEFAULT_STEP
    );
    assert_eq!(
        compute_insert_position(99, &[100, 200, 300]),
        300 + DEFAULT_STEP
    );
}

#[test]
fn compute_insert_position_between_existing_returns_midpoint() {
    assert_eq!(compute_insert_position(1, &[100, 300]), 200);
    assert_eq!(compute_insert_position(2, &[100, 200, 400]), 300);
}

#[test]
fn compute_insert_position_cascade_when_gap_too_tight() {
    assert_eq!(compute_insert_position(1, &[100, 101]), 100);
}

#[test]
fn compute_insert_position_sorts_input_before_computing() {
    assert_eq!(compute_insert_position(1, &[300, 100, 200]), 150);
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
