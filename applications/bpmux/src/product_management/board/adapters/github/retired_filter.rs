use crate::product_management::story::{Story, StoryState};

pub fn filter_retired_empty_iterations(stories: Vec<Story>) -> Vec<Story> {
    let purge: Vec<u64> = iteration_numbers_to_purge_as_retired_empty(&stories);
    if purge.is_empty() {
        return stories;
    }
    stories
        .into_iter()
        .filter(|s| !purge.contains(&s.number))
        .collect()
}

pub fn iteration_numbers_to_purge_as_retired_empty(stories: &[Story]) -> Vec<u64> {
    use crate::product_management::story::StoryType;
    stories
        .iter()
        .filter(|s| s.story_type() == StoryType::Iteration && s.state == StoryState::Retired)
        .map(|s| s.number)
        .filter(|iteration_number| {
            !stories.iter().any(|s| {
                super::iteration_membership_from_labels(&s.raw_labels)
                    .is_some_and(|(member_of, _)| member_of == *iteration_number)
            })
        })
        .collect()
}
