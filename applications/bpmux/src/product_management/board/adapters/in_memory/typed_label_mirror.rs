use super::InnerState;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::story::{Acceptor, BoardState, Tag};

pub(super) fn tag_set(s: &mut InnerState, number: u64, tag: Tag) {
    let label = GhLabelCodec.encode_tag(tag);
    if let Some(story) = s.stories.get_mut(&number)
        && !story
            .raw_labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case(label))
    {
        story.raw_labels.push(label.to_string());
    }
}

pub(super) fn tag_unset(s: &mut InnerState, number: u64, tag: Tag) {
    let label = GhLabelCodec.encode_tag(tag);
    if let Some(story) = s.stories.get_mut(&number) {
        story.raw_labels.retain(|l| !l.eq_ignore_ascii_case(label));
    }
}

pub(super) fn blocked_by_set(s: &mut InnerState, number: u64, blocker: u64) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| GhLabelCodec.decode_blocked_by(l).is_none());
        story
            .raw_labels
            .push(GhLabelCodec.encode_blocked_by(blocker));
    }
}

pub(super) fn blocked_by_cleared(s: &mut InnerState, number: u64) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| GhLabelCodec.decode_blocked_by(l).is_none());
    }
}

pub(super) fn iteration_slot_set(s: &mut InnerState, number: u64, name: &str, position: u32) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| !GhLabelCodec.is_iteration_label_loose(l));
        story
            .raw_labels
            .push(GhLabelCodec.encode_iteration(name, position));
        story.iteration_slot = Some(crate::product_management::story::IterationSlot {
            name: name.to_string(),
            position,
        });
    }
}

pub(super) fn iteration_slot_cleared(s: &mut InnerState, number: u64) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| !GhLabelCodec.is_iteration_label_loose(l));
        story.iteration_slot = None;
    }
}

pub(super) fn acceptor_assigned(s: &mut InnerState, number: u64, acceptor: Acceptor) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| GhLabelCodec.decode_acceptor(l).is_none());
        story
            .raw_labels
            .push(GhLabelCodec.encode_acceptor(acceptor).to_string());
    }
}

pub(super) fn parent_epic_set(s: &mut InnerState, number: u64, epic: u64) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
        story.raw_labels.push(GhLabelCodec.encode_parent_epic(epic));
    }
}

pub(super) fn parent_epic_unset(s: &mut InnerState, number: u64) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
    }
}

pub(super) fn current_iteration_designated(s: &mut InnerState, number: u64) {
    if let Some(story) = s.stories.get_mut(&number)
        && !story
            .raw_labels
            .iter()
            .any(|l| GhLabelCodec.is_current_iteration_label(l))
    {
        story
            .raw_labels
            .push(GhLabelCodec.encode_current_iteration());
    }
}

pub(super) fn current_iteration_cleared(s: &mut InnerState, number: u64) {
    if let Some(story) = s.stories.get_mut(&number) {
        story
            .raw_labels
            .retain(|l| !GhLabelCodec.is_current_iteration_label(l));
    }
}

pub(super) fn state_transitioned(s: &mut InnerState, number: u64, new_state: BoardState) {
    let keep = new_state.github_label();
    let keep_or_other =
        |l: &String| !GhLabelCodec.is_workflow_state_label(l) || l.eq_ignore_ascii_case(keep);
    let labels = s.labels.entry(number).or_default();
    labels.retain(keep_or_other);
    if !labels.iter().any(|l| l.eq_ignore_ascii_case(keep)) {
        labels.push(keep.to_string());
    }
    if let Some(story) = s.stories.get_mut(&number) {
        story.raw_labels.retain(keep_or_other);
        if !story
            .raw_labels
            .iter()
            .any(|l| l.eq_ignore_ascii_case(keep))
        {
            story.raw_labels.push(keep.to_string());
        }
        story.state = new_state;
    }
}

pub(super) fn story_closed(s: &mut InnerState, number: u64) {
    let accepted = BoardState::Accepted.github_label();
    let keep_terminal =
        |l: &String| !GhLabelCodec.is_workflow_state_label(l) || l.eq_ignore_ascii_case(accepted);
    if let Some(labels) = s.labels.get_mut(&number) {
        labels.retain(keep_terminal);
    }
    if let Some(story) = s.stories.get_mut(&number) {
        story.raw_labels.retain(keep_terminal);
        story.is_closed = true;
    }
}
