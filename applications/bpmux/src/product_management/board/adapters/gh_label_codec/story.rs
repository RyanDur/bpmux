use super::{
    ACCEPTOR_DEV_LABEL, ACCEPTOR_OWNER_LABEL, ACCEPTOR_TEAM_LABEL, BLOCKED_BY_PREFIX,
    BLOCKED_LABEL, DESK_CHECK_FAIL_LABEL, GhLabelCodec, WORKFLOW_STATE_LABELS,
};
use crate::product_management::story::{Acceptor, StoryKind, Tag};

impl GhLabelCodec {
    pub fn decode_parent_epic(&self, label: &str) -> Option<u64> {
        label.strip_prefix("parent-epic-#")?.parse().ok()
    }

    pub fn decode_acceptor(&self, label: &str) -> Option<Acceptor> {
        match label {
            ACCEPTOR_TEAM_LABEL => Some(Acceptor::Team),
            ACCEPTOR_OWNER_LABEL => Some(Acceptor::Owner),
            ACCEPTOR_DEV_LABEL => Some(Acceptor::Dev),
            _ => None,
        }
    }

    pub fn encode_acceptor(&self, acceptor: Acceptor) -> &'static str {
        match acceptor {
            Acceptor::Team => ACCEPTOR_TEAM_LABEL,
            Acceptor::Owner => ACCEPTOR_OWNER_LABEL,
            Acceptor::Dev => ACCEPTOR_DEV_LABEL,
        }
    }

    pub fn decode_tag(&self, label: &str) -> Option<Tag> {
        if label.eq_ignore_ascii_case(BLOCKED_LABEL) {
            Some(Tag::Blocked)
        } else if label.eq_ignore_ascii_case(DESK_CHECK_FAIL_LABEL) {
            Some(Tag::DeskCheckFail)
        } else {
            None
        }
    }

    pub fn encode_tag(&self, tag: Tag) -> &'static str {
        match tag {
            Tag::Blocked => BLOCKED_LABEL,
            Tag::DeskCheckFail => DESK_CHECK_FAIL_LABEL,
        }
    }

    pub fn decode_blocked_by(&self, label: &str) -> Option<u64> {
        let raw = label.strip_prefix(BLOCKED_BY_PREFIX)?;
        let id: u64 = raw.parse().ok()?;
        if id == 0 {
            return None;
        }
        Some(id)
    }

    pub fn encode_blocked_by(&self, blocker: u64) -> String {
        format!("{BLOCKED_BY_PREFIX}{blocker}")
    }

    pub fn is_workflow_state_label(&self, label: &str) -> bool {
        WORKFLOW_STATE_LABELS
            .iter()
            .any(|s| label.eq_ignore_ascii_case(s))
    }

    pub fn decode_story_type(&self, label: &str) -> Option<StoryKind> {
        match label.to_ascii_lowercase().as_str() {
            "feature" => Some(StoryKind::Feature),
            "bug" => Some(StoryKind::Bug),
            "chore" => Some(StoryKind::Chore),
            _ => None,
        }
    }

    pub fn encode_story_type(&self, story_type: StoryKind) -> String {
        story_type.as_label().to_string()
    }
}
