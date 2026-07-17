#![allow(unused_imports)]

use super::super::tools_stories::*;

#[cfg(test)]
mod stage_verb_behavior {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::StoryEvent;
    use product_management::story::{Stage, Story, StoryId, Verdict, Work};
    use std::sync::Arc;

    async fn seeded(
        id: u64,
        state: Work,
        labels: &[&str],
    ) -> (
        crate::store::SeededStore,
        Arc<SyncOrchestrator>,
        crate::store::Select,
    ) {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let story = Story::new_minimal(
            StoryId::new(id),
            "t".into(),
            state,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        adapter.seed_labels(id, labels.iter().map(|l| l.to_string()).collect());
        adapter.seed_stories(vec![story]);
        let orch: Arc<SyncOrchestrator> = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
        store.bind_refresh(&orch);
        orch.refresh_all().await.unwrap();
        let select = store.select();
        (store, orch, select)
    }

    fn pass_args(id: u64) -> serde_json::Map<String, serde_json::Value> {
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(id));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("drove the stage's outcome end-to-end in the running app"),
        );
        args
    }

    fn fail_args(id: u64, reason: &str) -> serde_json::Map<String, serde_json::Value> {
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(id));
        args.insert("reason".into(), serde_json::json!(reason));
        args
    }

    #[tokio::test]
    async fn pass_stage_at_the_first_stage_advances_to_the_next_configured_stage() {
        let (_store, orch, select) = seeded(8100, Work::InProgress, &["chore"]).await;
        let result = handle_pass_stage(
            &pass_args(8100),
            "dev:claude-code",
            "dev",
            None,
            Some(&orch),
            &select,
        )
        .await;
        assert!(result.is_ok(), "the dev judges the dev stage: {result:?}");
        let story = select
            .select(|state| {
                product_management::story::slice::get(&state.stories, StoryId::new(8100))
            })
            .unwrap();
        assert_eq!(story.state, Work::InProgress);
        assert_eq!(
            select.select(|state| board::fine_stage(&story, &state.board)),
            Some(Stage::DeskCheck),
            "passing the first middle stage lands the story at the next configured stage"
        );
    }

    #[tokio::test]
    async fn pass_stage_refuses_an_actor_who_is_not_the_configured_judge() {
        let (_store, orch, select) = seeded(8101, Work::InProgress, &["chore", "desk-check"]).await;
        let err = handle_pass_stage(
            &pass_args(8101),
            "dev:claude-code",
            "dev",
            None,
            Some(&orch),
            &select,
        )
        .await
        .expect_err("dev is not desk-check's judge");
        assert!(
            err.contains("workflow-role-not-allowed") && err.contains("judge:desk-check"),
            "the refusal names the stage judgment: {err}"
        );
    }

    #[tokio::test]
    async fn pass_stage_at_the_last_stage_crosses_to_acceptance() {
        let (_store, orch, select) = seeded(8102, Work::InProgress, &["chore", "desk-check"]).await;
        let result = handle_pass_stage(
            &pass_args(8102),
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
        )
        .await;
        assert!(result.is_ok(), "designer judges desk-check: {result:?}");
        orch.drain_pass().await.unwrap();
        orch.refresh_all().await.unwrap();
        let story = select
            .select(|state| {
                product_management::story::slice::get(&state.stories, StoryId::new(8102))
            })
            .unwrap();
        assert_eq!(
            story.state,
            Work::Acceptance,
            "passing the last middle stage crosses the section boundary"
        );
    }

    #[tokio::test]
    async fn fail_stage_sends_the_story_back_per_the_ladder() {
        let (_store, orch, select) = seeded(8103, Work::InProgress, &["chore", "desk-check"]).await;
        let result = handle_fail_stage(
            &fail_args(8103, "the drill-in header overlaps the avatar at 320px"),
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
        )
        .await;
        assert!(result.is_ok(), "designer judges desk-check: {result:?}");
        let story = select
            .select(|state| {
                product_management::story::slice::get(&state.stories, StoryId::new(8103))
            })
            .unwrap();
        assert_eq!(story.state, Work::InProgress);
        assert_eq!(
            select.select(|state| board::fine_stage(&story, &state.board)),
            Some(Stage::Dev),
            "a failed desk-check judgment sends the story back one stage"
        );
    }

    #[tokio::test]
    async fn fail_stage_records_the_verdict_at_the_judged_stage() {
        let (_store, orch, select) = seeded(8104, Work::InProgress, &["chore", "desk-check"]).await;
        handle_fail_stage(
            &fail_args(8104, "the falsifier scenario reproduces the overlap"),
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
        )
        .await
        .unwrap();
        orch.drain_pass().await.unwrap();
        let story = select
            .select(|state| {
                product_management::story::slice::get(&state.stories, StoryId::new(8104))
            })
            .unwrap();
        let verdicts = story.verdicts.as_deref().unwrap_or(&[]);
        assert!(
            verdicts.iter().any(|v| matches!(
                &v.verdict,
                Verdict::Fail { step } if step.as_str() == "desk-check"
            )),
            "the fail verdict is cast AT the judged stage, not a hardcoded step: {verdicts:?}"
        );
    }

    #[tokio::test]
    async fn fail_stage_requires_a_cited_reason() {
        let (_store, orch, select) = seeded(8105, Work::InProgress, &["chore", "desk-check"]).await;
        let err = handle_fail_stage(
            &fail_args(8105, "nope"),
            "designer:claude-code",
            "designer",
            None,
            Some(&orch),
            &select,
        )
        .await
        .expect_err("a bare 'nope' is not a citable judgment");
        assert!(
            err.contains("workflow-gate-mismatch"),
            "short reasons are refused like fail_desk_check's falsifier-cite gate: {err}"
        );
    }

    #[tokio::test]
    async fn stage_verbs_require_the_story_in_the_middle() {
        let (_store, orch, select) = seeded(8106, Work::NotStarted, &["chore"]).await;
        let err = handle_pass_stage(
            &pass_args(8106),
            "dev:claude-code",
            "dev",
            None,
            Some(&orch),
            &select,
        )
        .await
        .expect_err("a ready story has no stage to judge");
        assert!(
            err.contains("not in the board's middle"),
            "the refusal explains the story has no current stage: {err}"
        );
    }
}
