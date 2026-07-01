use super::{
    ACCEPTOR_DEV_LABEL, ACCEPTOR_OWNER_LABEL, ACCEPTOR_TEAM_LABEL, BLOCKED_LABEL, GhLabelCodec,
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
        } else {
            None
        }
    }

    pub fn encode_tag(&self, tag: Tag) -> &'static str {
        match tag {
            Tag::Blocked => BLOCKED_LABEL,
        }
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
