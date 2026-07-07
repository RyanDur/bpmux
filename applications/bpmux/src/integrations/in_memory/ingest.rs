use std::collections::HashMap;

use crate::integrations::github::mapper::{
    decode_epic_from_story, decode_iteration_from_story, decode_outcome_from_story, is_epic_story,
    placement_from_labels,
};
use crate::product_management::epic::Epic;
use crate::product_management::iteration::Iteration;
use crate::product_management::outcome::Outcome;
use crate::product_management::story::{Story, StoryId};
use crate::sync::{Ingested, SyncError};
use crate::workflow::StageId;

pub(super) struct Classified {
    pub items: Vec<Story>,
    pub outcomes: Vec<Outcome>,
    pub iterations: Vec<Iteration>,
    pub epics: Vec<Epic>,
    pub placements: Vec<(StoryId, StageId)>,
}

pub(super) fn derive_typed_fields_from_labels(story: &mut Story, labels: &[String]) {
    let fields = crate::integrations::github::mapper::project_labels(labels);
    let labeled_type =
        crate::integrations::github::mapper::story_type_from_labels_or_title(labels, &story.title);
    if labeled_type != story.story_type
        && crate::product_management::story::story_type_from_title(&story.title) == story.story_type
    {
        story.story_type = labeled_type;
    }
    if story.tags.is_empty() {
        story.tags = fields.tags;
    }
    if story.acceptor.is_none() {
        story.acceptor = fields.acceptor;
    }
    if story.parent_epic.is_none() {
        story.parent_epic = fields.parent_epic;
    }
    if story.iteration_slot.is_none() {
        story.iteration_slot = fields.iteration_slot;
    }
}

pub(super) fn ingest_metadata(
    state: &std::sync::Mutex<super::InnerState>,
) -> Result<Ingested, SyncError> {
    let s = state.lock().unwrap_or_else(|e| e.into_inner());
    if s.failures_on_ingest {
        return Err(SyncError::SourceUnreachable(
            "in-memory test failure".into(),
        ));
    }
    let Classified {
        items,
        outcomes,
        mut iterations,
        epics,
        placements,
    } = classify_ingested(s.stories.values().cloned(), &s.labels, &s.closed);
    iterations.retain(|iteration| !(iteration.complete && iteration.stories.is_empty()));
    Ok(Ingested {
        items,
        outcomes,
        iterations,
        epics,
        placements,
    })
}

pub(super) fn classify_ingested(
    stories: impl Iterator<Item = Story>,
    labels: &HashMap<u64, Vec<String>>,
    closed: &std::collections::HashSet<u64>,
) -> Classified {
    let mut c = Classified {
        items: Vec::new(),
        outcomes: Vec::new(),
        iterations: Vec::new(),
        epics: Vec::new(),
        placements: Vec::new(),
    };
    for mut story in stories {
        let story_labels = labels.get(&story.id.value()).cloned().unwrap_or_default();
        derive_typed_fields_from_labels(&mut story, &story_labels);
        let is_closed = closed.contains(&story.id.value());
        let has_label = |needle: &str| story_labels.iter().any(|l| l.eq_ignore_ascii_case(needle));
        if has_label("outcome") {
            c.outcomes
                .push(decode_outcome_from_story(&story, &story_labels));
        } else if has_label("iteration") {
            c.iterations.push(decode_iteration_from_story(
                &story,
                &story_labels,
                is_closed,
            ));
        } else if is_epic_story(&story, &story_labels) {
            c.epics.push(decode_epic_from_story(&story, is_closed));
        } else {
            if let Some(stage) = placement_from_labels(&story_labels, is_closed) {
                c.placements.push((story.id, stage));
            }
            c.items.push(story);
        }
    }
    c
}
