use std::sync::Arc;

use serde::Serialize;

use crate::board::{BoardStore, StoryState};

#[derive(Serialize)]
struct DevInFlight {
    dev_in_flight: bool,
}

pub(in crate::bridge) async fn handle_dev_in_flight(
    store: &Arc<BoardStore>,
) -> Result<String, String> {
    let in_flight = !store.stories_in_state(StoryState::InProgress).is_empty();
    let response = DevInFlight {
        dev_in_flight: in_flight,
    };
    serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::board::Story;
    use chrono::{TimeZone, Utc};

    fn seed(state: StoryState, number: u64) -> Story {
        Story::new_minimal(
            number,
            format!("title-{number}"),
            state,
            Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            "pm".to_string(),
        )
    }

    #[tokio::test]
    async fn returns_false_when_no_story_in_flight() {
        let store = Arc::new(BoardStore::new());
        store.seed_all(vec![
            seed(StoryState::Ready, 1),
            seed(StoryState::Delivered, 2),
            seed(StoryState::Accepted, 3),
        ]);
        let result = handle_dev_in_flight(&store).await.unwrap();
        assert!(result.contains("\"dev_in_flight\":false"));
    }

    #[tokio::test]
    async fn returns_true_when_a_story_is_in_progress() {
        let store = Arc::new(BoardStore::new());
        store.seed_all(vec![
            seed(StoryState::Ready, 1),
            seed(StoryState::InProgress, 2),
            seed(StoryState::Delivered, 3),
        ]);
        let result = handle_dev_in_flight(&store).await.unwrap();
        assert!(result.contains("\"dev_in_flight\":true"));
    }

    #[tokio::test]
    async fn response_contains_no_story_identifiers() {
        let store = Arc::new(BoardStore::new());
        store.seed_all(vec![seed(StoryState::InProgress, 1)]);
        let result = handle_dev_in_flight(&store).await.unwrap();
        assert!(!result.contains("number"));
        assert!(!result.contains("title"));
    }

    #[tokio::test]
    async fn returns_false_on_empty_store() {
        let store = Arc::new(BoardStore::new());
        let result = handle_dev_in_flight(&store).await.unwrap();
        assert!(result.contains("\"dev_in_flight\":false"));
    }
}
