use crate::product_management::story::{BoardState, Stage};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnBucket {
    Icebox,
    Ready,
    InProgress,
    DeskCheck,
    Acceptance,
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
            ColumnBucket::Acceptance => "ACCEPTANCE",
            ColumnBucket::Accepted => "ACCEPTED",
            ColumnBucket::Retired => "RETIRED",
        }
    }
}

pub fn column_for(state: BoardState, is_closed: bool) -> ColumnBucket {
    if is_closed && state != BoardState::Accepted && state != BoardState::Retired {
        return ColumnBucket::Retired;
    }
    match state {
        BoardState::Icebox => ColumnBucket::Icebox,
        BoardState::Ready => ColumnBucket::Ready,
        BoardState::InProgress(stage) => match stage.as_stage() {
            Some(Stage::DeskCheck) => ColumnBucket::DeskCheck,
            Some(Stage::Dev) | None => ColumnBucket::InProgress,
        },
        BoardState::Acceptance => ColumnBucket::Acceptance,
        BoardState::Accepted => ColumnBucket::Accepted,
        BoardState::Retired => ColumnBucket::Retired,
    }
}
