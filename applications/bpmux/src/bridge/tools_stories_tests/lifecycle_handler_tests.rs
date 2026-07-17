#![allow(unused_imports)]

use super::super::tools_stories::*;
use product_management::story::{StoryId, StoryKind};
use protocol::ClientMessage;

fn td() -> crate::store::Dispatch {
    crate::store::Dispatch::recording().0
}

#[tokio::test]
async fn start_story_migrated_path_emits_pickup_gate_and_state_events() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Stage, Story, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(203),
        "t".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(203, vec!["chore".to_string()]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orchestrator);
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(203));
    args.insert(
        "pickup_paraphrase".into(),
        serde_json::json!("paraphrase content"),
    );
    args.insert(
        "falsifier_reframing".into(),
        serde_json::json!("reframing content"),
    );
    let result = handle_start_story(
        &args,
        "dev:claude-code",
        "dev",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(203)))
        .unwrap();
    assert_eq!(story.state, Work::InProgress);
    let gates = story.verdicts.as_ref().expect("verdicts seeded");
    assert!(
        gates.iter().any(|g| g.verdict
            == Verdict::Succeed {
                step: "ready".into()
            }
            && g.payload.as_deref().is_some_and(
                |p| p.contains("paraphrase content") && p.contains("reframing content")
            )),
        "verdicts should contain Pickup with both payload sections: {gates:?}"
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(
        drained, 2,
        "start_story returns optimistically — the pickup gate + state events stay pending and are pushed on the async drain, not before return"
    );
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        2,
        "gate + state events pushed; state-label reaping is the transition export's concern now"
    );
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::VerdictCast {
            verdict: Verdict::Succeed { step },
            payload: Some(p),
            ..
        } if pushed[0].id == StoryId::new(203) && step.as_str() == "ready" && p.contains("paraphrase content") && p.contains("reframing content")
    ));
    assert!(matches!(
        &pushed[1].kind,
        StoryEventKind::StateTransitioned {
            new_state: Work::InProgress,
            ..
        } if pushed[1].id == StoryId::new(203)
    ));
    let labels = adapter.labels_for(203);
    assert!(
        !labels.iter().any(|l| l == "ready") && labels.iter().any(|l| l == "in progress"),
        "the transition export strips the stale ready label and applies in progress at the adapter; got {labels:?}"
    );
}

#[tokio::test]
async fn deliver_story_migrated_path_emits_delivered_gate_with_observation_and_state_events() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Stage, Story, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(204),
        "t".into(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(204, vec!["chore".to_string()]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orchestrator);
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(204));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("drove the falsifier scenario end-to-end"),
    );
    let result = handle_deliver_story(
        &args,
        "dev:claude-code",
        "dev",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(204)))
        .unwrap();
    assert_eq!(story.state, Work::InProgress);
    let gates = story.verdicts.as_ref().expect("verdicts seeded");
    assert!(
        gates.iter().any(|g| g.verdict
            == Verdict::Succeed {
                step: "in-progress".into()
            }
            && g.payload.as_deref() == Some("drove the falsifier scenario end-to-end")),
        "verdicts should contain Delivered with the observation payload: {gates:?}"
    );

    orchestrator.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        2,
        "gate + state events pushed; state-label reaping is the transition export's concern now: {pushed:?}"
    );
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::VerdictCast {
            verdict: Verdict::Succeed { step },
            payload: Some(p),
            ..
        } if pushed[0].id == StoryId::new(204) && step.as_str() == "in-progress" && p == "drove the falsifier scenario end-to-end"
    ));
    assert!(
        matches!(
            &pushed[1].kind,
            StoryEventKind::StateTransitioned {
                new_state: Work::InProgress,
                placement: Some(stage),
                ..
            } if pushed[1].id == StoryId::new(204) && stage.as_str() == "desk-check"
        ),
        "deliver carries the desk-check fine placement on the transition (the fine stage is set even though the coarse state stays InProgress): {pushed:?}"
    );
    let labels = adapter.labels_for(204);
    assert!(
        labels.iter().any(|l| l == "in progress") && !labels.iter().any(|l| l == "desk-check"),
        "de-leak: GitHub carries only the coarse 'in progress' label; the fine desk-check stage stays local in placements.json, never on GitHub; got {labels:?}"
    );
}

