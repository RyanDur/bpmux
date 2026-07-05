use std::collections::HashMap;

use crate::product_management::story::{BoardState, Stage, Story};
use crate::workflow::StageId;

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Column {
    Icebox,
    Ready,
    Middle(StageId),
    Acceptance,
    Accepted,
    Retired,
}

pub fn board_columns(middle: &[StageId]) -> Vec<Column> {
    let mut columns = vec![Column::Icebox, Column::Ready];
    columns.extend(middle.iter().cloned().map(Column::Middle));
    columns.extend([Column::Acceptance, Column::Accepted, Column::Retired]);
    columns
}

pub fn board_position(state: &BoardState, is_closed: bool) -> BoardState {
    if is_closed && !matches!(state, BoardState::Accepted | BoardState::Retired) {
        return BoardState::Retired;
    }
    state.clone()
}

pub fn column_of(story: &Story, fine: Option<StageId>) -> Column {
    match board_position(&story.state, story.is_closed) {
        BoardState::Icebox => Column::Icebox,
        BoardState::Ready => Column::Ready,
        BoardState::InProgress(_) => Column::Middle(fine.unwrap_or_else(|| Stage::Dev.id())),
        BoardState::Acceptance => Column::Acceptance,
        BoardState::Accepted => Column::Accepted,
        BoardState::Retired => Column::Retired,
    }
}

pub fn group_by_column<'a>(
    stories: impl IntoIterator<Item = (&'a Story, Option<StageId>)>,
) -> HashMap<Column, Vec<&'a Story>> {
    let mut board: HashMap<Column, Vec<&Story>> = HashMap::new();
    for (story, fine) in stories {
        board.entry(column_of(story, fine)).or_default().push(story);
    }
    board
}

pub fn column_header(column: &Column) -> String {
    match column {
        Column::Icebox => "ICEBOX".to_string(),
        Column::Ready => "READY".to_string(),
        Column::Middle(stage) => stage.as_str().to_ascii_uppercase().replace('-', " "),
        Column::Acceptance => "ACCEPTANCE".to_string(),
        Column::Accepted => "ACCEPTED".to_string(),
        Column::Retired => "RETIRED".to_string(),
    }
}
