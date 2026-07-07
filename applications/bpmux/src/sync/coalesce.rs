use std::collections::VecDeque;

use crate::integrations::github::mapper::stage_github_label;
use crate::product_management::story::events::StoryEvent;
use crate::product_management::story::{BoardState, Stage, Story};

use super::{MutationId, QueuedEvent};

pub(super) fn iteration_prefix_class(
    codec: &dyn crate::product_management::label_codec::LabelCodec,
    label: &str,
) -> Option<String> {
    codec.iteration_label_class(label)
}

pub(super) fn coalesce_iteration_label_events(
    codec: &dyn crate::product_management::label_codec::LabelCodec,
    queue: &mut VecDeque<QueuedEvent>,
    new_event: &StoryEvent,
    new_id: MutationId,
    new_pre_state: &Option<Story>,
) -> bool {
    let (id, label, is_add) = match new_event {
        StoryEvent::TagAdded { id, label, .. } => (*id, label.clone(), true),
        StoryEvent::TagRemoved { id, label, .. } => (*id, label.clone(), false),
        _ => return false,
    };
    let Some(prefix) = iteration_prefix_class(codec, &label) else {
        return false;
    };
    let class_indices: Vec<usize> = queue
        .iter()
        .enumerate()
        .filter_map(|(i, q)| match &q.event {
            StoryEvent::TagAdded {
                id: n, label: l, ..
            }
            | StoryEvent::TagRemoved {
                id: n, label: l, ..
            } if *n == id
                && iteration_prefix_class(codec, l).as_deref() == Some(prefix.as_str()) =>
            {
                Some(i)
            }
            _ => None,
        })
        .collect();
    if class_indices.is_empty() {
        return false;
    }
    let mut first_r_idx: Option<usize> = None;
    let mut last_a_idx: Option<usize> = None;
    for &i in &class_indices {
        match &queue[i].event {
            StoryEvent::TagRemoved { .. } if first_r_idx.is_none() => first_r_idx = Some(i),
            StoryEvent::TagAdded { .. } => last_a_idx = Some(i),
            _ => {}
        }
    }
    let r_to_keep: Option<QueuedEvent>;
    let a_to_keep: Option<QueuedEvent>;
    if is_add {
        r_to_keep = first_r_idx.map(|i| queue[i].clone());
        a_to_keep = Some(QueuedEvent::new_with_id(
            new_id,
            new_event.clone(),
            new_pre_state.clone(),
        ));
    } else {
        r_to_keep = match first_r_idx {
            Some(i) => Some(queue[i].clone()),
            None => Some(QueuedEvent::new_with_id(
                new_id,
                new_event.clone(),
                new_pre_state.clone(),
            )),
        };
        a_to_keep = last_a_idx.map(|i| queue[i].clone());
    }
    let mut sorted = class_indices;
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    for i in sorted {
        queue.remove(i);
    }
    if let Some(r) = r_to_keep {
        queue.push_back(r);
    }
    if let Some(a) = a_to_keep {
        queue.push_back(a);
    }
    true
}

pub(super) fn stale_state_labels_for(
    story: &Story,
    raw_labels: &[String],
    fine: Option<Stage>,
    iterations: &[crate::product_management::iteration::Iteration],
) -> Vec<String> {
    let expected = match crate::product_management::scheduling::projected_state(story, iterations) {
        BoardState::Icebox => "",
        BoardState::InProgress => stage_github_label(fine.unwrap_or(Stage::Dev)),
        other => other.github_label(),
    };
    let canonical_state_labels = [
        BoardState::Ready.github_label(),
        stage_github_label(Stage::Dev),
        stage_github_label(Stage::DeskCheck),
        BoardState::Acceptance.github_label(),
        BoardState::Accepted.github_label(),
    ];
    let legacy_state_labels = crate::integrations::github::gh_label_codec::LEGACY_STATE_LABELS;
    raw_labels
        .iter()
        .filter(|l| {
            let lower = l.to_ascii_lowercase();
            let is_state_label = canonical_state_labels.contains(&lower.as_str())
                || legacy_state_labels.contains(&lower.as_str());
            is_state_label && lower != expected
        })
        .cloned()
        .collect()
}