#[tokio::test]
async fn accept_story_migrated_path_emits_gate_state_and_close_events() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(205),
        "t".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orchestrator);
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(205));
    args.insert("would_file_bug".into(), serde_json::json!(false));
    let result = handle_accept_story(
        &args,
        "owner:human",
        "owner",
        None,
        Some(&orchestrator),
        &select,
        None,
        &None,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(205)))
        .unwrap();
    assert_eq!(story.state, Work::Accepted);
    let gates = story.verdicts.as_ref().expect("verdicts seeded");
    assert!(
        gates.iter().any(|g| g.verdict
            == Verdict::Succeed {
                step: "acceptance".into()
            }
            && g.payload.is_none()),
        "verdicts should contain Accepted with no payload: {gates:?}"
    );

    orchestrator.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "gate + state + close events pushed; state-label reaping is the transition export's concern now: {pushed:?}"
    );
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::VerdictCast {
            verdict: Verdict::Succeed { step },
            payload: None,
            ..
        } if pushed[0].id == StoryId::new(205) && step.as_str() == "acceptance"
    ));
    assert!(matches!(
        &pushed[1].kind,
        StoryEventKind::StateTransitioned {
            new_state: Work::Accepted,
            ..
        } if pushed[1].id == StoryId::new(205)
    ));
    assert!(matches!(
        &pushed[2].kind,
        StoryEventKind::StoryClosed {
            reason: None,
            ..
        } if pushed[2].id == StoryId::new(205)
    ));
    let labels = adapter.labels_for(205);
    assert!(
        !labels.iter().any(|l| l == "delivered") && labels.iter().any(|l| l == "accepted"),
        "the transition export strips the stale delivered label and applies accepted at the adapter (and the close keeps accepted); got {labels:?}"
    );
}

#[tokio::test]
async fn accept_story_on_delivered_story_post_slice_5b_drains_full_cascade() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(5070),
        "[Bug] post-slice-5b accept-on-delivered cascade".into(),
        Work::Acceptance,
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
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orch);
    orch.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(5070));
    args.insert("would_file_bug".into(), serde_json::json!(false));
    handle_accept_story(
        &args,
        "owner:human",
        "owner",
        None,
        Some(&orch),
        &select,
        None,
        &None,
    )
    .await
    .unwrap();

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        3,
        "gate + state + close events pushed; state-label reaping is the transition export's concern now: {pushed:?}"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            &e.kind,
            StoryEventKind::StateTransitioned {
                new_state: Work::Accepted,
                ..
            } if e.id == StoryId::new(5070)
        )),
        "must emit StateTransitioned(Accepted) → adapter strips stale state labels + adds 'accepted'"
    );
    let labels = adapter.labels_for(5070);
    for stale in ["delivered", "ready for accept", "desk-check"] {
        assert!(
            !labels.iter().any(|l| l == stale),
            "the transition export must strip the stale '{stale}' label at the adapter (canonical + legacy + #507 DeskCheck classes); got {labels:?}"
        );
    }
    assert!(
        labels.iter().any(|l| l == "accepted"),
        "the transition export applies the accepted label at the adapter (and the close keeps it); got {labels:?}"
    );
}

