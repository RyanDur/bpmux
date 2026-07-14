#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

mod gate_retire_1348 {
    use crate::bridge::tools_stories::{handle_deliver_story, handle_start_story};
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{Story, StoryId, StoryKind, Work};
    use std::sync::Arc;

    async fn seeded_orchestrator(
        id: u64,
        title: &str,
        state: Work,
        story_type: StoryKind,
        body: &str,
    ) -> Arc<dyn RefreshDriver> {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        let seeded = Story::new_minimal(
            StoryId::new(id),
            title.into(),
            state,
            Utc.with_ymd_and_hms(2026, 5, 15, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        adapter.seed_labels(
            id,
            vec![source_codec::gh_label_codec::GhLabelCodec.encode_story_type(story_type)],
        );
        adapter.seed_stories(vec![seeded]);
        adapter.seed_body(id, body.into());
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            adapter,
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
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
            Work::NotStarted,
            StoryKind::Feature,
            body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(9101));
        args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
        args.insert("falsifier_reframing".into(), serde_json::json!("f"));
        let result = handle_start_story(&args, "dev:claude-code", "dev", None, Some(&orch)).await;
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
            Work::InProgress,
            StoryKind::Feature,
            body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(9102));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("drove the user-job end-to-end via real-backend tests"),
        );
        let result = handle_deliver_story(&args, "dev:claude-code", "dev", None, Some(&orch)).await;
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
            Work::NotStarted,
            StoryKind::Bug,
            body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(9103));
        args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
        args.insert("falsifier_reframing".into(), serde_json::json!("f"));
        let result = handle_start_story(&args, "dev:claude-code", "dev", None, Some(&orch)).await;
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
            Work::NotStarted,
            StoryKind::Feature,
            &body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(9104));
        args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
        args.insert("falsifier_reframing".into(), serde_json::json!("f"));
        let result = handle_start_story(&args, "dev:claude-code", "dev", None, Some(&orch)).await;
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
            Work::InProgress,
            StoryKind::Feature,
            &body,
        )
        .await;
        let mut args = serde_json::Map::new();
        args.insert("id".into(), serde_json::json!(9105));
        args.insert(
            "verified_observation".into(),
            serde_json::json!("drove end-to-end via tests"),
        );
        let result = handle_deliver_story(&args, "dev:claude-code", "dev", None, Some(&orch)).await;
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
