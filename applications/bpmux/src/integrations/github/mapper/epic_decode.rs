use super::story_decode::{decode_story_metadata, extract_label_names};
use crate::product_management::epic::{Epic, EpicId};
use crate::product_management::story::Story;

pub fn is_epic_title(title: &str) -> bool {
    title.trim_start().to_ascii_lowercase().starts_with("[epic")
}

pub fn is_epic_issue(v: &serde_json::Value) -> bool {
    let has_label = extract_label_names(v)
        .iter()
        .any(|l| l.eq_ignore_ascii_case("epic"));
    let title_prefixed = v
        .get("title")
        .and_then(|x| x.as_str())
        .map(is_epic_title)
        .unwrap_or(false);
    has_label || title_prefixed
}

pub fn is_epic_story(story: &Story, labels: &[String]) -> bool {
    let has_label = labels.iter().any(|l| l.eq_ignore_ascii_case("epic"));
    has_label || is_epic_title(story.title())
}

pub fn decode_epic(v: &serde_json::Value) -> Option<Epic> {
    let is_closed = match v.get("gh_state").and_then(|x| x.as_str()) {
        Some(s) => matches!(s, "closed" | "CLOSED"),
        None => matches!(
            v.get("state").and_then(|x| x.as_str()),
            Some("closed" | "CLOSED")
        ),
    };
    decode_story_metadata(v).map(|story| decode_epic_from_story(&story, is_closed))
}

pub fn decode_epic_from_story(story: &Story, is_closed: bool) -> Epic {
    Epic {
        id: EpicId::new(story.id.value()),
        title: story.title().to_string(),
        body: story.body().map(str::to_string),
        filed_at: story.filed_at(),
        filed_by: story.filed_by().to_string(),
        state: story.state(),
        is_closed,
    }
}
