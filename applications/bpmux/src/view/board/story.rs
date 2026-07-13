use serde::Serialize;

use product_management::epic::EpicId;
use product_management::label_codec::LabelCodec;
use product_management::story::{Story, most_recent_needs_work};

#[derive(Debug, Clone, Serialize)]
pub struct StoryView {
    pub id: u64,
    pub title: String,
    pub state: String,
    pub labels: Vec<String>,
    pub parent_epic: Option<EpicId>,
    pub acceptor: Option<&'static str>,
    pub assignees: Vec<String>,
    pub created_at: String,
    pub body: String,
    pub rework_comment: Option<String>,
}

impl StoryView {
    pub fn project<C: LabelCodec>(codec: &C, story: &Story, state: &str) -> Self {
        Self {
            id: story.id.value(),
            title: story.title().to_string(),
            state: state.to_string(),
            labels: wire_labels(codec, story),
            parent_epic: story.parent_epic(),
            acceptor: story.acceptor().map(|a| a.as_str()),
            assignees: Vec::new(),
            created_at: story.filed_at().to_rfc3339(),
            body: story.body().unwrap_or_default().to_string(),
            rework_comment: rework_comment(story),
        }
    }
}

pub(crate) fn wire_labels<C: LabelCodec>(codec: &C, story: &Story) -> Vec<String> {
    let mut labels: Vec<String> = Vec::new();
    labels.push(story.story_type().as_label().to_string());
    for tag in story.tags() {
        labels.push(codec.encode_tag(*tag).to_string());
    }
    if let Some(acceptor) = story.acceptor() {
        labels.push(codec.encode_acceptor(acceptor).to_string());
    }
    if let Some(epic) = story.parent_epic() {
        labels.push(codec.encode_parent_epic(epic.value()));
    }
    labels
}

fn rework_comment(story: &Story) -> Option<String> {
    story
        .verdicts()
        .and_then(most_recent_needs_work)
        .and_then(|g| g.payload.clone())
}
