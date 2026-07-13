use crate::integrations::github::mapper::{
    decode_epic, decode_iteration, decode_outcome, decode_story_with_placement, is_epic_issue,
    is_iteration_issue, is_outcome_issue,
};
use product_management::epic::Epic;
use product_management::iteration::Iteration;
use product_management::outcome::Outcome;
use product_management::story::{Story, StoryId};
use workflow::StageId;

pub(super) struct Classified {
    pub stories: Vec<Story>,
    pub outcomes: Vec<Outcome>,
    pub iterations: Vec<Iteration>,
    pub epics: Vec<Epic>,
    pub placements: Vec<(StoryId, StageId)>,
    pub dropped: usize,
}

pub(super) fn classify_ingested(values: &[serde_json::Value]) -> Classified {
    let mut c = Classified {
        stories: Vec::with_capacity(values.len()),
        outcomes: Vec::new(),
        iterations: Vec::new(),
        epics: Vec::new(),
        placements: Vec::new(),
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
        } else if let Some((story, placement)) = decode_story_with_placement(v) {
            if let Some(stage) = placement {
                c.placements.push((story.id, stage));
            }
            c.stories.push(story);
        } else {
            c.dropped += 1;
        }
    }
    c
}
