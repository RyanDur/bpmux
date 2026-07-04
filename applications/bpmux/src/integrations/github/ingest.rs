use crate::integrations::github::mapper::{
    decode_epic, decode_iteration, decode_outcome, decode_story_metadata, is_epic_issue,
    is_iteration_issue, is_outcome_issue,
};
use crate::product_management::epic::Epic;
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::Story;

pub(super) struct Classified {
    pub stories: Vec<Story>,
    pub outcomes: Vec<Outcome>,
    pub iterations: Vec<Iteration>,
    pub epics: Vec<Epic>,
    pub dropped: usize,
}

pub(super) fn classify_ingested(values: &[serde_json::Value]) -> Classified {
    let mut c = Classified {
        stories: Vec::with_capacity(values.len()),
        outcomes: Vec::new(),
        iterations: Vec::new(),
        epics: Vec::new(),
        dropped: 0,
    };
    for v in values.iter() {
        if is_outcome_issue(v) {
            match decode_outcome(v) {
                Some(outcome) => c.outcomes.push(outcome),
                None => c.dropped += 1,
            }
        } else if is_iteration_issue(v) {
            match decode_iteration(v) {
                Some(iteration) => c.iterations.push(iteration),
                None => c.dropped += 1,
            }
        } else if is_epic_issue(v) {
            match decode_epic(v) {
                Some(epic) => c.epics.push(epic),
                None => c.dropped += 1,
            }
        } else if let Some(story) = decode_story_metadata(v) {
            c.stories.push(story);
        } else {
            c.dropped += 1;
        }
    }
    c
}