#[tokio::test]
async fn reject_story_migrated_path_emits_gate_with_reason_and_state_events() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(206),
        "t".into(),
        Work::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let current = product_management::iteration::Iteration {
        id: product_management::iteration::IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: vec![StoryId::new(206)],
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    };
    let store = crate::store::SeededStore::new(vec![], vec![current], vec![]);
    store.bind_refresh(&orchestrator);
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(206));
    args.insert("reason".into(), serde_json::json!("scope mismatch"));
    let result = handle_reject_story(
        &args,
        "owner:human",
        "owner",
        None,
        Some(&orchestrator),
        &select,
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(206)))
        .unwrap();
    assert_eq!(story.state, Work::NotStarted);
    let gates = story.verdicts.as_ref().expect("verdicts seeded");
    assert!(
        gates.iter().any(|g| g.verdict
            == Verdict::Fail {
                step: "acceptance".into()
            }
            && g.payload.as_deref() == Some("scope mismatch")),
        "verdicts should contain Rejected with reason payload: {gates:?}"
    );

    orchestrator.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    assert_eq!(
        pushed.len(),
        2,
        "gate + state events pushed; state-label reaping is the transition export's concern now: {pushed:?}"
    );
    assert!(matches!(
        &pushed[0].kind,
        StoryEventKind::VerdictCast {
            verdict: Verdict::Fail { step },
            payload: Some(p),
            ..
        } if pushed[0].id == StoryId::new(206) && step.as_str() == "acceptance" && p == "scope mismatch"
    ));
    assert!(matches!(
        &pushed[1].kind,
        StoryEventKind::StateTransitioned {
            new_state: Work::NotStarted,
            ..
        } if pushed[1].id == StoryId::new(206)
    ));
    let labels = adapter.labels_for(206);
    assert!(
        !labels.iter().any(|l| l == "delivered") && labels.iter().any(|l| l == "ready"),
        "the transition export strips the stale delivered label and applies ready at the adapter; got {labels:?}"
    );
}

#[tokio::test]
async fn create_story_migrated_path_assigns_id_and_emits_story_created_event() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::events::{StoryEvent, StoryEventKind};
    use product_management::story::{Story, StoryKind, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(vec![Story::new_minimal(
        StoryId::new(50),
        "seed".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    )]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let bound_select = crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
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

    let select = crate::store::Select::new(std::sync::Weak::new());
    let result = handle_create_story(
        &args,
        "pm:claude-code",
        "pm",
        None,
        Some(&orchestrator),
        &select,
        &td(),
    )
    .await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");
    let response = result.unwrap();
    assert!(
        response.contains("#51"),
        "response should carry the source-assigned id 51 (max+1 over seed 50): {response}"
    );

    let story = bound_select
        .select(|state| product_management::story::slice::get(&state.stories, StoryId::new(51)))
        .expect("StoryCreated event applied to store");
    assert_eq!(story.title, "[Bug] live-verify failure");
    assert_eq!(story.state, Work::NotStarted);
    assert_eq!(
        story.story_type,
        StoryKind::Bug,
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
        &pushed[0].kind,
        StoryEventKind::StoryCreated {
            initial_state: Work::NotStarted,
            story_type: StoryKind::Bug,
            ..
        } if pushed[0].id == StoryId::new(51)
    ));
    assert!(
        matches!(
            &pushed[1].kind,
            StoryEventKind::AcceptorAssigned { acceptor, .. } if pushed[1].id == StoryId::new(51) && *acceptor == product_management::story::Acceptor::Team
        ),
        "the 'As an agent (pm or dev)' multi-role use-axis derives the team acceptor, assigned at create (#1787); got {:?}",
        &pushed[1]
    );
}

#[tokio::test]
async fn deliver_story_migrated_path_uses_store_title_not_tracker() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{Stage, Story, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(207),
        "store-titled-207".into(),
        Work::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(207, vec!["chore".to_string()]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orchestrator);
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let dir = tempfile::TempDir::new().unwrap();
    let (tx, _rx0) = tokio::sync::broadcast::channel(16);
    let hub = Arc::new(crate::hub::Hub::new(
        dir.path().to_path_buf(),
        "owner".to_string(),
        tx,
    ));
    let mut rx = hub.subscribe();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(207));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("ran end-to-end"),
    );
    let result = crate::bridge::dispatch_deliver_story(
        args,
        "dev:claude-code",
        "dev",
        None,
        Some(&orchestrator),
        &select,
        &hub,
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
        "migrated deliver path must use the store-side title in the delivery signal broadcast via hub; got: {combined}"
    );
}

