use super::InnerState;
use crate::integrations::github::gh_label_codec::GhLabelCodec;
use crate::product_management::label_codec::LabelCodec;
use crate::product_management::story::{Acceptor, BoardState, Tag};

pub(super) fn tag_set(s: &mut InnerState, number: u64, tag: Tag) {
    let label = GhLabelCodec.encode_tag(tag);
    let labels = s.labels.entry(number).or_default();
    if !labels.iter().any(|l| l.eq_ignore_ascii_case(label)) {
        labels.push(label.to_string());
    }
}

pub(super) fn tag_unset(s: &mut InnerState, number: u64, tag: Tag) {
    let label = GhLabelCodec.encode_tag(tag);
    s.labels
        .entry(number)
        .or_default()
        .retain(|l| !l.eq_ignore_ascii_case(label));
}

pub(super) fn iteration_slot_set(s: &mut InnerState, number: u64, name: &str, position: u32) {
    let labels = s.labels.entry(number).or_default();
    labels.retain(|l| !GhLabelCodec.is_iteration_label_loose(l));
    labels.push(GhLabelCodec.encode_iteration(name, position));
    if let Some(story) = s.stories.get_mut(&number) {
        story.iteration_slot = Some(crate::product_management::story::IterationSlot {
            name: name.to_string(),
            position,
        });
    }
}

pub(super) fn iteration_slot_cleared(s: &mut InnerState, number: u64) {
    s.labels
        .entry(number)
        .or_default()
        .retain(|l| !GhLabelCodec.is_iteration_label_loose(l));
    if let Some(story) = s.stories.get_mut(&number) {
        story.iteration_slot = None;
    }
}

pub(super) fn acceptor_assigned(s: &mut InnerState, number: u64, acceptor: Acceptor) {
    let labels = s.labels.entry(number).or_default();
    labels.retain(|l| GhLabelCodec.decode_acceptor(l).is_none());
    labels.push(GhLabelCodec.encode_acceptor(acceptor).to_string());
}

pub(super) fn parent_epic_set(s: &mut InnerState, number: u64, epic: u64) {
    let labels = s.labels.entry(number).or_default();
    labels.retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
    labels.push(GhLabelCodec.encode_parent_epic(epic));
}

pub(super) fn parent_epic_unset(s: &mut InnerState, number: u64) {
    s.labels
        .entry(number)
        .or_default()
        .retain(|l| GhLabelCodec.decode_parent_epic(l).is_none());
}

pub(super) fn current_iteration_designated(s: &mut InnerState, number: u64) {
    let labels = s.labels.entry(number).or_default();
    if !labels
        .iter()
        .any(|l| GhLabelCodec.is_current_iteration_label(l))
    {
        labels.push(GhLabelCodec.encode_current_iteration());
    }
}

pub(super) fn current_iteration_cleared(s: &mut InnerState, number: u64) {
    s.labels
        .entry(number)
        .or_default()
        .retain(|l| !GhLabelCodec.is_current_iteration_label(l));
}

pub(super) fn state_transitioned(
    s: &mut InnerState,
    number: u64,
    new_state: BoardState,
    placement: Option<crate::workflow::StageId>,
) {
    let keep =
        crate::integrations::github::mapper::outbound_state_label(&new_state, placement.as_ref());
    let keep_or_other =
        |l: &String| !GhLabelCodec.is_workflow_state_label(l) || l.eq_ignore_ascii_case(keep);
    let labels = s.labels.entry(number).or_default();
    labels.retain(keep_or_other);
    if !labels.iter().any(|l| l.eq_ignore_ascii_case(keep)) {
        labels.push(keep.to_string());
    }
    if let Some(story) = s.stories.get_mut(&number) {
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
        story.is_closed = true;
    }
}
