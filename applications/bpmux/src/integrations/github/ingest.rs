use crate::product_management::board::adapters::github::{
    decode_iteration, decode_outcome, decode_story_metadata, is_iteration_issue, is_outcome_issue,
};
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::Story;

pub(super) fn classify_ingested(
    values: &[serde_json::Value],
) -> (Vec<Story>, Vec<Outcome>, Vec<Iteration>, usize) {
    let mut stories = Vec::with_capacity(values.len());
    let mut outcomes = Vec::new();
    let mut iterations = Vec::new();
    let mut dropped = 0usize;
    for v in values.iter() {
        if is_outcome_issue(v) {
            match decode_outcome(v) {
                Some(outcome) => outcomes.push(outcome),
                None => dropped += 1,
            }
        } else if is_iteration_issue(v) {
            match decode_iteration(v) {
                Some(iteration) => iterations.push(iteration),
                None => dropped += 1,
            }
        } else if let Some(story) = decode_story_metadata(v) {
            stories.push(story);
        } else {
            dropped += 1;
        }
    }
    (stories, outcomes, iterations, dropped)
}
