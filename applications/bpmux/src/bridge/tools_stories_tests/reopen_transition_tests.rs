#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::transport::protocol::ClientMessage;

#[cfg(test)]
mod reopen_on_state_transition_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Stage, Story, StoryId};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn seeded_closed_chore(id: u64, state: BoardState) -> Story {
        let mut s = Story::new_minimal(
            StoryId::new(id),
            "t".into(),
            state,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        s.raw_labels.push("chore".to_string());
        s.is_closed = true;
        s
    }

    async fn build_orchestrator(
        seeded: Story,
    ) -> (
        Arc<BoardStore>,
        Arc<dyn RefreshDriver>,
        Arc<InMemorySourceAdapter>,
    ) {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(vec![seeded.clone()]);
        let orch_concrete = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter))
            .with_ci_state_store(super::test_ci_state_store_green());
        let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
        orch.refresh_all().await.unwrap();
        (store, orch, adapter)
    }

    #[tokio::test]
    async fn start_story_reopens_when_prior_is_closed_true() {
        let seeded = seeded_closed_chore(7110, BoardState::Ready);
        let (_store, orch, adapter) = build_orchestrator(seeded).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7110));
        args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
        args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
        let result = handle_start_story(&args, "dev:claude-code", Some(&orch)).await;
        assert!(result.is_ok(), "start_story should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        let pushed = adapter.pushed_events();
        assert!(
            pushed.iter().any(
                |e| matches!(e, BoardEvent::StoryReopened { id, .. } if *id == StoryId::new(7110))
            ),
            "start_story on closed-state must emit StoryReopened: {pushed:?}"
        );
    }

    #[tokio::test]
    async fn start_story_does_not_reopen_when_prior_is_closed_false() {
        let mut seeded = seeded_closed_chore(7111, BoardState::Ready);
        seeded.is_closed = false;
        let (_store, orch, adapter) = build_orchestrator(seeded).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7111));
        args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
        args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
        handle_start_story(&args, "dev:claude-code", Some(&orch))
            .await
            .unwrap();
        orch.drain_pending().await.unwrap();
        let pushed = adapter.pushed_events();
        assert!(
            !pushed.iter().any(
                |e| matches!(e, BoardEvent::StoryReopened { id, .. } if *id == StoryId::new(7111))
            ),
            "start_story on open story must NOT emit StoryReopened: {pushed:?}"
        );
    }

    #[tokio::test]
    async fn deliver_story_reopens_when_prior_is_closed_true() {
        let seeded = seeded_closed_chore(7120, BoardState::InProgress(Stage::Dev));
        let (_store, orch, adapter) = build_orchestrator(seeded).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7120));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("verified end-to-end"),
        );
        let result = handle_deliver_story(&args, "dev:claude-code", Some(&orch)).await;
        assert!(result.is_ok(), "deliver_story should succeed: {result:?}");
        let pushed = adapter.pushed_events();
        assert!(
            pushed.iter().any(
                |e| matches!(e, BoardEvent::StoryReopened { id, .. } if *id == StoryId::new(7120))
            ),
            "deliver_story on closed-state must emit StoryReopened: {pushed:?}"
        );
    }

    #[tokio::test]
    async fn pass_desk_check_reopens_when_prior_is_closed_true() {
        let seeded = seeded_closed_chore(7130, BoardState::InProgress(Stage::DeskCheck));
        let (_store, orch, adapter) = build_orchestrator(seeded).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7130));
        args.insert("acceptor".into(), serde_json::json!("owner"));
        args.insert("would_file_bug".into(), serde_json::json!(false));
        let result = handle_pass_desk_check(&args, "designer:claude-code", Some(&orch), None).await;
        assert!(result.is_ok(), "pass_desk_check should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        let pushed = adapter.pushed_events();
        assert!(
            pushed.iter().any(
                |e| matches!(e, BoardEvent::StoryReopened { id, .. } if *id == StoryId::new(7130))
            ),
            "pass_desk_check on closed-state must emit StoryReopened: {pushed:?}"
        );
    }
}
