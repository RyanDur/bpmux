use crate::board::domain::{Story, StoryState};

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
    use crate::board::domain::StoryType;
    let retired_iterations: Vec<(&Story, String)> = stories
        .iter()
        .filter(|s| s.story_type == StoryType::Iteration && s.state == StoryState::Retired)
        .filter_map(|s| {
            s.title
                .strip_prefix("Iteration ")
                .map(|theme| (s, theme.to_string()))
        })
        .collect();
    if retired_iterations.is_empty() {
        return Vec::new();
    }
    retired_iterations
        .into_iter()
        .filter(|(_, theme)| {
            let prefix = format!("iteration-{theme}.");
            !stories
                .iter()
                .any(|s| s.raw_labels.iter().any(|l| l.starts_with(&prefix)))
        })
        .map(|(s, _)| s.number)
        .collect()
}

#[cfg(test)]
mod filter_retired_empty_iterations_tests {
    use super::*;
    use crate::board::domain::{Story, StoryState, StoryType};
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

    fn member_story(number: u64, theme: &str, position: u64) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("[Feature] member-{number}"),
            StoryState::Icebox,
            Utc::now(),
            "test".to_string(),
        );
        s.story_type = StoryType::Feature;
        s.raw_labels.push(format!("iteration-{theme}.{position}"));
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
            member_story(2, "Andromeda", 100),
            member_story(3, "Andromeda", 200),
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
            member_story(2, "Carina", 100),
        ];
        let filtered = filter_retired_empty_iterations(stories);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn closed_accepted_with_members_preserved() {
        let stories = vec![
            iter_story(1, "Pegasus", StoryState::Accepted),
            member_story(2, "Pegasus", 100),
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
            member_story(3, "WithKids", 100),
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
}
