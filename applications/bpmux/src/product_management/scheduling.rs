use super::iteration::Iteration;
use super::iteration::label::current_iteration_id;
use super::story::{BoardState, Story, StoryId};

pub fn in_current_iteration(id: StoryId, iterations: &[Iteration]) -> bool {
    match current_iteration_id(iterations) {
        Some(current) => iterations
            .iter()
            .any(|it| it.id == current && it.stories.contains(&id)),
        None => false,
    }
}

pub fn in_any_iteration(id: StoryId, iterations: &[Iteration]) -> bool {
    iterations.iter().any(|it| it.stories.contains(&id))
}

pub fn projected_state(story: &Story, iterations: &[Iteration]) -> BoardState {
    match &story.state {
        BoardState::Icebox | BoardState::Ready => {
            if in_current_iteration(story.id, iterations) {
                BoardState::Ready
            } else {
                BoardState::Icebox
            }
        }
        other => other.clone(),
    }
}
