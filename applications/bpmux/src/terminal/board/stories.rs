use super::model::{BoardStory, StoryState};

/// Fetch all open + recently closed issues and categorize them into
/// board columns. Returns stories sorted by state priority.
///
/// #320: ported from `gh issue list` to octocrab. Async because
/// octocrab's client is async; callers in `event.rs` now
/// `tokio::spawn` instead of `spawn_blocking`. Auth via
/// `BPMUX_REPO` + `GH_TOKEN`/`GITHUB_TOKEN` (same env as the
/// background poller). Test fixture support via the env-var
/// BPMUX_BOARD_FIXTURE was retired in #724 — tests use httpmock
/// against the real GitHubSourceAdapter cascade instead, per
/// "only octocrab as test-double" directive.
pub async fn fetch_board_stories() -> Result<Vec<BoardStory>, String> {
    let (client, owner, repo) = crate::github::octocrab_client()?;
    let page = client
        .issues(&owner, &repo)
        .list()
        .state(octocrab::params::State::All)
        .per_page(100)
        .send()
        .await
        .map_err(|e| crate::github::classify_error(&e).user_message())?;
    let mut stories: Vec<BoardStory> = page
        .items
        .iter()
        .map(crate::terminal::board_poller::issue_to_story)
        .collect();
    sort_stories(&mut stories);
    Ok(stories)
}

/// Sort by column priority: active work (Ready, InProgress, Delivered)
/// first, then Accepted, Deferred, Icebox. The column order matches
/// what `stories_by_state` returns.
fn sort_stories(stories: &mut [BoardStory]) {
    stories.sort_by_key(|s| match s.state {
        StoryState::Ready => 0,
        StoryState::InProgress => 1,
        StoryState::Delivered => 2,
        StoryState::Deferred => 3,
        StoryState::Accepted => 4,
        StoryState::Icebox => 5,
    });
}

/// Group stories by state for the column layout, sorted by
/// PM-set priority order from `~/.bpmux/priority.json`.
pub fn stories_by_state(stories: &[BoardStory]) -> Vec<(StoryState, Vec<&BoardStory>)> {
    use crate::session::priority;
    use StoryState::*;

    let priority_index = priority::default_priority_path()
        .map(|p| priority::load_priority(&p))
        .unwrap_or_default();

    let states = [Ready, InProgress, Delivered, Accepted, Deferred, Icebox];
    states
        .iter()
        .filter_map(|&st| {
            let group: Vec<&BoardStory> = stories.iter().filter(|s| s.state == st).collect();
            if group.is_empty() {
                return None;
            }
            // Apply priority ordering
            let numbers: Vec<u64> = group.iter().map(|s| s.number).collect();
            let prio = priority_index
                .columns
                .get(st.priority_key())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let ordered = priority::sort_by_priority(&numbers, prio);
            let sorted: Vec<&BoardStory> = ordered
                .iter()
                .filter_map(|&n| group.iter().find(|s| s.number == n).copied())
                .collect();
            Some((st, sorted))
        })
        .collect()
}
