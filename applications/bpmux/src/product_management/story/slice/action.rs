use crate::product_management::board::events::{BoardEvent, LabelDerivedFields};
use crate::product_management::story::{Story, StoryId};

#[derive(Clone)]
pub enum StoryAction {
    Event(BoardEvent),
    RawLabelsAssigned {
        id: StoryId,
        fields: LabelDerivedFields,
    },
    Seeded(Vec<Story>),
    Removed(StoryId),
    Cleared,
}
