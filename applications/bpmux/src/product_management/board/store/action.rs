use crate::product_management::board::events::{BoardEvent, LabelDerivedFields};
use crate::product_management::board::item::ProductItem;
use crate::product_management::story::StoryId;

#[derive(Clone)]
pub enum BoardAction {
    Event(BoardEvent),
    RawLabelsAssigned {
        id: StoryId,
        fields: LabelDerivedFields,
    },
    Seeded(Vec<ProductItem>),
    Removed(StoryId),
    Cleared,
}
