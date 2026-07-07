use super::iteration::Iteration;
use super::iteration::label::current_iteration_id;
use super::story::{Story, StoryId, Work};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProjectedState {
    Icebox,
    Ready,
    InProgress,
    Acceptance,
    Accepted,
    Retired,
}

impl ProjectedState {
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectedState::Icebox => "icebox",
            ProjectedState::Ready => "ready",
            ProjectedState::InProgress => "in-progress",
            ProjectedState::Acceptance => "acceptance",
            ProjectedState::Accepted => "accepted",
            ProjectedState::Retired => "retired",
        }
    }

    pub fn github_label(&self) -> &'static str {
        match self {
            ProjectedState::Icebox => "icebox",
            ProjectedState::Ready => "ready",
            ProjectedState::InProgress => "in progress",
            ProjectedState::Acceptance => "delivered",
            ProjectedState::Accepted => "accepted",
            ProjectedState::Retired => "retired",
        }
    }
}

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

pub fn projected_state(story: &Story, iterations: &[Iteration]) -> ProjectedState {
    match &story.state {
        Work::NotStarted => {
            if in_current_iteration(story.id, iterations) {
                ProjectedState::Ready
            } else {
                ProjectedState::Icebox
            }
        }
        Work::InProgress => ProjectedState::InProgress,
        Work::Acceptance => ProjectedState::Acceptance,
        Work::Accepted => ProjectedState::Accepted,
        Work::Retired => ProjectedState::Retired,
    }
}
