#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use protocol::ClientMessage;

#[cfg(test)]
mod reopen_on_state_transition_tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Stage, Story, StoryId, Work};
    use std::sync::Arc;

    fn seeded_chore(id: u64, state: Work) -> (Story, Vec<String>) {
        let s = Story::new_minimal(
            StoryId::new(id),
            "t".into(),
            state,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        (s, vec!["chore".to_string()])
    }

    async fn build_orchestrator(
        seeded: (Story, Vec<String>),
    ) -> (
        crate::store::SeededStore,
        Arc<dyn RefreshDriver>,
        Arc<InMemorySourceAdapter>,
        crate::store::Select,
    ) {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let (story, labels) = seeded;
        adapter.seed_labels(story.id.value(), labels);
        adapter.seed_stories(vec![story]);
        let orch_concrete = SyncOrchestrator::new(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        )
        .with_ci_state_store(super::test_ci_state_store_green());
        let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
        let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
        store.bind_refresh(&orch);
        orch.refresh_all().await.unwrap();
        let select = store.select();
        (store, orch, adapter, select)
    }

    #[tokio::test]
    async fn start_story_reopens_when_prior_is_concluded() {
        let seeded = seeded_chore(7110, Work::Accepted);
        let (_store, orch, adapter, select) = build_orchestrator(seeded).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7110));
        args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
        args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
        let result =
            handle_start_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select).await;
        assert!(result.is_ok(), "start_story should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        let pushed = adapter.pushed_events();
        assert!(
            pushed.iter().any(
                |e| matches!(&e.kind, StoryEventKind::StoryReopened if e.id == StoryId::new(7110))
            ),
            "start_story on a concluded story must emit StoryReopened: {pushed:?}"
        );
    }

    #[tokio::test]
    async fn start_story_does_not_reopen_when_prior_is_active() {
        let (story, labels) = seeded_chore(7111, Work::NotStarted);
        let (_store, orch, adapter, select) = build_orchestrator((story, labels)).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7111));
        args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
        args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
        handle_start_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select)
            .await
            .unwrap();
        orch.drain_pending().await.unwrap();
        let pushed = adapter.pushed_events();
        assert!(
            !pushed.iter().any(
                |e| matches!(&e.kind, StoryEventKind::StoryReopened if e.id == StoryId::new(7111))
            ),
            "start_story on open story must NOT emit StoryReopened: {pushed:?}"
        );
    }

    #[tokio::test]
    async fn deliver_story_reopens_when_prior_is_concluded() {
        let seeded = seeded_chore(7120, Work::Accepted);
        let (_store, orch, adapter, select) = build_orchestrator(seeded).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7120));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("verified end-to-end"),
        );
        let result =
            handle_deliver_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select).await;
        assert!(result.is_ok(), "deliver_story should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        let pushed = adapter.pushed_events();
        assert!(
            pushed.iter().any(
                |e| matches!(&e.kind, StoryEventKind::StoryReopened if e.id == StoryId::new(7120))
            ),
            "deliver_story on a concluded story must emit StoryReopened: {pushed:?}"
        );
    }

    #[tokio::test]
    async fn pass_desk_check_reopens_when_prior_is_concluded() {
        let seeded = seeded_chore(7130, Work::Accepted);
        let (_store, orch, adapter, select) = build_orchestrator(seeded).await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7130));
        args.insert("acceptor".into(), serde_json::json!("owner"));
        args.insert("would_file_bug".into(), serde_json::json!(false));
        let result = handle_pass_desk_check(
            &args,
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
            None,
        )
        .await;
        assert!(result.is_ok(), "pass_desk_check should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        let pushed = adapter.pushed_events();
        assert!(
            pushed.iter().any(
                |e| matches!(&e.kind, StoryEventKind::StoryReopened if e.id == StoryId::new(7130))
            ),
            "pass_desk_check on a concluded story must emit StoryReopened: {pushed:?}"
        );
    }

    #[tokio::test]
    async fn the_full_lifecycle_walks_success_fail_reject_and_re_entry() {
        let seeded_story = Story::new_minimal(
            StoryId::new(7200),
            "t".into(),
            Work::NotStarted,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        let (_store, orch, adapter, select) =
            build_orchestrator((seeded_story, vec!["chore".to_string()])).await;

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
        args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
        let result =
            handle_start_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select).await;
        assert!(result.is_ok(), "start_story should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        orch.refresh_all().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(story.state, Work::InProgress);

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("drove the chore end-to-end"),
        );
        let result =
            handle_deliver_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select).await;
        assert!(result.is_ok(), "deliver_story should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(story.state, Work::InProgress);
        assert_eq!(orch.story_fine_stage(&story), Some(Stage::DeskCheck));

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert(
            "reason".into(),
            serde_json::json!("the drill-in header still overlaps the avatar at 320px"),
        );
        let result = handle_fail_desk_check(
            &args,
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
        )
        .await;
        assert!(result.is_ok(), "fail_desk_check should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(story.state, Work::InProgress);
        let rework_placement = adapter
            .pushed_events()
            .iter()
            .rev()
            .find_map(|e| match &e.kind {
                StoryEventKind::StateTransitioned { placement, .. }
                    if e.id == StoryId::new(7200) =>
                {
                    Some(placement.clone())
                }
                _ => None,
            });
        assert_eq!(
            rework_placement,
            Some(None),
            "fail_desk_check must clear the desk-check placement, sending the story back to the dev entry stage"
        );

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("reworked the header overlap and drove it end-to-end"),
        );
        let result =
            handle_deliver_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select).await;
        assert!(
            result.is_ok(),
            "re-entry deliver_story should succeed: {result:?}"
        );
        orch.drain_pending().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(story.state, Work::InProgress);
        assert_eq!(orch.story_fine_stage(&story), Some(Stage::DeskCheck));

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert("acceptor".into(), serde_json::json!("owner"));
        args.insert("would_file_bug".into(), serde_json::json!(false));
        let result = handle_pass_desk_check(
            &args,
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
            None,
        )
        .await;
        assert!(result.is_ok(), "pass_desk_check should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        orch.refresh_all().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(story.state, Work::Acceptance);

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert(
            "reason".into(),
            serde_json::json!("scope drifted from the accepted falsifier"),
        );
        let result =
            handle_reject_story(&args, "pm:claude-code", "pm", None, Some(&orch), &select).await;
        assert!(result.is_ok(), "reject_story should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        orch.refresh_all().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(story.state, Work::NotStarted);

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert(
            "pickup_paraphrase".into(),
            serde_json::json!("paraphrase, take two"),
        );
        args.insert(
            "falsifier_reframing".into(),
            serde_json::json!("reframing, take two"),
        );
        let result =
            handle_start_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select).await;
        let ok = result.expect("re-entry start_story should succeed");
        assert!(
            ok.contains("REWORK") && ok.contains("scope drifted from the accepted falsifier"),
            "re-entry start_story should surface the prior-reject rework context: {ok}"
        );
        orch.drain_pending().await.unwrap();
        orch.refresh_all().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(story.state, Work::InProgress);

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("drove the accepted fix end-to-end after rework"),
        );
        handle_deliver_story(&args, "dev:claude-code", "dev", None, Some(&orch), &select)
            .await
            .unwrap();
        orch.drain_pending().await.unwrap();
        orch.refresh_all().await.unwrap();

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert("acceptor".into(), serde_json::json!("team"));
        args.insert("would_file_bug".into(), serde_json::json!(false));
        handle_pass_desk_check(
            &args,
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
            None,
        )
        .await
        .unwrap();
        orch.drain_pending().await.unwrap();
        orch.refresh_all().await.unwrap();

        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(7200));
        args.insert("would_file_bug".into(), serde_json::json!(false));
        let result = handle_accept_story(
            &args,
            "pm:claude-code",
            "pm",
            None,
            Some(&orch),
            &select,
            None,
        )
        .await;
        assert!(result.is_ok(), "accept_story should succeed: {result:?}");
        orch.drain_pending().await.unwrap();
        orch.refresh_all().await.unwrap();
        let story = orch.story(7200).unwrap();
        assert_eq!(
            story.state,
            Work::Accepted,
            "accepted story is concluded — the close rides the state"
        );
    }
}
