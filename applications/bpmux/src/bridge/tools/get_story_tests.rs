#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::get_story::handle_get_story;
use crate::product_management::board::adapters::github::decode_iteration_from_story;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::product_management::iteration::Iteration;
use crate::product_management::story::{BoardState, Gate, GateComment, Stage, Story, StoryId};
use crate::store::Select;

async fn seeded_orchestrator(
    mut stories: Vec<Story>,
    gates: Vec<(u64, Vec<GateComment>)>,
) -> (Arc<BoardStore>, Arc<dyn RefreshDriver>) {
    for (n, gs) in &gates {
        if let Some(s) = stories.iter_mut().find(|s| s.id == StoryId::new(*n)) {
            s.gate_comments = Some(gs.clone());
        }
    }
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories.clone());
    for (n, gs) in &gates {
        adapter.seed_gate_comments(*n, gs.clone());
    }
    store.seed_all(stories.clone());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let iterations: Vec<Iteration> = stories
        .iter()
        .filter(|s| {
            s.raw_labels
                .iter()
                .any(|l| l.eq_ignore_ascii_case("iteration"))
        })
        .map(decode_iteration_from_story)
        .collect();
    orchestrator.bind_select(Select::seeded(vec![], iterations, vec![]));
    (store, orchestrator)
}

fn make_story(id: u64, state: BoardState, body: Option<&str>) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
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
    let args = serde_json::json!({"id": 443});
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
    let args = serde_json::json!({"id": 9999});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found in board"));
}

#[tokio::test]
async fn enriches_body_for_rejected_story_via_typed_gates() {
    let story = make_story(443, BoardState::Ready, Some("original story body text"));
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
            actor: "owner".into(),
            payload: Some("channelKeyRef is not legitimate".into()),
        },
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(443, gates)]).await;
    let args = serde_json::json!({"id": 443});
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
    let story = make_story(
        200,
        BoardState::InProgress(Stage::Dev),
        Some("original body"),
    );
    let gates = vec![GateComment {
        gate: Gate::FailDeskCheck,
        ts: Utc.with_ymd_and_hms(2026, 4, 23, 1, 0, 0).unwrap(),
        actor: "designer".into(),
        payload: Some("failed spec".into()),
    }];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(200, gates)]).await;
    let args = serde_json::json!({"id": 200});
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
    let story = make_story(100, BoardState::Ready, Some("original body"));
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"id": 100});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(body, "original body");
}

#[tokio::test]
async fn enriches_body_for_ready_story_post_reject_via_transition_history() {
    let story = make_story(700, BoardState::Ready, Some("original body"));
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
            actor: "owner".into(),
            payload: Some("user-story not satisfied; reshape needed".into()),
        },
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(700, gates)]).await;
    let args = serde_json::json!({"id": 700});
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
    let story = make_story(701, BoardState::InProgress(Stage::Dev), Some("clean body"));
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
            actor: "owner".into(),
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
    let args = serde_json::json!({"id": 701});
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
    let story = make_story(50, BoardState::Ready, Some("plain body"));
    let (store, orch) = seeded_orchestrator(vec![story], vec![(50, vec![])]).await;
    let args = serde_json::json!({"id": 50});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(body, "plain body");
}

#[tokio::test]
async fn output_exposes_workflow_state_and_is_closed_separately() {
    let mut story = make_story(101, BoardState::Accepted, Some("body"));
    story.is_closed = true;
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"id": 101});
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
    let mut story = make_story(102, BoardState::Ready, Some("body"));
    story.is_closed = true;
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"id": 102});
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
    let mut story = make_story(103, BoardState::Accepted, Some("body"));
    story.is_closed = false;
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"id": 103});
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
    let mut story = make_story(1207, BoardState::Ready, Some("body"));
    story.raw_labels = vec![
        "bug".to_string(),
        "ready".to_string(),
        "blocked".to_string(),
        "blocked-by:514".to_string(),
        "desk-check-fail".to_string(),
    ];
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"id": 1207});
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
    let mut story = make_story(1208, BoardState::Icebox, Some("body"));
    story.raw_labels = Vec::new();
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"id": 1208});
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

