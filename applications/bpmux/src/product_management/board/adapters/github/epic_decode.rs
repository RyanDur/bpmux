use super::story_decode::{decode_story_metadata, extract_label_names};
use crate::product_management::epic::{Epic, EpicId};
use crate::product_management::story::Story;

pub fn is_epic_issue(v: &serde_json::Value) -> bool {
    let has_label = extract_label_names(v)
        .iter()
        .any(|l| l.eq_ignore_ascii_case("epic"));
    let title_prefixed = v
        .get("title")
        .and_then(|x| x.as_str())
        .map(|t| t.trim_start().to_ascii_lowercase().starts_with("[epic"))
        .unwrap_or(false);
    has_label || title_prefixed
}

pub fn is_epic_story(story: &Story) -> bool {
    let has_label = story
        .raw_labels()
        .iter()
        .any(|l| l.eq_ignore_ascii_case("epic"));
    let title_prefixed = story
        .title()
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("[epic");
    has_label || title_prefixed
}

pub fn decode_epic(v: &serde_json::Value) -> Option<Epic> {
    decode_story_metadata(v).map(|story| decode_epic_from_story(&story))
}

pub fn decode_epic_from_story(story: &Story) -> Epic {
    Epic {
        id: EpicId::new(story.id.value()),
        title: story.title().to_string(),
        body: story.body().map(str::to_string),
        filed_at: story.filed_at(),
        filed_by: story.filed_by().to_string(),
        state: story.state(),
        is_closed: story.is_closed,
    }
}
