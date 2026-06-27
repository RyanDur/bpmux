use crate::product_management::board::events::{BoardEvent, LabelDerivedFields};
use crate::product_management::story::Story;

#[derive(Clone)]
pub enum BoardAction {
    Event(BoardEvent),
    RawLabelsAssigned {
        number: u64,
        fields: LabelDerivedFields,
    },
    Seeded(Vec<Story>),
    Removed(u64),
    Cleared,
}
