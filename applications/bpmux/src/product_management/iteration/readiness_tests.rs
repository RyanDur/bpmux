#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::readiness::*;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::github::{
    decode_iteration_from_story, decode_outcome_from_story,
};
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, OutcomeRelationships, Relationships,
};
use crate::product_management::iteration::IterationId;
use crate::product_management::outcome::{Outcome, OutcomeId};
use crate::product_management::story::{BoardState, Story, StoryId};
use crate::workflow::bundled_default::bundled_default;
use crate::workflow::config::WorkflowConfig;
use chrono::Utc;

fn readiness(seed: &(Story, Vec<String>), workflow: &WorkflowConfig) -> IterationReadiness {
    let (story, labels) = seed;
    iteration_readiness(&decode_iteration_from_story(story, labels), workflow)
}

fn decode_iterations(
    seeds: &[(Story, Vec<String>)],
) -> Vec<crate::product_management::iteration::Iteration> {
    seeds
        .iter()
        .filter(|(_, labels)| labels.iter().any(|l| l.eq_ignore_ascii_case("iteration")))
        .map(|(story, labels)| decode_iteration_from_story(story, labels))
        .collect()
}

fn iteration(id: u64) -> (Story, Vec<String>) {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        "Iteration Vela".to_string(),
        BoardState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    s.body = Some("## Iteration overview\n\ncomposed".to_string());
    (s, vec!["iteration".to_string()])
}

fn iteration_with_stories(id: u64, stories: Vec<u64>) -> (Story, Vec<String>) {
    let (mut s, labels) = iteration(id);
    s.body = Some(
        IterationRelationships {
            outcome: None,
            stories: stories.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body(s.body.as_deref().unwrap_or_default()),
    );
    (s, labels)
}

#[test]
fn review_closed_true_when_a_configured_reviewer_closed() {
    let mut it = iteration(200);
    it.1.push(GhLabelCodec.encode_iteration_review_closed("designer"));
    let r = readiness(&it, &bundled_default());
    assert!(
        r.review_closed,
        "designer is the configured iteration-reviewer; their close derives review_closed"
    );
}

#[test]
fn review_closed_false_when_closer_is_not_a_configured_reviewer() {
    let mut it = iteration(200);
    it.1.push(GhLabelCodec.encode_iteration_review_closed("pm"));
    let r = readiness(&it, &bundled_default());
    assert!(
        !r.review_closed,
        "pm is not a configured iteration-reviewer; their label must not flip the check"
    );
}

#[test]
fn review_closed_false_when_nothing_on_record() {
    let it = iteration(200);
    let r = readiness(&it, &bundled_default());
    assert!(
        !r.review_closed,
        "no review-close on record → no false-positive"
    );
}

#[test]
fn stories_ordered_true_when_the_iteration_has_members() {
    let it = iteration_with_stories(200, vec![1, 2]);
    assert!(readiness(&it, &bundled_default()).stories_ordered);
}

#[test]
fn stories_ordered_false_when_no_members() {
    let it = iteration(200);
    let r = readiness(&it, &bundled_default());
    assert!(!r.stories_ordered);
}

#[test]
fn drafted_true_with_body_false_when_empty() {
    let mut it = iteration(200);
    assert!(readiness(&it, &bundled_default()).drafted);
    it.0.body = Some("   ".to_string());
    assert!(!readiness(&it, &bundled_default()).drafted);
    it.0.body = None;
    assert!(!readiness(&it, &bundled_default()).drafted);
}

#[test]
fn all_met_requires_every_check() {
    let mut it = iteration_with_stories(200, vec![1]);
    it.1.push(GhLabelCodec.encode_iteration_review_closed("designer"));
    it.1.push(GhLabelCodec.encode_iteration_dev_ok("dev"));
    let r = readiness(&it, &bundled_default());
    assert!(
        r.all_met(),
        "body + ordered member + reviewer-close + dev-ok → all met"
    );
}

#[test]
fn all_met_false_when_dev_ok_missing() {
    let mut it = iteration_with_stories(200, vec![1]);
    it.1.push(GhLabelCodec.encode_iteration_review_closed("designer"));
    let r = readiness(&it, &bundled_default());
    assert!(
        !r.all_met(),
        "every check is required; a missing dev-OK keeps the iteration not-ready"
    );
}

#[test]
fn dev_ok_true_when_a_configured_builder_approved() {
    let mut it = iteration(200);
    it.1.push(GhLabelCodec.encode_iteration_dev_ok("dev"));
    assert!(
        readiness(&it, &bundled_default()).dev_ok,
        "dev is the configured iteration-builder; their dev-OK derives dev_ok"
    );
}

#[test]
fn dev_ok_false_when_approver_is_not_a_configured_builder() {
    let mut it = iteration(200);
    it.1.push(GhLabelCodec.encode_iteration_dev_ok("pm"));
    assert!(
        !readiness(&it, &bundled_default()).dev_ok,
        "pm is not a configured iteration-builder; their label must not flip the check"
    );
}

#[test]
fn dev_ok_false_when_nothing_on_record() {
    let it = iteration(200);
    assert!(
        !readiness(&it, &bundled_default()).dev_ok,
        "no dev-OK on record → no false-positive"
    );
}

#[test]
fn dev_ok_is_config_driven_not_hardcoded_dev() {
    let workflow = workflow_with_builder_role("release-eng");
    let mut approved = iteration(200);
    approved
        .1
        .push(GhLabelCodec.encode_iteration_dev_ok("release-eng"));
    assert!(
        readiness(&approved, &workflow).dev_ok,
        "the config-designated builder (release-eng, not dev) flips dev_ok"
    );
    let mut dev_approved = iteration(200);
    dev_approved
        .1
        .push(GhLabelCodec.encode_iteration_dev_ok("dev"));
    assert!(
        !readiness(&dev_approved, &workflow).dev_ok,
        "dev is not a builder in this config; their label must not flip the check — proves no hard-coded dev"
    );
}

fn workflow_with_reviewer_role(reviewer_id: &str) -> WorkflowConfig {
    WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![],
        role: vec![
            crate::workflow::config::Role {
                id: "designer".into(),
                purposes: vec!["designer".into()],
            },
            crate::workflow::config::Role {
                id: reviewer_id.into(),
                purposes: vec![ITERATION_REVIEWER_PURPOSE.into()],
            },
        ],
    }
}

