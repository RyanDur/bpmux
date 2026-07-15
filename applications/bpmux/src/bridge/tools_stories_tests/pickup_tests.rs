#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use protocol::ClientMessage;

#[tokio::test]
async fn start_story_rejects_missing_pickup_paraphrase() {
    let args = serde_json::json!({
        "id": 398,
        "falsifier_reframing": "I would reject if the pickup signal is invisible."
    });
    let result = handle_start_story(
        args.as_object().unwrap(),
        "dev:claude-code",
        "dev",
        None,
        None,
    )
    .await;
    assert!(
        result.is_err(),
        "missing pickup_paraphrase must fail: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("pickup_paraphrase"),
        "error should name the field: {err}"
    );
    assert!(
        err.contains("retro") || err.contains("pickup time"),
        "error should cite retro framing so agents know the why: {err}"
    );
}

#[tokio::test]
async fn start_story_rejects_empty_pickup_paraphrase() {
    let args = serde_json::json!({
        "id": 398,
        "pickup_paraphrase": "   \n  ",
        "falsifier_reframing": "I would reject if..."
    });
    let result = handle_start_story(
        args.as_object().unwrap(),
        "dev:claude-code",
        "dev",
        None,
        None,
    )
    .await;
    assert!(
        result.is_err(),
        "whitespace-only pickup_paraphrase must fail"
    );
    assert!(result.unwrap_err().contains("pickup_paraphrase"));
}

#[tokio::test]
async fn start_story_rejects_missing_falsifier_reframing() {
    let args = serde_json::json!({
        "id": 398,
        "pickup_paraphrase": "As a dev, I restate the user-job..."
    });
    let result = handle_start_story(
        args.as_object().unwrap(),
        "dev:claude-code",
        "dev",
        None,
        None,
    )
    .await;
    assert!(
        result.is_err(),
        "missing falsifier_reframing must fail: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("falsifier_reframing"),
        "error should name the field: {err}"
    );
}

#[tokio::test]
async fn start_story_validation_passes_then_refresh_required_after_5n() {
    let args = serde_json::json!({
        "id": 398,
        "pickup_paraphrase": "User-job: dev articulates their read of the story at pickup so the team knows work is in flight.",
        "falsifier_reframing": "I would reject if start_story without the new fields still succeeds."
    });
    let result = handle_start_story(
        args.as_object().unwrap(),
        "dev:claude-code",
        "dev",
        None,
        None,
    )
    .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("requires refresh"));
}

#[tokio::test]
async fn start_story_stamps_actor_id_from_the_threaded_identify_user_id() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::StoryEventKind;
    use product_management::story::{Story, StoryId, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(886),
        "[Chore] actor_id threads through pickup".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 10, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(886, vec!["chore".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(886));
    args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
    args.insert("falsifier_reframing".into(), serde_json::json!("f"));
    handle_start_story(
        &args,
        "dev:claude-code",
        "dev",
        Some("user-alpha-42"),
        Some(&orch),
    )
    .await
    .expect("pickup succeeds");

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    let transitioned = pushed
        .iter()
        .find(|e| {
            matches!(&e.kind, StoryEventKind::StateTransitioned { .. }) && e.id == StoryId::new(886)
        })
        .expect("StateTransitioned pushed to the adapter");
    assert_eq!(
        transitioned.meta.actor_id.as_deref(),
        Some("user-alpha-42"),
        "the tool call's threaded actor_id lands verbatim on the emitted event's meta — the id-carrying producer contract this slice adds"
    );
    assert_eq!(
        transitioned.meta.actor, "dev",
        "meta.actor stays the display short_name(agent_name) — B-4 is additive, GH/render attribution is unchanged"
    );
}

#[tokio::test]
async fn start_story_still_dev_only_with_new_params() {
    let args = serde_json::json!({
        "id": 398,
        "pickup_paraphrase": "any text",
        "falsifier_reframing": "any text"
    });
    for caller in ["pm:claude-code", "designer:claude-code"] {
        let result = handle_start_story(
            args.as_object().unwrap(),
            caller,
            role_from_name(caller),
            None,
            None,
        )
        .await;
        assert!(
            result.is_err(),
            "{caller} (non-dev, non-owner) must be rejected by workflow-substrate role-gate"
        );
        let err = result.unwrap_err();
        assert!(
            err.contains("\"kind\":\"workflow-role-not-allowed\""),
            "{caller} rejection should cite structured kind; got {err}"
        );
        assert!(
            err.contains("\"attempted_transition\":\"pickup\""),
            "{caller} rejection should cite attempted_transition; got {err}"
        );
    }
}
