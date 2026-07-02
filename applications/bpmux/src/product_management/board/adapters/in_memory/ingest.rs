use std::collections::HashMap;

use crate::product_management::board::adapters::github::{
    decode_epic_from_story, decode_iteration_from_story, decode_outcome_from_story, is_epic_story,
};
use crate::product_management::epic::Epic;
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::Story;

pub(super) struct Classified {
    pub items: Vec<Story>,
    pub outcomes: Vec<Outcome>,
    pub iterations: Vec<Iteration>,
    pub epics: Vec<Epic>,
}

pub(super) fn classify_ingested(
    stories: impl Iterator<Item = Story>,
    labels: &HashMap<u64, Vec<String>>,
) -> Classified {
    let mut c = Classified {
        items: Vec::new(),
        outcomes: Vec::new(),
        iterations: Vec::new(),
        epics: Vec::new(),
    };
    for mut story in stories {
        if let Some(labels) = labels.get(&story.id.value()) {
            story.raw_labels = labels.clone();
        }
        if story.iteration_slot.is_none()
            && let Some(slot) =
                crate::product_management::board::adapters::github::iteration_slot_from_labels(
                    &story.raw_labels,
                )
        {
            story.iteration_slot = Some(slot);
        }
        let has_label = |needle: &str| {
            story
                .raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case(needle))
        };
        if has_label("outcome") {
            c.outcomes.push(decode_outcome_from_story(&story));
        } else if has_label("iteration") {
            c.iterations.push(decode_iteration_from_story(&story));
        } else if is_epic_story(&story) {
            c.epics.push(decode_epic_from_story(&story));
        } else {
            c.items.push(story);
        }
    }
    c
}