fn workflow_with_builder_role(builder_id: &str) -> WorkflowConfig {
    WorkflowConfig {
        section: vec![],
        state: vec![],
        transition: vec![],
        role: vec![
            crate::workflow::config::Role {
                id: "dev".into(),
                purposes: vec!["dev".into()],
            },
            crate::workflow::config::Role {
                id: builder_id.into(),
                purposes: vec![ITERATION_BUILDER_PURPOSE.into()],
            },
        ],
    }
}

#[test]
fn review_closed_is_config_driven_not_hardcoded_designer() {
    let workflow = workflow_with_reviewer_role("qa");
    let mut reviewed = iteration(200);
    reviewed
        .1
        .push(GhLabelCodec.encode_iteration_review_closed("qa"));
    assert!(
        readiness(&reviewed, &workflow).review_closed,
        "the config-designated reviewer (qa, not designer) flips review_closed"
    );
    let mut designer_closed = iteration(200);
    designer_closed
        .1
        .push(GhLabelCodec.encode_iteration_review_closed("designer"));
    assert!(
        !readiness(&designer_closed, &workflow).review_closed,
        "designer is not a reviewer in this config; their close must not flip the check — proves no hard-coded designer"
    );
}

#[test]
fn owner_close_does_not_spoof_review_closed() {
    let mut it = iteration(200);
    it.1.push(GhLabelCodec.encode_iteration_review_closed("owner"));
    assert!(
        !readiness(&it, &bundled_default()).review_closed,
        "an owner-close is recorded but does not spoof review_closed — the derive validates the closer is a configured reviewer"
    );
}

fn building_iteration(id: u64, outcome_id: u64, stories: Vec<u64>) -> (Story, Vec<String>) {
    let (mut s, labels) = iteration(id);
    s.body = Some(
        IterationRelationships {
            outcome: Some(OutcomeId::new(outcome_id)),
            stories: stories.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body(s.body.as_deref().unwrap_or_default()),
    );
    (s, labels)
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

#[test]
fn outcome_readiness_rolls_up_gaps_from_building_iterations() {
    let outcome = outcome_with(700, vec![710, 720]);
    let mut ready_it = building_iteration(710, 700, vec![7101]);
    ready_it
        .1
        .push(GhLabelCodec.encode_iteration_review_closed("designer"));
    ready_it.1.push(GhLabelCodec.encode_iteration_dev_ok("dev"));
    let gappy_it = building_iteration(720, 700, vec![7201]);
    let all = vec![ready_it, gappy_it];
    let rolled = outcome_readiness(&outcome, &decode_iterations(&all), &bundled_default());
    assert_eq!(
        rolled.iteration_count, 2,
        "two iterations build this outcome"
    );
    assert_eq!(
        rolled.gap_count, 2,
        "the second iteration lacks review-close + dev-OK = exactly 2 gaps rolled up"
    );
    assert!(
        !rolled.all_ready,
        "an outcome is not ready while a building iteration has gaps"
    );
}

#[test]
fn outcome_readiness_all_ready_when_every_building_iteration_is_met() {
    let outcome = outcome_with(700, vec![710]);
    let mut it = building_iteration(710, 700, vec![7101]);
    it.1.push(GhLabelCodec.encode_iteration_review_closed("designer"));
    it.1.push(GhLabelCodec.encode_iteration_dev_ok("dev"));
    let all = vec![it];
    let rolled = outcome_readiness(&outcome, &decode_iterations(&all), &bundled_default());
    assert_eq!(rolled.gap_count, 0);
    assert!(
        rolled.all_ready,
        "every building iteration met → the outcome's readiness rolls up to ready"
    );
}

#[test]
fn outcome_readiness_not_ready_with_no_building_iterations() {
    let outcome = outcome_with(700, vec![]);
    let rolled = outcome_readiness(&outcome, &[], &bundled_default());
    assert_eq!(rolled.iteration_count, 0);
    assert!(
        !rolled.all_ready,
        "an outcome with no iterations building it is not ready (no false-positive)"
    );
}
