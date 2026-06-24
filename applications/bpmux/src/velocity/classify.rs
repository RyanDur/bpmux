pub use crate::board::domain::StoryType;

pub fn classify_story(labels: &[String], title: &str) -> StoryType {
    for l in labels {
        match l.to_ascii_lowercase().as_str() {
            "feature" => return StoryType::Feature,
            "bug" => return StoryType::Bug,
            "chore" => return StoryType::Chore,
            "epic" => return StoryType::Epic,
            _ => {}
        }
    }
    let lower = title.to_lowercase();
    if lower.starts_with("[epic") {
        return StoryType::Epic;
    }
    if lower.starts_with("[feature")
        || lower.starts_with("[vertical")
        || lower.starts_with("[stranded-vertical")
    {
        return StoryType::Feature;
    }
    if lower.starts_with("[bug") || lower.starts_with("bug:") {
        return StoryType::Bug;
    }
    if lower.starts_with("[chore")
        || lower.starts_with("[honest-plumbing")
        || lower.starts_with("[half-slice")
    {
        return StoryType::Chore;
    }
    StoryType::Feature
}

pub fn is_feature_verdict(labels: &[String], title: &str) -> bool {
    matches!(classify_story(labels, title), StoryType::Feature)
}
