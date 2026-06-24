#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::product_management::story::StoryType;
use crate::product_management::story::story_type_from_labels_or_title;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn start_story_migrated_path_emits_pickup_gate_and_state_events() {
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        203,
        "t".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = crate::product_management::story::StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(203));
    args.insert(
        "pickup_paraphrase".into(),
        serde_json::json!("paraphrase content"),
    );
    args.insert(
        "falsifier_reframing".into(),
        serde_json::json!("reframing content"),
    );
    let result = handle_start_story(&args, "dev:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(203).unwrap();
    assert_eq!(story.state, StoryState::InProgress);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates.iter().any(|g| g.gate == Gate::Pickup
            && g.payload.as_deref().is_some_and(
                |p| p.contains("paraphrase content") && p.contains("reframing content")
            )),
        "gate_comments should contain Pickup with both payload sections: {gates:?}"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 0,
        "post-#1292: start_story drains synchronously; pending is empty by return"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "3 events still pushed to adapter (just drained synchronously now)"
    );
    let reap_count = pushed
        .iter()
        .filter(|e| matches!(e, BoardEvent::TagRemoved { number: 203, .. }))
        .count();
    assert_eq!(
        reap_count, 1,
        "post-#1156 pre-filter: only present-stale 'ready' label reaped"
    );
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 203,
            gate: Gate::Pickup,
            payload: Some(p),
            ..
        } if p.contains("paraphrase content") && p.contains("reframing content")
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 203,
            new_state: StoryState::InProgress,
            ..
        }
    ));
}

#[tokio::test]
async fn deliver_story_migrated_path_emits_delivered_gate_with_observation_and_state_events() {
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        204,
        "t".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = crate::product_management::story::StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(204));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("drove the falsifier scenario end-to-end"),
    );
    let result = handle_deliver_story(&args, "dev:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(204).unwrap();
    assert_eq!(story.state, StoryState::DeskCheck);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates.iter().any(|g| g.gate == Gate::Delivered
            && g.payload.as_deref() == Some("drove the falsifier scenario end-to-end")),
        "gate_comments should contain Delivered with the observation payload: {gates:?}"
    );

    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 3, "3 events expected post-#1156: {pushed:?}");
    let reap_count = pushed
        .iter()
        .filter(|e| matches!(e, BoardEvent::TagRemoved { number: 204, .. }))
        .count();
    assert_eq!(
        reap_count, 1,
        "post-#1156 pre-filter: only present-stale 'in progress' label reaped"
    );
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 204,
            gate: Gate::Delivered,
            payload: Some(p),
            ..
        } if p == "drove the falsifier scenario end-to-end"
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 204,
            new_state: StoryState::DeskCheck,
            ..
        }
    ));
}

#[tokio::test]
async fn accept_story_migrated_path_emits_tag_remove_gate_and_state_events() {
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        205,
        "t".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(205));
    args.insert("would_file_bug".into(), serde_json::json!(false));
    let result = handle_accept_story(&args, "owner:human", Some(&orchestrator), None).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(205).unwrap();
    assert_eq!(story.state, StoryState::Accepted);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates
            .iter()
            .any(|g| g.gate == Gate::Accepted && g.payload.is_none()),
        "gate_comments should contain Accepted with no payload: {gates:?}"
    );

    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 4, "4 events expected post-#1156: {pushed:?}");
    let reap_labels: Vec<&str> = pushed[..1]
        .iter()
        .filter_map(|e| match e {
            BoardEvent::TagRemoved {
                number: 205, label, ..
            } => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reap_labels,
        vec!["delivered"],
        "post-#1156 pre-filter: only present-stale 'delivered' label reaped"
    );
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 205,
            gate: Gate::Accepted,
            payload: None,
            ..
        }
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 205,
            new_state: StoryState::Accepted,
            ..
        }
    ));
    assert!(matches!(
        &pushed[3],
        BoardEvent::StoryClosed {
            number: 205,
            reason: None,
            ..
        }
    ));
    assert!(
        store.story(205).expect("story exists").is_closed,
        "is_closed flag must be set after StoryClosed event applies"
    );
}

