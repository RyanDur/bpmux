#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::iteration_readiness::*;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::workflow::bundled_default::bundled_default;
use crate::workflow::config::WorkflowConfig;
use chrono::Utc;

fn iteration(number: u64) -> Story {
    let mut s = Story::new_minimal(
        number,
        "Iteration Vela".to_string(),
        StoryState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    s.story_type = StoryType::Iteration;
    s.body = Some("## Iteration overview\n\ncomposed".to_string());
    s
}

fn member(number: u64, iteration_number: u64, position: u32) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("member #{number}"),
        StoryState::Ready,
        Utc::now(),
        "pm".to_string(),
    );
    s.iteration_membership = Some((iteration_number, position));
    s
}

#[test]
fn review_closed_true_when_a_configured_reviewer_closed() {
    let mut it = iteration(200);
    it.iteration_review_closed_by = vec!["designer".to_string()];
    let r = iteration_readiness(&it, &[], &bundled_default());
    assert!(
        r.review_closed,
        "designer is the configured iteration-reviewer; their close derives review_closed"
    );
}

#[test]
fn review_closed_false_when_closer_is_not_a_configured_reviewer() {
    let mut it = iteration(200);
    it.iteration_review_closed_by = vec!["pm".to_string()];
    let r = iteration_readiness(&it, &[], &bundled_default());
    assert!(
        !r.review_closed,
        "pm is not a configured iteration-reviewer; their label must not flip the check"
    );
}

#[test]
fn review_closed_false_when_nothing_on_record() {
    let it = iteration(200);
    let r = iteration_readiness(&it, &[], &bundled_default());
    assert!(
        !r.review_closed,
        "no review-close on record → no false-positive"
    );
}

#[test]
fn stories_ordered_true_when_members_carry_distinct_positions() {
    let it = iteration(200);
    let m1 = member(1, 200, 100);
    let m2 = member(2, 200, 200);
    let r = iteration_readiness(&it, &[&m1, &m2], &bundled_default());
    assert!(r.stories_ordered);
}

#[test]
fn stories_ordered_false_when_positions_collide() {
    let it = iteration(200);
    let m1 = member(1, 200, 100);
    let m2 = member(2, 200, 100);
    let r = iteration_readiness(&it, &[&m1, &m2], &bundled_default());
    assert!(
        !r.stories_ordered,
        "duplicate positions are not a total order"
    );
}

#[test]
fn stories_ordered_false_when_no_members() {
    let it = iteration(200);
    let r = iteration_readiness(&it, &[], &bundled_default());
    assert!(!r.stories_ordered);
}

#[test]
fn drafted_true_with_body_false_when_empty() {
    let mut it = iteration(200);
    assert!(iteration_readiness(&it, &[], &bundled_default()).drafted);
    it.body = Some("   ".to_string());
    assert!(!iteration_readiness(&it, &[], &bundled_default()).drafted);
    it.body = None;
    assert!(!iteration_readiness(&it, &[], &bundled_default()).drafted);
}

#[test]
fn all_met_requires_every_check() {
    let mut it = iteration(200);
    it.iteration_review_closed_by = vec!["designer".to_string()];
    it.iteration_dev_ok_by = vec!["dev".to_string()];
    let m1 = member(1, 200, 100);
    let r = iteration_readiness(&it, &[&m1], &bundled_default());
    assert!(
        r.all_met(),
        "body + ordered member + reviewer-close + dev-ok → all met"
    );
}

#[test]
fn all_met_false_when_dev_ok_missing() {
    let mut it = iteration(200);
    it.iteration_review_closed_by = vec!["designer".to_string()];
    let m1 = member(1, 200, 100);
    let r = iteration_readiness(&it, &[&m1], &bundled_default());
    assert!(
        !r.all_met(),
        "every check is required; a missing dev-OK keeps the iteration not-ready"
    );
}

#[test]
fn dev_ok_true_when_a_configured_builder_approved() {
    let mut it = iteration(200);
    it.iteration_dev_ok_by = vec!["dev".to_string()];
    assert!(
        iteration_readiness(&it, &[], &bundled_default()).dev_ok,
        "dev is the configured iteration-builder; their dev-OK derives dev_ok"
    );
}