#[tokio::test]
async fn cache_miss_eagerly_fetches_body_via_refresh_driver() {
    let story = make_story(1226, BoardState::Ready, None);
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![story.clone()]);
    adapter.seed_body(
        1226,
        "live-fetched body content via read-through".to_string(),
    );
    store.seed_all(vec![story]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let args = serde_json::json!({"id": 1226});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(
        parsed["body_hydrated"].as_bool(),
        Some(true),
        "cache-miss must trigger read-through fetch and surface body_hydrated=true"
    );
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(
        body, "live-fetched body content via read-through",
        "cache-miss path must return adapter-fetched body, not placeholder"
    );
}

#[tokio::test]
async fn cache_miss_returns_placeholder_when_no_refresh_driver() {
    let story = make_story(1226, BoardState::Ready, None);
    let store = Arc::new(BoardStore::new());
    store.seed_all(vec![story]);
    let args = serde_json::json!({"id": 1226});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), None)
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(
        parsed["body_hydrated"].as_bool(),
        Some(false),
        "no-refresh-driver path keeps body_hydrated=false (placeholder branch)"
    );
    let body = parsed["body"].as_str().expect("body is string");
    assert!(
        body.contains("not yet hydrated"),
        "no-refresh-driver path returns placeholder text; got {body:?}"
    );
}

#[tokio::test]
async fn cache_hit_does_not_refetch_when_body_already_present() {
    let story = make_story(1226, BoardState::Ready, Some("already-cached body"));
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![story.clone()]);
    adapter.seed_body(
        1226,
        "ADAPTER body — should NOT be returned because cache is hit".to_string(),
    );
    store.seed_all(vec![story]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let args = serde_json::json!({"id": 1226});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert_eq!(
        body, "already-cached body",
        "cache-hit path must NOT re-fetch; adapter body must not leak"
    );
}

fn make_iteration(id: u64, name: &str, body: &str) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration {name}"),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels.push("iteration".to_string());
    s.body = Some(body.to_string());
    s
}

fn make_iteration_with_members(id: u64, name: &str, body: &str, members: Vec<u64>) -> Story {
    use crate::product_management::board::adapters::relationships::{
        IterationRelationships, Relationships,
    };
    let mut s = make_iteration(id, name, body);
    s.body = Some(
        IterationRelationships {
            outcome: None,
            stories: members.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body(body),
    );
    s
}

fn make_member(id: u64, title: &str, iteration_id: u64, position: u32) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        title.into(),
        BoardState::Ready,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels = vec![
        "ready".into(),
        format!("parent-iteration-#{iteration_id}.{position}"),
    ];
    s
}

const ITERATION_BODY_TEMPLATE: &str = "## Iteration Telescopium\n\
\n\
**Created**: 2026-05-19\n\
**Status**: Open\n\
\n\
## Member stories\n\
\n\
<!-- Members are attributed by canonical iteration membership (parent-iteration-#N);\n\
     ordering is by position ascending. -->\n\
\n\
_(no members yet)_\n\
\n\
## Notes\n\
\n\
bpmux iteration substrate.\n";

#[tokio::test]
async fn get_story_auto_derives_iteration_members_into_body_1462() {
    let iteration = make_iteration_with_members(
        1397,
        "Telescopium",
        ITERATION_BODY_TEMPLATE,
        vec![1309, 1265, 1463],
    );
    let members = vec![
        make_member(1309, "ci_get_job_status MCP tool", 1397, 100),
        make_member(1265, "reorder_story PM-only gate", 1397, 200),
        make_member(1463, "Split e2e workflow", 1397, 300),
    ];
    let mut all = vec![iteration];
    all.extend(members);
    let (store, orch) = seeded_orchestrator(all, vec![]).await;
    let args = serde_json::json!({"id": 1397});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(
        body.contains("- #1309 — ci_get_job_status MCP tool\n"),
        "first member auto-derived into body: {body}"
    );
    assert!(
        body.contains("- #1265 — reorder_story PM-only gate\n"),
        "second member auto-derived: {body}"
    );
    assert!(
        body.contains("- #1463 — Split e2e workflow\n"),
        "third member auto-derived: {body}"
    );
    assert!(
        !body.contains("_(no members yet)_"),
        "placeholder replaced when members present: {body}"
    );
    let pos_1309 = body.find("- #1309").expect("1309 present");
    let pos_1265 = body.find("- #1265").expect("1265 present");
    let pos_1463 = body.find("- #1463").expect("1463 present");
    assert!(
        pos_1309 < pos_1265 && pos_1265 < pos_1463,
        "members ordered position-ascending (100 < 200 < 300): {body}"
    );
}

