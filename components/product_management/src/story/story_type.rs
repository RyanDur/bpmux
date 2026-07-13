use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StoryKind {
    #[default]
    Feature,
    Bug,
    Chore,
}

impl StoryKind {
    pub fn as_label(self) -> &'static str {
        match self {
            StoryKind::Feature => "feature",
            StoryKind::Bug => "bug",
            StoryKind::Chore => "chore",
        }
    }
}

pub fn story_type_from_title(title: &str) -> StoryKind {
    let lower = title.to_lowercase();
    story_kind_from_title(&lower)
}

fn story_kind_from_title(lower: &str) -> StoryKind {
    if lower.starts_with("[bug") || lower.starts_with("bug:") {
        return StoryKind::Bug;
    }
    if lower.starts_with("[chore")
        || lower.starts_with("[honest-plumbing")
        || lower.starts_with("[half-slice")
    {
        return StoryKind::Chore;
    }
    StoryKind::Feature
}
