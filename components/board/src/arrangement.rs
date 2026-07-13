use std::collections::HashMap;

use product_management::iteration::Iteration;
use product_management::scheduling::{ProjectedState, projected_state};
use product_management::story::{Stage, Story};
use workflow::StageId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

pub fn column_of(story: &Story, fine: Option<StageId>, iterations: &[Iteration]) -> Column {
    match projected_state(story, iterations) {
        ProjectedState::Icebox => Column::Icebox,
        ProjectedState::Ready => Column::Ready,
        ProjectedState::InProgress => Column::Middle(fine.unwrap_or_else(|| Stage::Dev.id())),
        ProjectedState::Acceptance => Column::Acceptance,
        ProjectedState::Accepted => Column::Accepted,
        ProjectedState::Retired => Column::Retired,
    }
}

pub fn group_by_column<'a>(
    stories: impl IntoIterator<Item = (&'a Story, Option<StageId>)>,
    iterations: &[Iteration],
) -> HashMap<Column, Vec<&'a Story>> {
    let mut board: HashMap<Column, Vec<&Story>> = HashMap::new();
    for (story, fine) in stories {
        board
            .entry(column_of(story, fine, iterations))
            .or_default()
            .push(story);
    }
    board
}
