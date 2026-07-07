#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::drill::*;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::integrations::github::mapper::decode_outcome_from_story;
use crate::integrations::github::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::product_management::iteration::IterationId;
use crate::product_management::outcome::{Outcome, OutcomeId};
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::workflow::bundled_default::bundled_default;
use chrono::Utc;

fn iteration(id: u64, outcome_id: u64) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Crux-{id}"),
        BoardState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    s.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome_id)),
            stories: vec![],
        }
        .write_into_body("## composed"),
    );
    (s, vec!["iteration".to_string()])
}

fn outcome_with(id: u64, iterations: Vec<u64>) -> Outcome {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Outcome #{id}"),
        BoardState::Icebox,
        Utc::now(),
        "pm".to_string(),
    );
    s.body = Some(
        OutcomeRelationships {
            roadmap: None,
            iterations: iterations.into_iter().map(IterationId::new).collect(),
        }
        .write_into_body("## Outcome\n"),
    );
    decode_outcome_from_story(&s, &["outcome".to_string()])
}

fn decode_iterations(
    seeds: &[(Story, Vec<String>)],
) -> Vec<crate::product_management::iteration::Iteration> {
    seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| {
            crate::integrations::github::mapper::decode_iteration_from_story(
                story,
                labels,
                story.is_concluded(),
            )
        })
        .collect()
}

fn fully_ready(id: u64, outcome_id: u64, member: u64) -> (Story, Vec<String>) {
    let (mut it, mut labels) = iteration(id, outcome_id);
    it.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome_id)),
            stories: vec![StoryId::new(member)],
        }
        .write_into_body("## composed"),
    );
    labels.push(GhLabelCodec.encode_iteration_review_closed("designer"));
    labels.push(GhLabelCodec.encode_iteration_dev_ok("dev"));
    (it, labels)
}

#[test]
fn iterations_render_in_the_outcomes_priority_order() {
    let outcome = outcome_with(11, vec![20, 21]);
    let stories = vec![iteration(21, 11), iteration(20, 11)];
    let drill = derive_outcome_drill_in(&outcome, &decode_iterations(&stories), &bundled_default());
    let ids: Vec<u64> = drill.iterations.iter().map(|i| i.id.value()).collect();
    assert_eq!(
        ids,
        vec![20, 21],
        "ordered by the outcome's iteration list, not insertion order"
    );
    assert_eq!(drill.iterations[0].order, 1);
    assert_eq!(drill.iterations[1].order, 2);
}

#[test]
fn an_iteration_with_nothing_done_reads_not_started_with_no_checks_and_no_convene() {
    let (mut it, it_labels) = iteration(20, 11);
    it.body = None;
    let outcome = outcome_with(11, vec![20]);
    let stories = vec![(it, it_labels)];
    let drill = derive_outcome_drill_in(&outcome, &decode_iterations(&stories), &bundled_default());
    assert_eq!(drill.iterations[0].state, DrillIterationState::NotStarted);
    assert!(drill.iterations[0].checks.is_empty());
    assert!(
        drill.convene.is_none(),
        "nothing-done is not a convene candidate"
    );
}

#[test]
fn an_iteration_meeting_every_check_reads_ready_with_no_convene() {
    let it = fully_ready(20, 11, 30);
    let outcome = outcome_with(11, vec![20]);
    let stories = vec![it];
    let drill = derive_outcome_drill_in(&outcome, &decode_iterations(&stories), &bundled_default());
    assert_eq!(drill.iterations[0].state, DrillIterationState::Ready);
    assert_eq!(drill.iterations[0].gap_count, 0);
    assert!(
        drill.convene.is_none(),
        "a fully-ready outcome has nothing to convene"
    );
}

#[test]
fn the_first_partially_done_gap_holder_reads_next_and_others_read_gap() {
    let outcome = outcome_with(11, vec![20, 21]);
    let stories = vec![iteration(20, 11), iteration(21, 11)];
    let drill = derive_outcome_drill_in(&outcome, &decode_iterations(&stories), &bundled_default());
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
    let (mut done, done_labels) = iteration(20, 11);
    done.state = BoardState::Accepted;
    let outcome = outcome_with(11, vec![20, 21]);
    let stories = vec![(done, done_labels), iteration(21, 11)];
    let drill = derive_outcome_drill_in(&outcome, &decode_iterations(&stories), &bundled_default());
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
    let outcome = outcome_with(11, vec![20]);
    let stories = vec![iteration(20, 11)];
    let drill = derive_outcome_drill_in(&outcome, &decode_iterations(&stories), &bundled_default());
    let checks = &drill.iterations[0].checks;
    assert_eq!(checks.len(), 4);
    assert_eq!(checks[0].label, "drafted");
    assert_eq!(checks[0].state, ConveneCheckState::Met);
    assert_eq!(checks[1].label, "stories-ordered");
    assert_eq!(checks[1].state, ConveneCheckState::Gap);
    assert_eq!(checks[2].state, ConveneCheckState::Pending);
    assert_eq!(checks[3].state, ConveneCheckState::Pending);
}
