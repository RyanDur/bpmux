use serde::Serialize;

use crate::board::domain::Story;
use crate::workflow::config::WorkflowConfig;

pub const ITERATION_REVIEWER_PURPOSE: &str = "iteration-reviewer";
pub const ITERATION_BUILDER_PURPOSE: &str = "iteration-builder";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IterationReadiness {
    pub drafted: bool,
    pub stories_ordered: bool,
    pub review_closed: bool,
    pub dev_ok: bool,
}

impl IterationReadiness {
    pub fn all_met(&self) -> bool {
        self.drafted && self.stories_ordered && self.review_closed && self.dev_ok
    }
}

pub fn iteration_readiness(
    iteration: &Story,
    members: &[&Story],
    workflow: &WorkflowConfig,
) -> IterationReadiness {
    IterationReadiness {
        drafted: drafted(iteration),
        stories_ordered: stories_ordered(members),
        review_closed: review_closed(iteration, workflow),
        dev_ok: dev_ok(iteration, workflow),
    }
}

fn drafted(iteration: &Story) -> bool {
    iteration
        .body
        .as_deref()
        .is_some_and(|b| !b.trim().is_empty())
}

fn stories_ordered(members: &[&Story]) -> bool {
    if members.is_empty() {
        return false;
    }
    let positions: Vec<u32> = members
        .iter()
        .filter_map(|s| s.iteration_membership.map(|(_, position)| position))
        .collect();
    if positions.len() != members.len() {
        return false;
    }
    let mut distinct = positions.clone();
    distinct.sort_unstable();
    distinct.dedup();
    distinct.len() == positions.len()
}

fn review_closed(iteration: &Story, workflow: &WorkflowConfig) -> bool {
    let reviewers = workflow.role_ids_for_purpose(ITERATION_REVIEWER_PURPOSE);
    iteration
        .iteration_review_closed_by
        .iter()
        .any(|closer| reviewers.iter().any(|r| r == closer))
}

fn dev_ok(iteration: &Story, workflow: &WorkflowConfig) -> bool {
    let builders = workflow.role_ids_for_purpose(ITERATION_BUILDER_PURPOSE);
    iteration
        .iteration_dev_ok_by
        .iter()
        .any(|approver| builders.iter().any(|r| r == approver))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::board::domain::{Story, StoryState, StoryType};
    use crate::workflow::bundled_default::bundled_default;
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
}
