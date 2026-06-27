use crate::product_management::board::events::BoardEvent;
use crate::product_management::story::Story;

#[derive(Clone)]
pub enum BoardAction {
    Event(BoardEvent),
    RawLabelsAssigned {
        number: u64,
        raw_labels: Vec<String>,
    },
    Seeded(Vec<Story>),
    Removed(u64),
    Cleared,
}