#[tokio::test]
async fn start_story_leads_with_verbatim_reject_for_a_rejected_story() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{CastVerdict, Story, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let reason = "drag srt does not persist and does not have the same drag css as the iteration";
    let seeded = Story::new_minimal(
        StoryId::new(1524),
        "ready-queue drag".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 6, 6, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
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
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orchestrator);
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(1524));
    args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
    args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
    let result = handle_start_story(
        &args,
        "dev:claude-code",
        "dev",
        None,
        Some(&orchestrator),
        &select,
    )
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{CastVerdict, Story, Verdict, Work};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let older = "older reject — already addressed in a prior pass";
    let latest = "latest reject — the active rework to ground on";
    let seeded = Story::new_minimal(
        StoryId::new(1525),
        "twice-rejected".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 6, 6, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_verdicts(
        1525,
        vec![
            CastVerdict {
                verdict: Verdict::Fail {
                    step: "acceptance".into(),
                },
                ts: Utc.with_ymd_and_hms(2026, 6, 6, 9, 0, 0).unwrap(),
                actor: "owner".into(),
                payload: Some(older.into()),
            },
            CastVerdict {
                verdict: Verdict::Succeed {
                    step: "in-progress".into(),
                },
                ts: Utc.with_ymd_and_hms(2026, 6, 6, 10, 0, 0).unwrap(),
                actor: "dev".into(),
                payload: Some("re-delivered the rework".into()),
            },
            CastVerdict {
                verdict: Verdict::Fail {
                    step: "acceptance".into(),
                },
                ts: Utc.with_ymd_and_hms(2026, 6, 6, 11, 0, 0).unwrap(),
                actor: "owner".into(),
                payload: Some(latest.into()),
            },
        ],
    );
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&adapter),
        Arc::new(source_codec::gh_label_codec::GhLabelCodec),
    ));
    let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
    store.bind_refresh(&orchestrator);
    orchestrator.refresh_all().await.unwrap();
    let select = store.select();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(1525));
    args.insert("pickup_paraphrase".into(), serde_json::json!("paraphrase"));
    args.insert("falsifier_reframing".into(), serde_json::json!("reframing"));
    let result = handle_start_story(
        &args,
        "dev:claude-code",
        "dev",
        None,
        Some(&orchestrator),
        &select,
    )
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use product_management::story::{Story, Work};
    use std::sync::Arc;

    async fn select_with_seed(stories: Vec<Story>) -> crate::store::Select {
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
            Arc::clone(&adapter),
            Arc::new(source_codec::gh_label_codec::GhLabelCodec),
        ));
        let store = crate::store::SeededStore::new(vec![], vec![], vec![]);
        store.bind_refresh(&orch);
        orch.refresh_all().await.unwrap();
        let select = store.select();
        std::mem::forget(store);
        select
    }

    fn make_story(id: u64, story_type: StoryKind) -> Story {
        let mut s = Story::new_minimal(
            StoryId::new(id),
            format!("story-{id}"),
            Work::Acceptance,
            Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            "dev".into(),
        );
        s.story_type = story_type;
        s
    }

    #[tokio::test]
    async fn refreshed_path_returns_feature_via_typed_story_type() {
        let select = select_with_seed(vec![make_story(910, StoryKind::Feature)]).await;
        assert_eq!(
            classify_for_pass_desk_check(910, &select),
            StoryKind::Feature
        );
    }

    #[tokio::test]
    async fn refreshed_path_returns_bug_via_typed_story_type() {
        let select = select_with_seed(vec![make_story(911, StoryKind::Bug)]).await;
        assert_eq!(classify_for_pass_desk_check(911, &select), StoryKind::Bug);
    }

    #[tokio::test]
    async fn refreshed_path_returns_chore_via_typed_story_type() {
        let select = select_with_seed(vec![make_story(912, StoryKind::Chore)]).await;
        assert_eq!(classify_for_pass_desk_check(912, &select), StoryKind::Chore);
    }

    #[tokio::test]
    async fn defaults_to_feature_when_refresh_absent() {
        let select = crate::store::Select::new(std::sync::Weak::new());
        assert_eq!(
            classify_for_pass_desk_check(913, &select),
            StoryKind::Feature
        );
    }

    #[tokio::test]
    async fn defaults_to_feature_when_story_not_in_cache() {
        let select = select_with_seed(vec![]).await;
        assert_eq!(
            classify_for_pass_desk_check(914, &select),
            StoryKind::Feature
        );
    }
}
