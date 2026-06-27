#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::drill::*;
use crate::product_management::story::{Story, StoryState};
use crate::workflow::bundled_default::bundled_default;
use chrono::Utc;

fn iteration(number: u64, outcome_number: u64, position: u32) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Crux-{number}"),
        StoryState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    s.raw_labels.push("iteration".to_string());
    s.raw_labels
        .push(format!("parent-outcome-#{outcome_number}.{position}"));
    s.body = Some("## composed".to_string());
    s.story_type = crate::product_management::story::StoryType::Iteration;
    s
}

fn fully_ready(number: u64, outcome_number: u64, position: u32, member: u64) -> (Story, Story) {
    let mut it = iteration(number, outcome_number, position);
    it.raw_labels.push(
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_iteration_review_closed("designer"),
    );
    it.raw_labels.push(
        crate::product_management::board::adapters::gh_label_codec::GhLabelCodec
            .encode_iteration_dev_ok("dev"),
    );
    let mut m = Story::new_minimal(
        member,
        "member".to_string(),
        StoryState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    m.raw_labels.push(format!("parent-iteration-#{number}.1"));
    (it, m)
}

#[test]
fn iterations_render_in_outcome_membership_position_order() {
    let stories = vec![iteration(21, 11, 2), iteration(20, 11, 1)];
    let drill = derive_outcome_drill_in(11, &stories, &bundled_default());
    let numbers: Vec<u64> = drill.iterations.iter().map(|i| i.number).collect();
    assert_eq!(
        numbers,
        vec![20, 21],
        "sorted by position, not insertion order"
    );
    assert_eq!(drill.iterations[0].order, 1);
    assert_eq!(drill.iterations[1].order, 2);
}

#[test]
fn an_iteration_with_nothing_done_reads_not_started_with_no_checks_and_no_convene() {
    let mut it = iteration(20, 11, 1);
    it.body = None;
    let stories = vec![it];
    let drill = derive_outcome_drill_in(11, &stories, &bundled_default());
    assert_eq!(drill.iterations[0].state, DrillIterationState::NotStarted);
    assert!(drill.iterations[0].checks.is_empty());
    assert!(
        drill.convene.is_none(),
        "nothing-done is not a convene candidate"
    );
}

#[test]
fn an_iteration_meeting_every_check_reads_ready_with_no_convene() {
    let (it, member) = fully_ready(20, 11, 1, 30);
    let stories = vec![it, member];
    let drill = derive_outcome_drill_in(11, &stories, &bundled_default());
    assert_eq!(drill.iterations[0].state, DrillIterationState::Ready);
    assert_eq!(drill.iterations[0].gap_count, 0);
    assert!(
        drill.convene.is_none(),
        "a fully-ready outcome has nothing to convene"
    );
}

#[test]
fn the_first_partially_done_gap_holder_reads_next_and_others_read_gap() {
    let stories = vec![iteration(20, 11, 1), iteration(21, 11, 2)];
    let drill = derive_outcome_drill_in(11, &stories, &bundled_default());
    assert_eq!(drill.iterations[0].state, DrillIterationState::Next);
    assert_eq!(drill.iterations[0].gap_count, 3);
    assert_eq!(
        drill.iterations[1].state,
        DrillIterationState::Gap,
        "a later gap-holder is gap, not next"
    );
    let banner = drill
        .convene
        .expect("a next gap-holder yields a convene banner");
    assert_eq!(banner.next_iteration_name, "Crux-20");
    assert_eq!(banner.next_gap_count, 3);
    assert_eq!(banner.active_gap_label.as_deref(), Some("stories-ordered"));
    assert_eq!(banner.then_iteration_name.as_deref(), Some("Crux-21"));
}

#[test]
fn a_closed_iteration_reads_done_excluded_from_convene_with_no_checks() {
    let mut done = iteration(20, 11, 1);
    done.is_closed = true;
    let stories = vec![done, iteration(21, 11, 2)];
    let drill = derive_outcome_drill_in(11, &stories, &bundled_default());
    assert_eq!(drill.iterations[0].state, DrillIterationState::Done);
    assert!(drill.iterations[0].checks.is_empty());
    assert_eq!(
        drill.iterations[1].state,
        DrillIterationState::Next,
        "the convene-next skips the done iteration to the real gap-holder"
    );
    let banner = drill.convene.expect("the non-done gap-holder is next");
    assert_eq!(banner.next_iteration_name, "Crux-21");
}

#[test]
fn checks_render_met_then_first_gap_then_pending() {
    let stories = vec![iteration(20, 11, 1)];
    let drill = derive_outcome_drill_in(11, &stories, &bundled_default());
    let checks = &drill.iterations[0].checks;
    assert_eq!(checks.len(), 4);
    assert_eq!(checks[0].label, "drafted");
    assert_eq!(checks[0].state, ConveneCheckState::Met);
    assert_eq!(checks[1].label, "stories-ordered");
    assert_eq!(checks[1].state, ConveneCheckState::Gap);
    assert_eq!(checks[2].state, ConveneCheckState::Pending);
    assert_eq!(checks[3].state, ConveneCheckState::Pending);
}