#[tokio::test]
async fn accept_story_on_delivered_story_post_slice_5b_drains_full_cascade() {
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        5070,
        "[Bug] post-slice-5b accept-on-delivered cascade".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(
        5070,
        vec![
            "delivered".to_string(),
            "ready for accept".to_string(),
            "desk-check".to_string(),
        ],
    );
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(5070));
    args.insert("would_file_bug".into(), serde_json::json!(false));
    handle_accept_story(&args, "owner:human", Some(&orch), None)
        .await
        .unwrap();

    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        6,
        "post-#1156 pre-filter accept_story emits 6 events (3 TagRemoved + GateCommentAppended + StateTransitioned + StoryClosed): {pushed:?}"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::TagRemoved { number: 5070, label, .. } if label == "delivered"
        )),
        "must reap 'delivered' label (post-slice-5b: pass_desk_check sets this)"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::TagRemoved { number: 5070, label, .. } if label == "ready for accept"
        )),
        "must reap 'ready for accept' label (legacy back-compat for pre-slice-5b stories)"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::TagRemoved { number: 5070, label, .. } if label == "desk-check"
        )),
        "must reap 'desk-check' label (#507 fix: covers DeskCheck pre-state class post-slice-5b)"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::StateTransitioned {
                number: 5070,
                new_state: StoryState::Accepted,
                ..
            }
        )),
        "must emit StateTransitioned(Accepted) → adapter add_label('accepted')"
    );
}

#[tokio::test]
async fn reject_story_migrated_path_emits_tag_remove_gate_with_reason_and_state_events() {
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Gate, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        206,
        "t".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(206));
    args.insert("reason".into(), serde_json::json!("scope mismatch"));
    let result = handle_reject_story(&args, "owner:human", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(206).unwrap();
    assert_eq!(story.state, StoryState::Ready);
    let gates = story.gate_comments.as_ref().expect("gate_comments seeded");
    assert!(
        gates
            .iter()
            .any(|g| g.gate == Gate::Rejected && g.payload.as_deref() == Some("scope mismatch")),
        "gate_comments should contain Rejected with reason payload: {gates:?}"
    );

    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "post-#1156: 1 TagRemoved + Gate + State; pushed: {pushed:?}"
    );
    let reap_labels: Vec<&str> = pushed[..1]
        .iter()
        .filter_map(|e| match e {
            BoardEvent::TagRemoved {
                number: 206, label, ..
            } => Some(label.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        reap_labels,
        vec!["delivered"],
        "post-#1156 pre-filter: only present-stale 'delivered' label reaped"
    );
    assert!(matches!(
        &pushed[1],
        BoardEvent::GateCommentAppended {
            number: 206,
            gate: Gate::Rejected,
            payload: Some(p),
            ..
        } if p == "scope mismatch"
    ));
    assert!(matches!(
        &pushed[2],
        BoardEvent::StateTransitioned {
            number: 206,
            new_state: StoryState::Ready,
            ..
        }
    ));
}

#[tokio::test]
async fn create_story_migrated_path_assigns_number_and_emits_story_created_event() {
    use crate::product_management::board::BoardEvent;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        50,
        "seed".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    )]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert(
        "title".into(),
        serde_json::json!("[Bug] live-verify failure"),
    );
    args.insert(
        "user_job".into(),
        serde_json::json!("As an agent (pm or dev), I want the live-verify failure tracked"),
    );
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("I would reject if..."),
    );
    args.insert("body".into(), serde_json::json!("author note"));

    let result = handle_create_story(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");
    let response = result.unwrap();
    assert!(
        response.contains("#51"),
        "response should carry the source-assigned number 51 (max+1 over seed 50): {response}"
    );

    let story = store
        .story(51)
        .expect("StoryCreated event applied to store");
    assert_eq!(story.title, "[Bug] live-verify failure");
    assert_eq!(story.state, StoryState::Icebox);
    assert_eq!(
        story.story_type,
        StoryType::Bug,
        "story_type derived from title prefix [Bug]"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 2,
        "two events drained: StoryCreated + the use-axis-derived acceptor TagAdded (#1787)"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 2);
    assert!(matches!(
        &pushed[0],
        BoardEvent::StoryCreated {
            number: 51,
            initial_state: StoryState::Icebox,
            story_type: StoryType::Bug,
            ..
        }
    ));
    assert!(
        matches!(
            &pushed[1],
            BoardEvent::TagAdded { number: 51, label, .. } if label.as_str() == "awaiting team"
        ),
        "the 'As an agent (pm or dev)' multi-role use-axis derives the team acceptor, enqueued as awaiting-team at create (#1787); got {:?}",
        &pushed[1]
    );
}

#[tokio::test]
async fn deliver_story_migrated_path_uses_store_title_not_tracker() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        207,
        "store-titled-207".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = crate::product_management::story::StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let dir = tempfile::TempDir::new().unwrap();
    let (tx, _rx0) = tokio::sync::broadcast::channel(16);
    let session = Arc::new(crate::session::Session::new(
        dir.path().to_path_buf(),
        "owner".to_string(),
        tx,
    ));
    let mut rx = session.subscribe();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(207));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("ran end-to-end"),
    );
    let result = crate::bridge::dispatch_deliver_story(
        args,
        "dev:claude-code",
        Some(&orchestrator),
        &session,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let mut combined = String::new();
    while let Ok(msg) = rx.try_recv() {
        combined.push_str(&msg.text);
        combined.push('\n');
    }
    assert!(
        combined.contains("store-titled-207"),
        "migrated deliver path must use the store-side title in the delivery signal broadcast via session; got: {combined}"
    );
}

