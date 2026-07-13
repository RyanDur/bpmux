use crate::board::StoryView;
use product_management::label_codec::LabelCodec;
use product_management::story::Story;

pub fn story_to_board_json<C: LabelCodec>(
    codec: &C,
    story: &Story,
    state: &str,
) -> serde_json::Value {
    serde_json::to_value(StoryView::project(codec, story, state)).unwrap_or_default()
}
