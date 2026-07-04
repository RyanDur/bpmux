use std::collections::HashMap;

use crate::product_management::story::{BoardState, Story};
use crate::workflow::StageId;

pub fn board_columns(middle: &[StageId]) -> Vec<BoardState> {
    let mut columns = vec![BoardState::Icebox, BoardState::Ready];
    columns.extend(middle.iter().cloned().map(BoardState::InProgress));
    columns.extend([
        BoardState::Acceptance,
        BoardState::Accepted,
        BoardState::Retired,
    ]);
    columns
}

pub fn board_position(state: &BoardState, is_closed: bool) -> BoardState {
    if is_closed && !matches!(state, BoardState::Accepted | BoardState::Retired) {
        return BoardState::Retired;
    }
    state.clone()
}

pub fn group_by_position<'a>(
    stories: impl IntoIterator<Item = &'a Story>,
) -> HashMap<BoardState, Vec<&'a Story>> {
    let mut board: HashMap<BoardState, Vec<&Story>> = HashMap::new();
    for story in stories {
        board
            .entry(board_position(&story.state, story.is_closed))
            .or_default()
            .push(story);
    }
    board
}

pub fn column_header(column: &BoardState) -> String {
    match column {
        BoardState::Icebox => "ICEBOX".to_string(),
        BoardState::Ready => "READY".to_string(),
        BoardState::InProgress(stage) => stage.as_str().to_ascii_uppercase().replace('-', " "),
        BoardState::Acceptance => "ACCEPTANCE".to_string(),
        BoardState::Accepted => "ACCEPTED".to_string(),
        BoardState::Retired => "RETIRED".to_string(),
    }
}
