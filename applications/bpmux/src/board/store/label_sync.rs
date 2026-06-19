use super::super::domain::{Story, Tag};

pub(super) fn tag_from_label_string(s: &str) -> Option<Tag> {
    if s.eq_ignore_ascii_case("blocked") {
        Some(Tag::Blocked)
    } else {
        None
    }
}

pub(super) fn sync_raw_labels_on_tag_added(story: &mut Story, label: &str) {
    if !story.raw_labels.iter().any(|l| l == label) {
        story.raw_labels.push(label.to_string());
    }
    story.parent_epic = crate::board::adapters::github::parent_epic_from_labels(&story.raw_labels);
    story.iteration_membership =
        crate::board::adapters::github::iteration_membership_from_labels(&story.raw_labels);
    story.is_current = crate::board::adapters::github::is_current_from_labels(&story.raw_labels);
    story.acceptor = crate::board::adapters::github::acceptor_from_labels(&story.raw_labels);
}

pub(super) fn sync_raw_labels_on_tag_removed(story: &mut Story, label: &str) {
    story.raw_labels.retain(|l| l != label);
    story.parent_epic = crate::board::adapters::github::parent_epic_from_labels(&story.raw_labels);
    story.iteration_membership =
        crate::board::adapters::github::iteration_membership_from_labels(&story.raw_labels);
    story.is_current = crate::board::adapters::github::is_current_from_labels(&story.raw_labels);
    story.acceptor = crate::board::adapters::github::acceptor_from_labels(&story.raw_labels);
}
