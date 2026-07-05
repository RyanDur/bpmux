use crate::product_management::label_codec::LabelCodec;
use crate::product_management::story::{BoardState, Story};
use crate::view::board::StoryView;

pub fn story_to_board_json<C: LabelCodec>(
    codec: &C,
    story: &Story,
    position: &BoardState,
) -> serde_json::Value {
    serde_json::to_value(StoryView::project(codec, story, position)).unwrap_or_default()
}
