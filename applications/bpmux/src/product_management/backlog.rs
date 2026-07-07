use std::collections::HashMap;

use crate::product_management::iteration::Iteration;
use crate::product_management::scheduling::{
    ProjectedState, in_any_iteration, in_current_iteration, projected_state,
};
use crate::product_management::story::Story;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BacklogColumn {
    Icebox,
    ReadyForDevelopment,
    Current,
    Accepted,
}

pub fn backlog_column(story: &Story, iterations: &[Iteration]) -> Option<BacklogColumn> {
    match projected_state(story, iterations) {
        ProjectedState::Accepted => Some(BacklogColumn::Accepted),
        ProjectedState::Retired => None,
        projected => {
            let on_board = matches!(
                projected,
                ProjectedState::Ready | ProjectedState::InProgress | ProjectedState::Acceptance
            );
            if on_board && in_current_iteration(story.id, iterations) {
                Some(BacklogColumn::Current)
            } else if in_any_iteration(story.id, iterations) {
                Some(BacklogColumn::ReadyForDevelopment)
            } else {
                Some(BacklogColumn::Icebox)
            }
        }
    }
}

pub fn group_by_backlog_column<'a>(
    stories: &'a [Story],
    iterations: &[Iteration],
) -> HashMap<BacklogColumn, Vec<&'a Story>> {
    let mut columns: HashMap<BacklogColumn, Vec<&Story>> = HashMap::new();
    for story in stories {
        if let Some(column) = backlog_column(story, iterations) {
            columns.entry(column).or_default().push(story);
        }
    }
    columns
}