#[tokio::test]
async fn get_story_preserves_placeholder_when_iteration_has_no_members_1462() {
    let iteration = make_iteration(1397, "Telescopium", ITERATION_BODY_TEMPLATE);
    let (store, orch) = seeded_orchestrator(vec![iteration], vec![]).await;
    let args = serde_json::json!({"id": 1397});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(
        body.contains("_(no members yet)_"),
        "empty-iteration body preserves placeholder: {body}"
    );
}

#[tokio::test]
async fn get_story_does_not_derive_members_for_non_iteration_stories_1462() {
    let story = make_story(
        1463,
        BoardState::Accepted,
        Some("## Iteration Telescopium\n\n## Member stories\n\n_(no members yet)_\n\n## Notes\n"),
    );
    let (store, orch) = seeded_orchestrator(vec![story], vec![]).await;
    let args = serde_json::json!({"id": 1463});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(
        body.contains("_(no members yet)_"),
        "non-iteration story with member-stories header is left alone (no substitution): {body}"
    );
}

#[tokio::test]
async fn get_story_excludes_closed_member_stories_from_derivation_1462() {
    let iteration = make_iteration_with_members(
        1397,
        "Telescopium",
        ITERATION_BODY_TEMPLATE,
        vec![1309, 1265],
    );
    let mut closed_member = make_member(1309, "Already shipped", 1397, 100);
    closed_member.is_closed = true;
    let open_member = make_member(1265, "Still open", 1397, 200);
    let (store, orch) =
        seeded_orchestrator(vec![iteration, closed_member, open_member], vec![]).await;
    let args = serde_json::json!({"id": 1397});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    let body = parsed["body"].as_str().expect("body is string");
    assert!(
        !body.contains("- #1309"),
        "closed member excluded (sister to members_with_iteration_label closed-skip semantic): {body}"
    );
    assert!(
        body.contains("- #1265 — Still open\n"),
        "open member preserved: {body}"
    );
}

#[tokio::test]
async fn rejected_story_hydrates_latest_reject_verbatim_via_read_through() {
    let story = make_story(1524, BoardState::Ready, Some("base story body"));
    let reason = "drag srt does not persist and does not have the same drag css as the iteration";
    let gates = vec![
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 8, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("delivered the drag-reorder".into()),
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 1, 0).unwrap(),
            actor: "owner".into(),
            payload: Some(reason.into()),
        },
    ];
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![story.clone()]);
    adapter.seed_gate_comments(1524, gates);
    store.seed_all(vec![story]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let args = serde_json::json!({"id": 1524});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");

    assert_eq!(
        parsed["gate_comments_hydrated"].as_bool(),
        Some(true),
        "read-through must hydrate the reject gate-records (the unfetched-not-absent bug)"
    );
    assert_eq!(
        parsed["latest_reject"]["reason"].as_str(),
        Some(reason),
        "reason must round-trip VERBATIM byte-exact — no paraphrase or truncation anywhere in the chain"
    );
    assert_eq!(parsed["latest_reject"]["by"].as_str(), Some("owner"));
    assert_eq!(parsed["latest_reject"]["gate"].as_str(), Some("rejected"));
}

#[tokio::test]
async fn non_rejected_story_carries_no_latest_reject() {
    let story = make_story(500, BoardState::Ready, Some("body"));
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![story.clone()]);
    store.seed_all(vec![story]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let args = serde_json::json!({"id": 500});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
        .await
        .expect("should succeed");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");

    assert!(
        parsed
            .get("latest_reject")
            .map(|x| x.is_null())
            .unwrap_or(true),
        "a non-rejected story must not carry a latest_reject"
    );
}

