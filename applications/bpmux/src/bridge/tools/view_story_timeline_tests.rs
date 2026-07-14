#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::view_story_timeline::handle_view_story_timeline;

#[tokio::test]
async fn handler_requires_refresh_as_the_canonical_read_handle() {
    let args = serde_json::json!({"id": 418});
    let result = handle_view_story_timeline(args.as_object().unwrap(), None).await;
    let err = result.expect_err("missing-refresh path errors");
    assert!(
        err.contains("requires refresh"),
        "the RefreshDriver is the single canonical read handle post-dissolution-S2; got: {err}"
    );
}

#[tokio::test]
async fn handler_errors_when_story_not_in_board() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();

    let args = serde_json::json!({"id": 9999});
    let result = handle_view_story_timeline(args.as_object().unwrap(), Some(&orchestrator)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in board"));
}

#[tokio::test]
async fn handler_reads_gates_from_canonical_store() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::slice::StoryAction;
    use product_management::story::{CastVerdict, Story, StoryId, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let filed_at = Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap();
    let mut story = Story::new_minimal(
        StoryId::new(418),
        "title".into(),
        Work::Acceptance,
        filed_at,
        "pm".into(),
    );
    story.verdicts = Some(vec![CastVerdict {
        verdict: Verdict::Succeed {
            step: "in-progress".into(),
        },
        ts: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
        actor: "dev".into(),
        payload: Some("ran end-to-end".into()),
    }]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let handle = crate::store::SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orchestrator);
    handle.dispatch(crate::store::AppAction::Story(StoryAction::Seeded(vec![
        story,
    ])));

    let args = serde_json::json!({"id": 418});
    let result = handle_view_story_timeline(args.as_object().unwrap(), Some(&orchestrator))
        .await
        .expect("canonical-path handler success");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(parsed["gate_comments_hydrated"], true);
    let events = parsed["events"].as_array().expect("events array");
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["gate"], "filed");
    assert_eq!(events[0]["actor"], "pm");
    assert_eq!(events[1]["gate"], "delivered");
    assert_eq!(events[1]["actor"], "dev");
    assert_eq!(events[1]["payload"], "ran end-to-end");
}

#[tokio::test]
async fn handler_returns_filed_only_with_unhydrated_flag_when_verdicts_missing() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::slice::StoryAction;
    use product_management::story::{Story, StoryId, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    let story = Story::new_minimal(
        StoryId::new(419),
        "title".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let handle = crate::store::SeededStore::new(vec![], vec![], vec![]);
    handle.bind_refresh(&orchestrator);
    handle.dispatch(crate::store::AppAction::Story(StoryAction::Seeded(vec![
        story,
    ])));

    let args = serde_json::json!({"id": 419});
    let result = handle_view_story_timeline(args.as_object().unwrap(), Some(&orchestrator))
        .await
        .expect("canonical-path handler succeeds without live GH");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(parsed["gate_comments_hydrated"], false);
    let events = parsed["events"].as_array().expect("events array");
    assert_eq!(events.len(), 1, "filed event only when gates not hydrated");
    assert_eq!(events[0]["gate"], "filed");
}

#[test]
fn build_timeline_from_board_sorts_chronologically() {
    use super::view_story_timeline::build_timeline_from_board;
    use chrono::{TimeZone, Utc};
    use product_management::story::{CastVerdict, Story, StoryId, Verdict, Work};

    let story = Story::new_minimal(
        StoryId::new(500),
        "title".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let gates = vec![
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "in-progress".into(),
            },
            ts: Utc.with_ymd_and_hms(2026, 4, 25, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "ready".into(),
            },
            ts: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
    ];
    let events = build_timeline_from_board(&story, &gates);
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["gate"], "filed");
    assert_eq!(events[1]["gate"], "pickup");
    assert_eq!(events[2]["gate"], "delivered");
}

#[test]
fn build_timeline_from_board_renders_full_rework_cycle() {
    use super::view_story_timeline::build_timeline_from_board;
    use chrono::{TimeZone, Utc};
    use product_management::story::{CastVerdict, Story, StoryId, Verdict, Work};

    let story = Story::new_minimal(
        StoryId::new(501),
        "title".into(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let gates = vec![
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "ready".into(),
            },
            ts: Utc.with_ymd_and_hms(2026, 4, 21, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "in-progress".into(),
            },
            ts: Utc.with_ymd_and_hms(2026, 4, 22, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("first delivery".into()),
        },
        CastVerdict {
            verdict: Verdict::Fail {
                step: "acceptance".into(),
            },
            ts: Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap(),
            actor: "owner".into(),
            payload: Some("not quite".into()),
        },
        CastVerdict {
            verdict: Verdict::Succeed {
                step: "ready".into(),
            },
            ts: Utc.with_ymd_and_hms(2026, 4, 24, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
    ];
    let events = build_timeline_from_board(&story, &gates);
    assert_eq!(events.len(), 5);
    assert_eq!(events[0]["gate"], "filed");
    assert_eq!(events[1]["gate"], "pickup");
    assert_eq!(events[2]["gate"], "delivered");
    assert_eq!(events[3]["gate"], "rejected");
    assert_eq!(events[4]["gate"], "pickup");
    assert_eq!(events[2]["payload"], "first delivery");
    assert_eq!(events[3]["payload"], "not quite");
}

#[tokio::test]
async fn timeline_hydrates_reject_gate_verbatim_via_read_through() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{CastVerdict, Story, StoryId, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let reason = "drag srt does not persist and does not have the same drag css as the iteration";
    let story = Story::new_minimal(
        StoryId::new(1524),
        "title".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 6, 6, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![story]);
    adapter.seed_verdicts(
        1524,
        vec![CastVerdict {
            verdict: Verdict::Fail {
                step: "acceptance".into(),
            },
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 1, 0).unwrap(),
            actor: "owner".into(),
            payload: Some(reason.into()),
        }],
    );
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        adapter,
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();

    let args = serde_json::json!({"id": 1524});
    let result = handle_view_story_timeline(args.as_object().unwrap(), Some(&orchestrator))
        .await
        .expect("success");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(
        parsed["gate_comments_hydrated"], true,
        "read-through hydrates the timeline's reject gate-records"
    );
    let events = parsed["events"].as_array().expect("events array");
    let reject = events
        .iter()
        .find(|e| e["gate"] == "rejected")
        .expect("reject event present in hydrated timeline");
    assert_eq!(
        reject["payload"], reason,
        "the reject reason is VERBATIM byte-exact in the timeline"
    );
    assert_eq!(reject["actor"], "owner");
}