#[test]
fn dev_ok_false_when_approver_is_not_a_configured_builder() {
    let mut it = iteration(200);
    it.iteration_dev_ok_by = vec!["pm".to_string()];
    assert!(
        !iteration_readiness(&it, &[], &bundled_default()).dev_ok,
        "pm is not a configured iteration-builder; their label must not flip the check"
    );
}

#[test]
fn dev_ok_false_when_nothing_on_record() {
    let it = iteration(200);
    assert!(
        !iteration_readiness(&it, &[], &bundled_default()).dev_ok,
        "no dev-OK on record → no false-positive"
    );
}

#[test]
fn dev_ok_is_config_driven_not_hardcoded_dev() {
    let workflow = workflow_with_builder_role("release-eng");
    let mut approved = iteration(200);
    approved.iteration_dev_ok_by = vec!["release-eng".to_string()];
    assert!(
        iteration_readiness(&approved, &[], &workflow).dev_ok,
        "the config-designated builder (release-eng, not dev) flips dev_ok"
    );
    let mut dev_approved = iteration(200);
    dev_approved.iteration_dev_ok_by = vec!["dev".to_string()];
    assert!(
        !iteration_readiness(&dev_approved, &[], &workflow).dev_ok,
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
    reviewed.iteration_review_closed_by = vec!["qa".to_string()];
    assert!(
        iteration_readiness(&reviewed, &[], &workflow).review_closed,
        "the config-designated reviewer (qa, not designer) flips review_closed"
    );
    let mut designer_closed = iteration(200);
    designer_closed.iteration_review_closed_by = vec!["designer".to_string()];
    assert!(
        !iteration_readiness(&designer_closed, &[], &workflow).review_closed,
        "designer is not a reviewer in this config; their close must not flip the check — proves no hard-coded designer"
    );
}

#[test]
fn owner_close_does_not_spoof_review_closed() {
    let mut it = iteration(200);
    it.iteration_review_closed_by = vec!["ryan".to_string()];
    assert!(
        !iteration_readiness(&it, &[], &bundled_default()).review_closed,
        "an owner-close is recorded but does not spoof review_closed — the derive validates the closer is a configured reviewer"
    );
}

fn building_iteration(number: u64, outcome_number: u64, position: u32) -> Story {
    let mut s = iteration(number);
    s.outcome_membership = Some((outcome_number, position));
    s
}

#[test]
fn outcome_readiness_rolls_up_gaps_from_building_iterations() {
    let outcome = iteration(700);
    let mut ready_it = building_iteration(710, 700, 1);
    ready_it.iteration_review_closed_by = vec!["designer".to_string()];
    ready_it.iteration_dev_ok_by = vec!["dev".to_string()];
    let ready_member = member(7101, 710, 100);
    let gappy_it = building_iteration(720, 700, 2);
    let gappy_member = member(7201, 720, 100);
    let all = vec![
        outcome.clone(),
        ready_it,
        ready_member,
        gappy_it,
        gappy_member,
    ];
    let rolled = outcome_readiness(&outcome, &all, &bundled_default());
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
    let outcome = iteration(700);
    let mut it = building_iteration(710, 700, 1);
    it.iteration_review_closed_by = vec!["designer".to_string()];
    it.iteration_dev_ok_by = vec!["dev".to_string()];
    let m = member(7101, 710, 100);
    let all = vec![outcome.clone(), it, m];
    let rolled = outcome_readiness(&outcome, &all, &bundled_default());
    assert_eq!(rolled.gap_count, 0);
    assert!(
        rolled.all_ready,
        "every building iteration met → the outcome's readiness rolls up to ready"
    );
}

#[test]
fn outcome_readiness_not_ready_with_no_building_iterations() {
    let outcome = iteration(700);
    let rolled = outcome_readiness(&outcome, std::slice::from_ref(&outcome), &bundled_default());
    assert_eq!(rolled.iteration_count, 0);
    assert!(
        !rolled.all_ready,
        "an outcome with no iterations building it is not ready (no false-positive)"
    );
}