#[tokio::test]
async fn latest_reject_survives_an_iteration_move() {
    let story = make_story(1524, BoardState::Ready, Some("base body"));
    let reason = "drag srt does not persist and does not have the same drag css as the iteration";
    let gates = vec![GateComment {
        gate: Gate::Rejected,
        ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 1, 0).unwrap(),
        actor: "owner".into(),
        payload: Some(reason.into()),
    }];
    let (store, orch) = seeded_orchestrator(vec![story], vec![(1524, gates)]).await;
    let args = serde_json::json!({"id": 1524});

    let before = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .unwrap();
    let before_json: serde_json::Value = serde_json::from_str(&before).unwrap();
    assert_eq!(
        before_json["latest_reject"]["reason"].as_str(),
        Some(reason),
        "pre-move: the rejected story surfaces its reject"
    );

    let meta = crate::product_management::board::orchestrator::cache_fill_meta();
    store.apply(&crate::product_management::board::StoryEvent::TagAdded {
        meta: meta.clone(),
        id: StoryId::new(1524),
        label: "parent-iteration-#1656.1".to_string(),
    });
    store.apply(&crate::product_management::board::StoryEvent::TagRemoved {
        meta,
        id: StoryId::new(1524),
        label: "parent-iteration-#1655.1".to_string(),
    });

    let after = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .unwrap();
    let after_json: serde_json::Value = serde_json::from_str(&after).unwrap();
    assert_eq!(
        after_json["latest_reject"]["reason"].as_str(),
        Some(reason),
        "the reject SURVIVES the iteration-move — gate-comments live on the issue; the move only re-labels parent-iteration, never touches the gate-records"
    );
}

#[tokio::test]
async fn get_story_stays_unhydrated_when_gate_fetch_fails() {
    let story = make_story(442, BoardState::Ready, Some("body"));
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.set_ingest_failures(true);
    store.seed_all(vec![story]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    let args = serde_json::json!({"id": 442});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orchestrator))
        .await
        .expect("handler degrades gracefully when the gate read-through fails");
    let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
    assert_eq!(
        parsed["gate_comments_hydrated"].as_bool(),
        Some(false),
        "a failed gate read-through degrades gracefully — unhydrated, not an error"
    );
    assert!(
        parsed
            .get("latest_reject")
            .map(|x| x.is_null())
            .unwrap_or(true),
        "no latest_reject surfaces when the gate-records couldn't be fetched"
    );
}

#[tokio::test]
async fn carried_reject_workflow_state_derives_rework_not_phantom_accepted() {
    let mut story = make_story(1700, BoardState::Accepted, Some("body"));
    story.is_closed = true;
    let reason = "carried reject — the active rework";
    let gates = vec![
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 8, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("delivered".into()),
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 0, 0).unwrap(),
            actor: "owner".into(),
            payload: Some(reason.into()),
        },
    ];
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![story.clone()]);
    adapter.seed_gate_comments(1700, gates);
    store.seed_all(vec![story]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let args = serde_json::json!({"id": 1700});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed["workflow_state"], "ready",
        "carried-reject (last gate = backward Rejected) must derive rework, NOT phantom-accepted from the stale closed-state"
    );
    assert_eq!(
        parsed["is_closed"], true,
        "is_closed (GH-state axis) still reflects the stale closed issue; only workflow_state derives from the gate-history"
    );
}

#[tokio::test]
async fn re_accepted_after_reject_workflow_state_stays_accepted() {
    let mut story = make_story(1701, BoardState::Accepted, Some("body"));
    story.is_closed = true;
    let gates = vec![
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 8, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("first delivery".into()),
        },
        GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 0, 0).unwrap(),
            actor: "owner".into(),
            payload: Some("rework".into()),
        },
        GateComment {
            gate: Gate::Delivered,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 10, 0, 0).unwrap(),
            actor: "dev".into(),
            payload: Some("re-delivery".into()),
        },
        GateComment {
            gate: Gate::Accepted,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 11, 0, 0).unwrap(),
            actor: "owner".into(),
            payload: None,
        },
    ];
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![story.clone()]);
    adapter.seed_gate_comments(1701, gates);
    store.seed_all(vec![story]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));

    let args = serde_json::json!({"id": 1701});
    let result = handle_get_story(args.as_object().unwrap(), Some(&store), Some(&orch))
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(
        parsed["workflow_state"], "accepted",
        "re-accepted-after-reject (last gate = forward Accepted) stays accepted — the earlier reject is SUPERSEDED; guards the .last() primitive against an any-backward regression"
    );
}
