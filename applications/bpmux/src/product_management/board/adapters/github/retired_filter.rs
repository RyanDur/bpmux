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
        .filter(|s| s.story_type == StoryType::Iteration && s.state == StoryState::Retired)
        .map(|s| s.number)
        .filter(|iteration_number| {
            !stories.iter().any(|s| {
                s.iteration_membership
                    .is_some_and(|(member_of, _)| member_of == *iteration_number)
            })
        })
        .collect()
}

#[cfg(test)]
mod filter_retired_empty_iterations_tests {
    use super::*;
    use crate::product_management::story::{Story, StoryState, StoryType};
    use chrono::Utc;

    fn iter_story(number: u64, theme: &str, state: StoryState) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("Iteration {theme}"),
            state,
            Utc::now(),
            "test".to_string(),
        );
        s.story_type = StoryType::Iteration;
        s
    }

    fn member_story(number: u64, iteration_number: u64, position: u32) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("[Feature] member-{number}"),
            StoryState::Icebox,
            Utc::now(),
            "test".to_string(),
        );
        s.story_type = StoryType::Feature;
        s.iteration_membership = Some((iteration_number, position));
        s
    }

    #[test]
    fn retired_zero_member_iteration_filtered_out() {
        let stories = vec![iter_story(1, "Aquila", StoryState::Retired)];
        let filtered = filter_retired_empty_iterations(stories);
        assert!(
            filtered.is_empty(),
            "retired+0-member iteration must be filtered; got {filtered:?}"
        );
    }

    #[test]
    fn retired_with_members_preserved_history_value() {
        let stories = vec![
            iter_story(1, "Andromeda", StoryState::Retired),
            member_story(2, 1, 100),
            member_story(3, 1, 200),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 3, "closed+N kept; got {filtered:?}");
        assert!(filtered.iter().any(|s| s.number == 1));
    }

    #[test]
    fn open_zero_member_iteration_preserved_per_1413_fix() {
        let stories = vec![iter_story(1, "Lyra", StoryState::Icebox)];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 1, "open+0 stays per #1413 fix");
    }

    #[test]
    fn open_with_members_preserved() {
        let stories = vec![
            iter_story(1, "Carina", StoryState::Icebox),
            member_story(2, 1, 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn closed_accepted_with_members_preserved() {
        let stories = vec![
            iter_story(1, "Pegasus", StoryState::Accepted),
            member_story(2, 1, 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(
            filtered.len(),
            2,
            "Accepted (not Retired) iteration not subject to filter"
        );
    }

    #[test]
    fn mix_of_retired_empty_and_retired_with_members_filters_only_empty() {
        let stories = vec![
            iter_story(1, "Empty", StoryState::Retired),
            iter_story(2, "WithKids", StoryState::Retired),
            member_story(3, 2, 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|s| s.number == 2));
        assert!(filtered.iter().any(|s| s.number == 3));
        assert!(!filtered.iter().any(|s| s.number == 1));
    }

    #[test]
    fn non_iteration_retired_story_not_subject_to_filter() {
        let mut s = Story::new_minimal(
            1,
            "[Chore] retired-chore".to_string(),
            StoryState::Retired,
            Utc::now(),
            "test".to_string(),
        );
        s.story_type = StoryType::Chore;
        let stories = vec![s];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 1, "only iteration-type-class is filtered");
    }

    #[test]
    fn retired_iteration_with_canonical_members_not_purged_1556() {
        let stories = vec![
            iter_story(1400, "Carina", StoryState::Retired),
            member_story(200, 1400, 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert!(
            filtered.iter().any(|s| s.number == 1400),
            "retired iteration with a canonically-attributed member (no name-label) is NOT purged: {filtered:?}"
        );
        assert_eq!(
            filtered.len(),
            2,
            "iteration + its canonical member both preserved"
        );
    }
}
