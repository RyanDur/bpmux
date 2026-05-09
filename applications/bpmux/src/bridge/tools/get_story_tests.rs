#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::get_story::handle_get_story;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Gate, GateComment, Story, StoryState};
use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};

async fn seeded_orchestrator(
    mut stories: Vec<Story>,
    gates: Vec<(u64, Vec<GateComment>)>,
) -> (Arc<BoardStore>, Arc<dyn RefreshDriver>) {
    for (n, gs) in &gates {
        if let Some(s) = stories.iter_mut().find(|s| s.number == *n) {
            s.gate_comments = Some(gs.clone());
        }
    }
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    for (n, gs) in &gates {
        adapter.seed_gate_comments(*n, gs.clone());
    }
    store.seed_all(stories);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    (store, orchestrator)
}

fn make_story(number: u64, state: StoryState, body: Option<&str>) -> Story {
    let mut s = Story::new_minimal(
        number,
        "title".into(),
        state,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    if let Some(b) = body {
        s.body = Some(b.to_string());
    }
    s
}

#[tokio::test]
async fn handler_requires_board_after_canonical_only_refactor() {
    let args = serde_json::json!({"number": 443});
    let result = handle_get_story(args.as_object().unwrap(), None, None).await;
    assert!(result.is_err());
    assert!(
        result.unwrap_err().contains("requires board"),
        "post-#1165 slice 1: refresh no longer required for read; canonical-only"
    );
}

#[tokio::test]
async fn handler_errors_when_story_not_in_board() {
    let (store, orch) = seeded_orchestrator(vec![], vec![]).await;
    let args = serde_json::json!({"number": 9999});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in board"));
}

#[tokio::test]
async fn enriches_body_for_rejected_story_via_typed_gates() {
    let story = make_story(443, StoryState::Ready, Some("original story body text"));
    let gates = vec![
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 4, 23, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("prior delivery ran end-to-end".into()),
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 4, 23, 1, 0, 0).unwrap(),
            actor: "ryan".into(),
            payload: Some("channelKeyRef is not legitimate".into()),
        },
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(443, gates)]).await;
    let args = serde_json::json!({"number": 443});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(body.starts_with("## Rework context"));
    assert!(body.contains("channelKeyRef is not legitimate"));
    assert!(body.contains("prior delivery ran end-to-end"));
    assert!(body.ends_with("original story body text"));
}

#[tokio::test]
async fn enriches_body_for_needs_fixes_story_via_typed_gates() {
    let story = make_story(200, StoryState::InProgress, Some("original body"));
    let gates = vec![GateComment {
        gate: Gate::FailDeskCheck,
        ts: Utc.with_ymd_and_hms(2026, 4, 23, 1, 0, 0).unwrap(),
        actor: "designer".into(),
        payload: Some("failed spec".into()),
    }];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(200, gates)]).await;
    let args = serde_json::json!({"number": 200});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(body.contains("## Rework context"));
    assert!(body.contains("failed spec"));
}

#[tokio::test]
async fn does_not_enrich_body_for_non_rework_story() {
    let story = make_story(100, StoryState::Ready, Some("original body"));
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"number": 100});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(body, "original body");
}

#[tokio::test]
async fn enriches_body_for_ready_story_post_reject_via_transition_history() {
    // #1146 slice 2a: rework-detect derives from transition-history
    // backward-edge predicate, not state-keyed match. Post-slice-1
    // reject_story transitions Delivered → Ready (not Rejected); this
    // test pins that get_story still surfaces "## Rework context" when
    // the story is in Ready state and has a backward-edge in its
    // transition history.
    let story = make_story(700, StoryState::Ready, Some("original body"));
    let gates = vec![
        GateComment {
            gate: Gate::Pickup,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 1, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("ATP green; dev verified user-job".into()),
        },
        GateComment {
            gate: Gate::PassDeskCheck,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 2, 0, 0).unwrap(),
            actor: "designer".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 3, 0, 0).unwrap(),
            actor: "ryan".into(),
            payload: Some("user-story not satisfied; reshape needed".into()),
        },
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(700, gates)]).await;
    let args = serde_json::json!({"number": 700});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(
        body.starts_with("## Rework context"),
        "rework context surfaces for Ready story with backward-edge in history; got:\n{body}"
    );
    assert!(body.contains("user-story not satisfied; reshape needed"));
    assert!(body.contains("ATP green; dev verified user-job"));
}

