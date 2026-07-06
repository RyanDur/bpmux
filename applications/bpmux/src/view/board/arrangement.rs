use std::collections::HashMap;

use crate::product_management::story::lifecycle::{Conclusion, Lifecycle, OnBoard};
use crate::product_management::story::{BoardState, Story};
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
    match Lifecycle::from_fields(&story.state, story.is_closed, fine, story.acceptor) {
        Lifecycle::Backlog => Column::Icebox,
        Lifecycle::OnBoard(OnBoard::Ready) => Column::Ready,
        Lifecycle::OnBoard(OnBoard::InProgress(stage)) => Column::Middle(stage),
        Lifecycle::OnBoard(OnBoard::Acceptance(_)) => Column::Acceptance,
        Lifecycle::Concluded(Conclusion::Accepted(_)) => Column::Accepted,
        Lifecycle::Concluded(Conclusion::Retired) => Column::Retired,
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
