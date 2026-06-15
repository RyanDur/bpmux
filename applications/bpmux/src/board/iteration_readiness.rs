use serde::Serialize;

use crate::board::domain::Story;
use crate::workflow::config::WorkflowConfig;

pub const ITERATION_REVIEWER_PURPOSE: &str = "iteration-reviewer";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct IterationReadiness {
    pub drafted: bool,
    pub stories_ordered: bool,
    pub review_closed: bool,
}

impl IterationReadiness {
    pub fn all_met(&self) -> bool {
        self.drafted && self.stories_ordered && self.review_closed
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
        let m1 = member(1, 200, 100);
        let r = iteration_readiness(&it, &[&m1], &bundled_default());
        assert!(
            r.all_met(),
            "body + ordered member + reviewer-close → all met"
        );
    }
}
