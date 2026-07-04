use crate::product_management::story::BoardState;
use crate::workflow::StageId;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ColumnBucket {
    Icebox,
    Ready,
    Middle(StageId),
    Acceptance,
    Accepted,
    Retired,
}

impl ColumnBucket {
    pub fn header(&self) -> String {
        match self {
            ColumnBucket::Icebox => "ICEBOX".to_string(),
            ColumnBucket::Ready => "READY".to_string(),
            ColumnBucket::Middle(stage) => stage.as_str().to_ascii_uppercase().replace('-', " "),
            ColumnBucket::Acceptance => "ACCEPTANCE".to_string(),
            ColumnBucket::Accepted => "ACCEPTED".to_string(),
            ColumnBucket::Retired => "RETIRED".to_string(),
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
        BoardState::InProgress(stage) => ColumnBucket::Middle(stage),
        BoardState::Acceptance => ColumnBucket::Acceptance,
        BoardState::Accepted => ColumnBucket::Accepted,
        BoardState::Retired => ColumnBucket::Retired,
    }
}
