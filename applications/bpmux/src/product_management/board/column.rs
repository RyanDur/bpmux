use crate::product_management::story::{Stage, StoryState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnBucket {
    Icebox,
    Ready,
    InProgress,
    DeskCheck,
    Delivered,
    Accepted,
    Retired,
}

impl ColumnBucket {
    pub fn header(self) -> &'static str {
        match self {
            ColumnBucket::Icebox => "ICEBOX",
            ColumnBucket::Ready => "READY",
            ColumnBucket::InProgress => "IN PROGRESS",
            ColumnBucket::DeskCheck => "DESK CHECK",
            ColumnBucket::Delivered => "DELIVERED",
            ColumnBucket::Accepted => "ACCEPTED",
            ColumnBucket::Retired => "RETIRED",
        }
    }
}

pub fn column_for(state: StoryState, is_closed: bool) -> ColumnBucket {
    if is_closed && state != StoryState::Accepted && state != StoryState::Retired {
        return ColumnBucket::Retired;
    }
    match state {
        StoryState::Icebox => ColumnBucket::Icebox,
        StoryState::Ready => ColumnBucket::Ready,
        StoryState::InProgress(Stage::Dev) => ColumnBucket::InProgress,
        StoryState::InProgress(Stage::DeskCheck) => ColumnBucket::DeskCheck,
        StoryState::Delivered => ColumnBucket::Delivered,
        StoryState::Accepted => ColumnBucket::Accepted,
        StoryState::Retired => ColumnBucket::Retired,
    }
}
