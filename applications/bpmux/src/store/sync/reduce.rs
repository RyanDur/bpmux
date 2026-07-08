use std::collections::VecDeque;

use crate::product_management::story::events::StoryEventKind;
use crate::sync::QueuedWrite;

use super::{SyncAction, SyncState};

pub fn reduce(state: &SyncState, action: &SyncAction) -> Option<SyncState> {
    match action {
        SyncAction::WriteQueued { write } => {
            let mut queue = state.queue.clone();
            absorb_or_push(&mut queue, write.clone());
            Some(SyncState { queue })
        }
        SyncAction::WritesQueued(writes) => {
            if writes.is_empty() {
                return None;
            }
            let mut queue = state.queue.clone();
            for write in writes {
                absorb_or_push(&mut queue, write.clone());
            }
            Some(SyncState { queue })
        }
        SyncAction::WritePushed { id } | SyncAction::WriteDropped { id } => {
            if !state.queue.iter().any(|write| write.id == *id) {
                return None;
            }
            let queue = state
                .queue
                .iter()
                .filter(|write| write.id != *id)
                .cloned()
                .collect();
            Some(SyncState { queue })
        }
        SyncAction::StartIngestPolling
        | SyncAction::StopIngestPolling
        | SyncAction::StartHydrationPolling
        | SyncAction::StopHydrationPolling => None,
    }
}

fn absorb_or_push(queue: &mut VecDeque<QueuedWrite>, write: QueuedWrite) {
    if !coalesce_iteration_label_write(queue, &write) {
        queue.push_back(write);
    }
}

fn coalesce_iteration_label_write(queue: &mut VecDeque<QueuedWrite>, new: &QueuedWrite) -> bool {
    let (story_id, is_add) = match &new.event.kind {
        StoryEventKind::TagAdded { .. } => (new.event.id, true),
        StoryEventKind::TagRemoved { .. } => (new.event.id, false),
        _ => return false,
    };
    let Some(prefix) = new.iteration_class.as_deref() else {
        return false;
    };
    let class_indices: Vec<usize> = queue
        .iter()
        .enumerate()
        .filter_map(|(i, q)| match &q.event.kind {
            StoryEventKind::TagAdded { .. } | StoryEventKind::TagRemoved { .. }
                if q.event.id == story_id && q.iteration_class.as_deref() == Some(prefix) =>
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
        match &queue[i].event.kind {
            StoryEventKind::TagRemoved { .. } if first_r_idx.is_none() => first_r_idx = Some(i),
            StoryEventKind::TagAdded { .. } => last_a_idx = Some(i),
            _ => {}
        }
    }
    let r_to_keep: Option<QueuedWrite>;
    let a_to_keep: Option<QueuedWrite>;
    if is_add {
        r_to_keep = first_r_idx.map(|i| queue[i].clone());
        a_to_keep = Some(new.clone());
    } else {
        r_to_keep = match first_r_idx {
            Some(i) => Some(queue[i].clone()),
            None => Some(new.clone()),
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
