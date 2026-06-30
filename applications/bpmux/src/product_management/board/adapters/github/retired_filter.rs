use crate::product_management::iteration::IterationId;
use crate::product_management::story::{Story, StoryState};

pub fn filter_retired_empty_iterations(stories: Vec<Story>) -> Vec<Story> {
    let purge: Vec<IterationId> = iterations_to_purge_as_retired_empty(&stories);
    if purge.is_empty() {
        return stories;
    }
    stories
        .into_iter()
        .filter(|s| !purge.contains(&IterationId::new(s.id.value())))
        .collect()
}

pub fn iterations_to_purge_as_retired_empty(stories: &[Story]) -> Vec<IterationId> {
    use crate::product_management::story::StoryType;
    stories
        .iter()
        .filter(|s| s.story_type() == StoryType::Iteration && s.state == StoryState::Retired)
        .map(|s| IterationId::new(s.id.value()))
        .filter(|iteration_id| {
            !stories.iter().any(|s| {
                s.iteration_membership()
                    .is_some_and(|(member_of, _)| member_of == *iteration_id)
            })
        })
        .collect()
}
