use std::collections::VecDeque;

use crate::board::domain::{Story, StoryState};
use crate::board::events::BoardEvent;

use super::{MutationId, QueuedEvent};

pub(super) fn iteration_prefix_class(
    codec: &dyn crate::board::label_codec::LabelCodec,
    label: &str,
) -> Option<String> {
    codec.iteration_label_class(label)
}

pub(super) fn coalesce_iteration_label_events(
    codec: &dyn crate::board::label_codec::LabelCodec,
    queue: &mut VecDeque<QueuedEvent>,
    new_event: &BoardEvent,
    new_id: MutationId,
    new_pre_state: &Option<Story>,
) -> bool {
    let (number, label, is_add) = match new_event {
        BoardEvent::TagAdded { number, label, .. } => (*number, label.clone(), true),
        BoardEvent::TagRemoved { number, label, .. } => (*number, label.clone(), false),
        _ => return false,
    };
    let Some(prefix) = iteration_prefix_class(codec, &label) else {
        return false;
    };
    let class_indices: Vec<usize> = queue
        .iter()
        .enumerate()
        .filter_map(|(i, q)| match &q.event {
            BoardEvent::TagAdded {
                number: n,
                label: l,
                ..
            }
            | BoardEvent::TagRemoved {
                number: n,
                label: l,
                ..
            } if *n == number
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
            BoardEvent::TagRemoved { .. } if first_r_idx.is_none() => first_r_idx = Some(i),
            BoardEvent::TagAdded { .. } => last_a_idx = Some(i),
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

pub(super) fn stale_state_labels_for(story: &Story, raw_labels: &[String]) -> Vec<String> {
    let expected = match story.state {
        StoryState::Icebox => "",
        other => other.github_label(),
    };
    let canonical_state_labels = [
        StoryState::Ready.github_label(),
        StoryState::InProgress.github_label(),
        StoryState::DeskCheck.github_label(),
        StoryState::Delivered.github_label(),
        StoryState::Accepted.github_label(),
    ];
    let legacy_state_labels = ["ready for accept", "needs fixes", "rejected"];
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
