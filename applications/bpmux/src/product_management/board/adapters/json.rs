use crate::product_management::board::label_codec::LabelCodec;
use crate::product_management::board::view_model::StoryView;
use crate::product_management::story::Story;

pub fn story_to_board_json<C: LabelCodec>(codec: &C, story: &Story) -> serde_json::Value {
    serde_json::to_value(StoryView::project(codec, story)).unwrap_or_default()
}