#[tokio::test]
async fn does_not_enrich_body_when_most_recent_transition_is_forward() {
    // #1146 slice 2a: most_recent_backward_transition returns None when
    // the latest replay-edge is forward (e.g., Ready → InProgress via
    // start_story after a prior reject). Rework-context stops surfacing
    // once the story moves forward again.
    let story = make_story(701, StoryState::InProgress, Some("clean body"));
    let gates = vec![
        GateComment {
            gate: Gate::Pickup,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 1, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("first delivery".into()),
        },
        GateComment {
            gate: Gate::PassDeskCheck,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 2, 0, 0).unwrap(),
            actor: "designer".into(),
            payload: None,
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 3, 0, 0).unwrap(),
            actor: "ryan".into(),
            payload: Some("rework needed".into()),
        },
        GateComment {
            gate: Gate::Pickup,
            ts: Utc.with_ymd_and_hms(2026, 5, 1, 4, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: None,
        },
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(701, gates)]).await;
    let args = serde_json::json!({"number": 701});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(
        body, "clean body",
        "forward transition (Ready → InProgress via Pickup) suppresses rework-context"
    );
}

#[tokio::test]
async fn skips_enrichment_when_no_rework_gate_comments() {
    let story = make_story(50, StoryState::Ready, Some("plain body"));
    let (store, orch) = seeded_orchestrator(vec![story], vec![(50, vec![])]).await;
    let args = serde_json::json!({"number": 50});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(body, "plain body");
}

#[tokio::test]
async fn output_exposes_workflow_state_and_is_closed_separately() {
    let mut story = make_story(101, StoryState::Accepted, Some("body"));
    story.is_closed = true;
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"number": 101});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(parsed["workflow_state"], "accepted");
    assert_eq!(parsed["is_closed"], true);
    assert!(
        parsed.get("state").is_none(),
        "post-#1144: legacy `state` field is removed; only workflow_state + is_closed appear"
    );
}

#[tokio::test]
async fn divergent_case_workflow_ready_but_gh_closed_observable() {
    let mut story = make_story(102, StoryState::Ready, Some("body"));
    story.is_closed = true;
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"number": 102});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(
        parsed["workflow_state"], "ready",
        "workflow axis reports Ready independent of GH-state"
    );
    assert_eq!(
        parsed["is_closed"], true,
        "GH-state axis reports is_closed=true even though workflow != Accepted"
    );
}

#[tokio::test]
async fn divergent_case_workflow_accepted_but_gh_open_observable() {
    let mut story = make_story(103, StoryState::Accepted, Some("body"));
    story.is_closed = false;
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"number": 103});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(parsed["workflow_state"], "accepted");
    assert_eq!(
        parsed["is_closed"], false,
        "GH-state axis reports is_closed=false even though workflow == Accepted (the 2026-05-02 false-success class)"
    );
}

#[tokio::test]
async fn surfaces_full_raw_labels_array_not_synthesized_state_label() {
    let mut story = make_story(1207, StoryState::Ready, Some("body"));
    story.raw_labels = vec![
        "bug".to_string(),
        "ready".to_string(),
        "blocked".to_string(),
        "blocked-by:514".to_string(),
        "desk-check-fail".to_string(),
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"number": 1207});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let labels = parsed["labels"].as_array().expect("labels is array");
    let names: Vec<&str> = labels
        .iter()
        .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        names.contains(&"bug"),
        "Type label 'bug' surfaces; got {names:?}"
    );
    assert!(
        names.contains(&"blocked"),
        "Block label 'blocked' surfaces; got {names:?}"
    );
    assert!(
        names.contains(&"blocked-by:514"),
        "Blocker-link label surfaces; got {names:?}"
    );
    assert!(
        names.contains(&"desk-check-fail"),
        "Rework label surfaces; got {names:?}"
    );
    assert!(
        names.contains(&"ready"),
        "State label still surfaces alongside others; got {names:?}"
    );
}

#[tokio::test]
async fn empty_raw_labels_returns_empty_labels_array() {
    let mut story = make_story(1208, StoryState::Icebox, Some("body"));
    story.raw_labels = Vec::new();
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"number": 1208});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let labels = parsed["labels"].as_array().expect("labels is array");
    assert!(
        labels.is_empty(),
        "raw_labels empty → labels array empty (no state-label synthesis); got {labels:?}"
    );
}
