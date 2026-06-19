use super::label_sync::{
    sync_raw_labels_on_tag_added, sync_raw_labels_on_tag_removed, tag_from_label_string,
};
use super::*;
use crate::board::domain::{FreeComment, GateComment, Story};
use crate::board::events::BoardEvent;
use std::sync::atomic::Ordering;

impl BoardStore {
    pub fn apply(&self, event: &BoardEvent) {
        let _ = self.event_tx.send(event.clone());
        #[cfg(test)]
        {
            let hook = self
                .test_post_send_hook
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(f) = hook.as_ref() {
                f();
            }
        }
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        let number = event.story_number();
        match event {
            BoardEvent::StoryCreated {
                meta,
                number: n,
                title,
                body,
                initial_state,
                story_type,
            } => {
                let story = Story {
                    number: *n,
                    title: title.clone(),
                    state: *initial_state,
                    story_type: *story_type,
                    tags: Vec::new(),
                    filed_at: meta.ts,
                    filed_by: meta.actor.clone(),
                    body: Some(body.clone()),
                    free_comments: Some(Vec::new()),
                    gate_comments: Some(Vec::new()),
                    is_closed: false,
                    parent_epic: None,
                    acceptance_test_path: crate::bridge::parse_acceptance_test_path(body),
                    raw_labels: Vec::new(),
                    body_etag: None,
                    iteration_membership: None,
                    iteration_order: None,
                    iteration_review_closed_by: Vec::new(),
                    iteration_dev_ok_by: Vec::new(),
                    horizon: None,
                    outcome_membership: None,
                    is_current: false,
                    acceptor: None,
                };
                w.insert(*n, story);
            }
            BoardEvent::StateTransitioned { new_state, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.state = *new_state;
                }
            }
            BoardEvent::TagAdded { label, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    if let Some(tag) = tag_from_label_string(label)
                        && !story.tags.contains(&tag)
                    {
                        story.tags.push(tag);
                    }
                    sync_raw_labels_on_tag_added(story, label);
                    story.iteration_order =
                        crate::board::adapters::github::iteration_order_from_labels(
                            &story.raw_labels,
                        );
                    story.iteration_review_closed_by =
                        crate::board::adapters::github::iteration_review_closers_from_labels(
                            &story.raw_labels,
                        );
                    story.iteration_dev_ok_by =
                        crate::board::adapters::github::iteration_dev_ok_by_from_labels(
                            &story.raw_labels,
                        );
                    story.horizon =
                        crate::board::adapters::github::horizon_from_labels(&story.raw_labels);
                }
            }
            BoardEvent::TagRemoved { label, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    if let Some(tag) = tag_from_label_string(label) {
                        story.tags.retain(|t| *t != tag);
                    }
                    sync_raw_labels_on_tag_removed(story, label);
                    story.iteration_order =
                        crate::board::adapters::github::iteration_order_from_labels(
                            &story.raw_labels,
                        );
                    story.iteration_review_closed_by =
                        crate::board::adapters::github::iteration_review_closers_from_labels(
                            &story.raw_labels,
                        );
                    story.iteration_dev_ok_by =
                        crate::board::adapters::github::iteration_dev_ok_by_from_labels(
                            &story.raw_labels,
                        );
                    story.horizon =
                        crate::board::adapters::github::horizon_from_labels(&story.raw_labels);
                }
            }
            BoardEvent::GateCommentAppended {
                meta,
                gate,
                payload,
                ..
            } => {
                if let Some(story) = w.get_mut(&number) {
                    let entry = GateComment::from_event(*gate, meta, payload.clone());
                    match &mut story.gate_comments {
                        Some(list) => list.push(entry),
                        None => story.gate_comments = Some(vec![entry]),
                    }
                }
            }
            BoardEvent::FreeCommentAppended { meta, body, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    let entry = FreeComment::from_event(body.clone(), meta);
                    match &mut story.free_comments {
                        Some(list) => list.push(entry),
                        None => story.free_comments = Some(vec![entry]),
                    }
                }
            }
            BoardEvent::BodyFetched { body, etag, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.body = Some(body.clone());
                    story.acceptance_test_path = crate::bridge::parse_acceptance_test_path(body);
                    story.body_etag = etag.clone();
                }
            }
            BoardEvent::FreeCommentsFetched { comments, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.free_comments = Some(comments.clone());
                }
            }
            BoardEvent::GateCommentsFetched { comments, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.gate_comments = Some(comments.clone());
                }
            }
            BoardEvent::StoryClosed { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.is_closed = true;
                }
            }
            BoardEvent::StoryReopened { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.is_closed = false;
                }
            }
            BoardEvent::ParentEpicSet { epic_number, .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.parent_epic = Some(*epic_number);
                }
            }
            BoardEvent::ParentEpicUnset { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.parent_epic = None;
                }
            }
            BoardEvent::IterationMembershipSet {
                iteration_number,
                position,
                ..
            } => {
                if let Some(story) = w.get_mut(&number) {
                    story.iteration_membership = Some((*iteration_number, *position));
                }
            }
            BoardEvent::OutcomeMembershipSet {
                outcome_number,
                position,
                ..
            } => {
                if let Some(story) = w.get_mut(&number) {
                    story.outcome_membership = Some((*outcome_number, *position));
                }
            }
            BoardEvent::CurrentIterationDesignated { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.is_current = true;
                }
            }
            BoardEvent::CurrentIterationCleared { .. } => {
                if let Some(story) = w.get_mut(&number) {
                    story.is_current = false;
                }
            }
            BoardEvent::StoryRefreshed { story, .. } => {
                if let Some(existing) = w.get_mut(&number) {
                    existing.title = story.title.clone();
                    existing.state = story.state;
                    existing.story_type = story.story_type;
                    existing.tags = story.tags.clone();
                    existing.is_closed = story.is_closed;
                    existing.parent_epic = story.parent_epic;
                    existing.iteration_membership = story.iteration_membership;
                    existing.outcome_membership = story.outcome_membership;
                    existing.iteration_order = story.iteration_order;
                    existing.iteration_review_closed_by = story.iteration_review_closed_by.clone();
                    existing.iteration_dev_ok_by = story.iteration_dev_ok_by.clone();
                    existing.horizon = story.horizon;
                    existing.is_current = story.is_current;
                    existing.acceptor = story.acceptor;
                    if story.body.is_some() {
                        existing.body = story.body.clone();
                        existing.acceptance_test_path = story.acceptance_test_path.clone();
                        existing.body_etag = None;
                    }
                } else {
                    w.insert(number, (**story).clone());
                }
            }
            BoardEvent::LabelsRefreshed { labels, .. } => {
                if labels.is_empty() {
                } else if let Some(existing) = w.get_mut(&number) {
                    existing.state = crate::board::adapters::github::state_from_labels(
                        labels,
                        existing.is_closed,
                    );
                    existing.tags = labels
                        .iter()
                        .filter_map(|l| tag_from_label_string(l))
                        .collect();
                    existing.parent_epic =
                        crate::board::adapters::github::parent_epic_from_labels(labels);
                    existing.iteration_membership =
                        crate::board::adapters::github::iteration_membership_from_labels(labels);
                    existing.outcome_membership =
                        crate::board::adapters::github::outcome_membership_from_labels(labels);
                    existing.iteration_order =
                        crate::board::adapters::github::iteration_order_from_labels(labels);
                    existing.iteration_review_closed_by =
                        crate::board::adapters::github::iteration_review_closers_from_labels(
                            labels,
                        );
                    existing.iteration_dev_ok_by =
                        crate::board::adapters::github::iteration_dev_ok_by_from_labels(labels);
                    existing.horizon = crate::board::adapters::github::horizon_from_labels(labels);
                    existing.is_current =
                        crate::board::adapters::github::is_current_from_labels(labels);
                    existing.acceptor =
                        crate::board::adapters::github::acceptor_from_labels(labels);
                    existing.raw_labels = labels.clone();
                }
            }
            BoardEvent::ConflictDetected { .. } | BoardEvent::ConflictResolved { .. } => {}
        }
        drop(w);
        self.dirty.store(true, Ordering::Release);
    }
}
