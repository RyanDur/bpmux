use crate::product_management::board::events::{LabelDerivedFields, StoryEvent};
use crate::product_management::story::{Story, StoryId};

#[derive(Clone)]
pub enum StoryAction {
    Event(StoryEvent),
    RawLabelsAssigned {
        id: StoryId,
        fields: LabelDerivedFields,
    },
    Seeded(Vec<Story>),
    Removed(StoryId),
    Cleared,
}
