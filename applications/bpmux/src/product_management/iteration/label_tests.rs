use super::label::*;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::iteration::IterationId;
use crate::product_management::story::{Story, StoryId, StoryState};
use chrono::{TimeZone, Utc};

fn seed(number: u64, raw_labels: Vec<String>) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(number),
        format!("member-{number}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.raw_labels = raw_labels;
    s
}

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

fn seed_with_iteration_membership(number: u64, iteration_number: u64, position: u32) -> Story {
    let mut s = seed(number, vec![]);
    s.raw_labels
        .push(format!("parent-iteration-#{iteration_number}.{position}"));
    s.iteration_membership = Some((IterationId::new(iteration_number), position));
    s
}

#[test]
fn members_of_iteration_attributes_by_canonical_identity_1553() {
    let stories = vec![
        seed_with_iteration_membership(2001, 1397, 100),
        seed_with_iteration_membership(2002, 1397, 200),
    ];
    assert_eq!(
        members_of_iteration(&stories, IterationId::new(1397)),
        vec![2001, 2002]
    );
}

#[test]
fn members_of_iteration_no_longer_attributes_legacy_name_label_1553() {
    let stories = vec![
        seed(2001, vec!["iteration-Telescopium.100".to_string()]),
        seed(2002, vec!["iteration-Telescopium.200".to_string()]),
    ];
    assert!(
        members_of_iteration(&stories, IterationId::new(1397)).is_empty(),
        "legacy name labels are retired; attribution is identity-only"
    );
}

#[test]
fn members_of_iteration_returns_only_canonical_members_dropping_legacy_only_1553() {
    let stories = vec![
        seed_with_iteration_membership(2001, 1397, 100),
        seed(2002, vec!["iteration-Telescopium.200".to_string()]),
        seed_with_iteration_membership(2003, 1397, 300),
    ];
    assert_eq!(
        members_of_iteration(&stories, IterationId::new(1397)),
        vec![2001, 2003],
        "a legacy-only member drops; only identity-attributed members remain"
    );
}

#[test]
fn members_of_iteration_ignores_legacy_label_when_canonical_present_1553() {
    let mut s = seed(2001, vec!["iteration-Telescopium.999".to_string()]);
    s.raw_labels.push("parent-iteration-#1397.100".to_string());
    s.iteration_membership = Some((IterationId::new(1397), 100));
    let stories = vec![s];
    assert_eq!(
        members_of_iteration(&stories, IterationId::new(1397)),
        vec![2001]
    );
}

#[test]
fn members_of_iteration_skips_closed_stories_1399() {
    let mut closed = seed_with_iteration_membership(2001, 1397, 100);
    closed.is_closed = true;
    let stories = vec![closed, seed_with_iteration_membership(2002, 1397, 200)];
    assert_eq!(
        members_of_iteration(&stories, IterationId::new(1397)),
        vec![2002]
    );
}

#[test]
fn members_of_iteration_does_not_match_different_iteration_number_1399() {
    let stories = vec![
        seed_with_iteration_membership(2001, 1397, 100),
        seed_with_iteration_membership(2002, 1398, 200),
    ];
    assert_eq!(
        members_of_iteration(&stories, IterationId::new(1397)),
        vec![2001]
    );
}

#[test]
fn members_of_iteration_sorts_canonical_members_by_position_ascending_1553() {
    let stories = vec![
        seed_with_iteration_membership(2001, 1397, 300),
        seed_with_iteration_membership(2002, 1397, 100),
        seed_with_iteration_membership(2003, 1397, 200),
    ];
    assert_eq!(
        members_of_iteration(&stories, IterationId::new(1397)),
        vec![2002, 2003, 2001]
    );
}

#[test]
fn members_of_iteration_does_not_inherit_legacy_same_named_iteration_1553() {
    let legacy_only = seed(2001, vec!["iteration-Carina.100".to_string()]);
    let stories = vec![legacy_only];
    assert!(
        members_of_iteration(&stories, IterationId::new(2000)).is_empty(),
        "a new iteration must not claim a story by a retired same-named iteration's name"
    );
}

#[test]
fn members_of_iteration_attributes_by_identity_across_rename_1553() {
    let stories = vec![seed_with_iteration_membership(2001, 2000, 1)];
    assert_eq!(
        members_of_iteration(&stories, IterationId::new(2000)),
        vec![2001],
        "canonical identity attribution is name-independent; survives a rename"
    );
}
