#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

mod gate_retire_1348 {
    use crate::bridge::tools_stories::{handle_deliver_story, handle_start_story};
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    async fn seeded_orchestrator(
        number: u64,
        title: &str,
        state: StoryState,
        story_type: StoryType,
        body: &str,
    ) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let mut seeded = Story::new_minimal(
            number,
            title.into(),
            state,
            Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        seeded.story_type = story_type;
        adapter.seed_stories(vec![seeded]);
        adapter.seed_body(number, body.into());
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            Arc::clone(&store),
            Arc::clone(&adapter),
        ));
        orch.refresh_all().await.unwrap();
        orch
    }

    #[tokio::test]
    async fn start_story_accepts_gherkin_only_feature_body_without_atp_section() {
        let body = "## User job\n\
                    \n\
                    When dev fires start_story on a Gherkin-only body, the gate accepts.\n\
                    \n\
                    ## Acceptance Criteria (Gherkin)\n\
                    \n\
                    - **Scenario:** start succeeds\n\
                    - **Given:** a body with Gherkin but no ATP section\n\
                    - **When:** dev fires start_story\n\
                    - **Then:** start_story returns Ok\n";
        let orch = seeded_orchestrator(
            9101,
            "[Feature] gherkin-only body",
            StoryState::Ready,
            StoryType::Feature,
            body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("number".into(), serde_json::json!(9101));
        args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
        args.insert("falsifier_reframing".into(), serde_json::json!("f"));
        let result = handle_start_story(&args, "dev:claude-code", Some(&orch)).await;
        assert!(
            result.is_ok(),
            "Gherkin-only Feature start_story must succeed without ATP section: {result:?}"
        );
    }

    #[tokio::test]
    async fn deliver_story_accepts_gherkin_only_feature_body_without_atp_section() {
        let body = "## Acceptance Criteria (Gherkin)\n\
                    \n\
                    - **Given:** Gherkin-only body\n\
                    - **When:** dev delivers\n\
                    - **Then:** gate accepts\n";
        let orch = seeded_orchestrator(
            9102,
            "[Feature] gherkin-only deliver",
            StoryState::InProgress,
            StoryType::Feature,
            body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("number".into(), serde_json::json!(9102));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("drove the user-job end-to-end via real-backend tests"),
        );
        let result = handle_deliver_story(&args, "dev:claude-code", Some(&orch)).await;
        assert!(
            result.is_ok(),
            "Gherkin-only Feature deliver_story must succeed without ATP section: {result:?}"
        );
    }

    #[tokio::test]
    async fn start_story_accepts_gherkin_only_bug_body_without_atp_section() {
        let body = "## Acceptance Criteria (Gherkin)\n\
                    \n\
                    - **Given:** Gherkin-only Bug body\n\
                    - **When:** dev starts\n\
                    - **Then:** start_story returns Ok\n";
        let orch = seeded_orchestrator(
            9103,
            "[Bug] gherkin-only body",
            StoryState::Ready,
            StoryType::Bug,
            body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("number".into(), serde_json::json!(9103));
        args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
        args.insert("falsifier_reframing".into(), serde_json::json!("f"));
        let result = handle_start_story(&args, "dev:claude-code", Some(&orch)).await;
        assert!(
            result.is_ok(),
            "Gherkin-only Bug start_story must succeed without ATP section: {result:?}"
        );
    }

    #[tokio::test]
    async fn start_story_still_validates_atp_path_when_present_in_body() {
        let body = format!("## Acceptance test path\n\n{}\n", "does/not/exist.spec.ts");
        let orch = seeded_orchestrator(
            9104,
            "[Feature] backward-compat ATP path",
            StoryState::Ready,
            StoryType::Feature,
            &body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("number".into(), serde_json::json!(9104));
        args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
        args.insert("falsifier_reframing".into(), serde_json::json!("f"));
        let result = handle_start_story(&args, "dev:claude-code", Some(&orch)).await;
        assert!(
            result.is_err(),
            "Feature with body-named ATP path that doesn't exist must still block: {result:?}"
        );
        assert!(
            result.unwrap_err().contains("does not exist"),
            "backward-compat path-existence check preserved"
        );
    }

    #[tokio::test]
    async fn deliver_story_still_validates_atp_path_when_present_in_body() {
        let body = format!("## Acceptance test path\n\n{}\n", "does/not/exist.spec.ts");
        let orch = seeded_orchestrator(
            9105,
            "[Feature] backward-compat ATP deliver",
            StoryState::InProgress,
            StoryType::Feature,
            &body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("number".into(), serde_json::json!(9105));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("drove end-to-end via tests"),
        );
        let result = handle_deliver_story(&args, "dev:claude-code", Some(&orch)).await;
        assert!(
            result.is_err(),
            "Feature with body-named ATP path that doesn't exist must still block at deliver: {result:?}"
        );
        assert!(
            result.unwrap_err().contains("does not exist"),
            "backward-compat path-existence check preserved at deliver"
        );
    }
}