#[tokio::test]
async fn start_story_leads_with_verbatim_reject_for_a_rejected_story() {
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Gate, GateComment, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let reason = "drag srt does not persist and does not have the same drag css as the iteration";
    let seeded = Story::new_minimal(
        1524,
        "ready-queue drag".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 6, 6, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_gate_comments(
        1524,
        vec![GateComment {
            gate: Gate::Rejected,
            ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 1, 0).unwrap(),
            actor: "owner".into(),
            payload: Some(reason.into()),
        }],
    );
    let store = Arc::new(crate::product_management::board::BoardStore::new());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(1524));
    args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
    args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
    let result = handle_start_story(&args, "dev:claude-code", Some(&orchestrator))
        .await
        .expect("start_story succeeds");

    assert!(
        result.contains(reason),
        "start_story must surface the VERBATIM reject reason at pickup: {result}"
    );
    let reject_pos = result.find(reason).expect("reason present in output");
    let progress_pos = result
        .find("marked as in progress")
        .expect("progress line present");
    assert!(
        reject_pos < progress_pos,
        "the reject must LEAD — appear before the marked-in-progress line: {result}"
    );
    assert!(
        result.contains("REWORK"),
        "the reject is framed as a REWORK lead at pickup: {result}"
    );
}

#[tokio::test]
async fn start_story_leads_with_the_latest_reject_not_an_older_one() {
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Gate, GateComment, Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let older = "older reject — already addressed in a prior pass";
    let latest = "latest reject — the active rework to ground on";
    let seeded = Story::new_minimal(
        1525,
        "twice-rejected".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 6, 6, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_gate_comments(
        1525,
        vec![
            GateComment {
                gate: Gate::Rejected,
                ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 0, 0).unwrap(),
                actor: "owner".into(),
                payload: Some(older.into()),
            },
            GateComment {
                gate: Gate::Delivered,
                ts: Utc.with_ymd_and_hms(2026, 6, 6, 10, 0, 0).unwrap(),
                actor: "dev".into(),
                payload: Some("re-delivered the rework".into()),
            },
            GateComment {
                gate: Gate::Rejected,
                ts: Utc.with_ymd_and_hms(2026, 6, 6, 11, 0, 0).unwrap(),
                actor: "owner".into(),
                payload: Some(latest.into()),
            },
        ],
    );
    let store = Arc::new(crate::product_management::board::BoardStore::new());
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(1525));
    args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
    args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
    let result = handle_start_story(&args, "dev:claude-code", Some(&orchestrator))
        .await
        .expect("start_story succeeds");

    assert!(
        result.contains(latest),
        "start_story must LEAD with the LATEST reject: {result}"
    );
    assert!(
        !result.contains(older),
        "the superseded older reject must NOT surface — only the latest leads: {result}"
    );
}

#[cfg(test)]
mod classify_for_pass_desk_check_tests {
    use super::*;
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    async fn orchestrator_with_seed(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            Arc::clone(&store),
            Arc::clone(&adapter),
        ));
        orch.refresh_all().await.unwrap();
        orch
    }

    fn make_story(number: u64, story_type: StoryType) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("story-{number}"),
            StoryState::Delivered,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        s.story_type = story_type;
        s
    }

    #[tokio::test]
    async fn refreshed_path_returns_feature_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(910, StoryType::Feature)]).await;
        assert_eq!(
            classify_for_pass_desk_check(910, Some(&orch)),
            StoryType::Feature
        );
    }

    #[tokio::test]
    async fn refreshed_path_returns_bug_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(911, StoryType::Bug)]).await;
        assert_eq!(
            classify_for_pass_desk_check(911, Some(&orch)),
            StoryType::Bug
        );
    }

    #[tokio::test]
    async fn refreshed_path_returns_chore_via_typed_story_type() {
        let orch = orchestrator_with_seed(vec![make_story(912, StoryType::Chore)]).await;
        assert_eq!(
            classify_for_pass_desk_check(912, Some(&orch)),
            StoryType::Chore
        );
    }

    #[tokio::test]
    async fn defaults_to_feature_when_refresh_absent() {
        assert_eq!(classify_for_pass_desk_check(913, None), StoryType::Feature);
    }

    #[tokio::test]
    async fn defaults_to_feature_when_story_not_in_cache() {
        let orch = orchestrator_with_seed(vec![]).await;
        assert_eq!(
            classify_for_pass_desk_check(914, Some(&orch)),
            StoryType::Feature
        );
    }
}
