use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StoryKind {
    #[default]
    Feature,
    Bug,
    Chore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoryType {
    Story(StoryKind),
}

impl Default for StoryType {
    fn default() -> Self {
        StoryType::Story(StoryKind::Feature)
    }
}

impl StoryType {
    pub fn kind(self) -> Option<StoryKind> {
        match self {
            StoryType::Story(kind) => Some(kind),
        }
    }

    pub fn as_label(self) -> &'static str {
        match self {
            StoryType::Story(StoryKind::Feature) => "feature",
            StoryType::Story(StoryKind::Bug) => "bug",
            StoryType::Story(StoryKind::Chore) => "chore",
        }
    }

    fn as_serde_str(self) -> &'static str {
        match self {
            StoryType::Story(StoryKind::Feature) => "Feature",
            StoryType::Story(StoryKind::Bug) => "Bug",
            StoryType::Story(StoryKind::Chore) => "Chore",
        }
    }

    fn from_serde_str(raw: &str) -> Option<Self> {
        match raw {
            "Feature" => Some(StoryType::Story(StoryKind::Feature)),
            "Bug" => Some(StoryType::Story(StoryKind::Bug)),
            "Chore" => Some(StoryType::Story(StoryKind::Chore)),
            _ => None,
        }
    }
}

impl Serialize for StoryType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_serde_str())
    }
}

impl<'de> Deserialize<'de> for StoryType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        StoryType::from_serde_str(&raw)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown StoryType: {raw}")))
    }
}

pub fn story_type_from_title(title: &str) -> StoryType {
    let lower = title.to_lowercase();
    StoryType::Story(story_kind_from_title(&lower))
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
