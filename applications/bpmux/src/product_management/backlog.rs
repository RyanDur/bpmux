use std::collections::HashMap;

use crate::product_management::iteration::Iteration;
use crate::product_management::story::{BoardState, Story, StoryId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BacklogColumn {
    Icebox,
    ReadyForWork,
    Current,
    Accepted,
}

pub fn backlog_column(story: &Story, iterations: &[Iteration]) -> Option<BacklogColumn> {
    match story.state.clone() {
        BoardState::Accepted => Some(BacklogColumn::Accepted),
        BoardState::Retired => None,
        on_board_position => {
            let on_board = matches!(
                on_board_position,
                BoardState::Ready | BoardState::InProgress | BoardState::Acceptance
            );
            if on_board && member_of_current_iteration(story.id, iterations) {
                Some(BacklogColumn::Current)
            } else if member_of_any_iteration(story.id, iterations) {
                Some(BacklogColumn::ReadyForWork)
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

fn member_of_current_iteration(id: StoryId, iterations: &[Iteration]) -> bool {
    iterations
        .iter()
        .filter(|iteration| iteration.is_current)
        .any(|iteration| iteration.stories.contains(&id))
}

fn member_of_any_iteration(id: StoryId, iterations: &[Iteration]) -> bool {
    iterations
        .iter()
        .any(|iteration| iteration.stories.contains(&id))
}
