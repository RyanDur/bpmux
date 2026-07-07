#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::product_management::story::StoryId;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn start_story_blocks_when_acceptance_test_path_does_not_exist() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(882),
        "[Feature] missing-test-file".into(),
        BoardState::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(882, vec!["feature".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(882));
    args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
    args.insert("falsifier_reframing".into(), serde_json::json!("f"));
    args.insert(
        "acceptance_test_path".into(),
        serde_json::json!("does/not/exist.spec.ts"),
    );
    let result = handle_start_story(&args, "dev:claude-code", Some(&orch)).await;
    assert!(result.is_err(), "missing path must block: {result:?}");
    assert!(result.unwrap_err().contains("does not exist"));
}

#[tokio::test]
async fn start_story_passes_chore_without_acceptance_test_path() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(883),
        "[Chore] no test required".into(),
        BoardState::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(883, vec!["chore".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(883));
    args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
    args.insert("falsifier_reframing".into(), serde_json::json!("f"));
    let result = handle_start_story(&args, "dev:claude-code", Some(&orch)).await;
    assert!(result.is_ok(), "Chore exempt: {result:?}");
}

#[tokio::test]
async fn reject_story_returns_optimistically_and_defers_gh_push_to_the_async_drain() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(4690),
        "[Bug] cascade-drain-on-reject".into(),
        BoardState::Accepted,
        Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(4690, vec!["bug".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], current_iteration_of(&[4690]), vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(4690));
    args.insert("reason".into(), serde_json::json!("cascade drain test"));
    handle_reject_story(&args, "owner:human", Some(&orch))
        .await
        .unwrap();

    assert!(
        adapter.pushed_events().is_empty(),
        "reject_story returns optimistically — the GH push is deferred to the async drain, so nothing is pushed before the handler returns: {:?}",
        adapter.pushed_events()
    );

    orch.drain_pending().await.unwrap();

    let pushed = adapter.pushed_events();
    assert!(
        pushed.iter().any(
            |e| matches!(e, StoryEvent::StoryReopened { id, .. } if *id == StoryId::new(4690))
        ),
        "the async drain propagates StoryReopened to the adapter (#469 cascade-fix): {pushed:?}"
    );
    let labels = adapter.labels_for(4690);
    assert!(
        !labels.iter().any(|l| l == "accepted"),
        "the state-transition export strips the stale accepted label at the adapter on drain; got {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == "ready"),
        "the state-transition export applies the new state label at the adapter on drain; got {labels:?}"
    );
}

#[tokio::test]
async fn fail_desk_check_returns_optimistically_and_defers_gh_push_to_the_async_drain() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Stage, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(4691),
        "[Bug] cascade-drain-on-fail-desk-check".into(),
        BoardState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(4691, vec!["bug".to_string(), "desk-check".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(4691));
    args.insert("reason".into(), serde_json::json!("cascade drain test"));
    handle_fail_desk_check(&args, "designer:test", Some(&orch))
        .await
        .unwrap();

    assert!(
        adapter.pushed_events().is_empty(),
        "fail_desk_check returns optimistically — the GH push is deferred to the async drain: {:?}",
        adapter.pushed_events()
    );

    orch.drain_pending().await.unwrap();

    let pushed = adapter.pushed_events();
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            StoryEvent::StateTransitioned { id, new_state, .. } if *id == StoryId::new(4691) && *new_state == BoardState::InProgress
        )),
        "the async drain propagates StateTransitioned(InProgress) to the adapter: {pushed:?}"
    );
}

#[tokio::test]
async fn reject_after_accept_strips_the_accepted_label_and_reopens() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(469),
        "[Bug] rework-after-accept".into(),
        BoardState::Accepted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(469, vec!["bug".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(469));
    args.insert("reason".into(), serde_json::json!("rework needed"));
    let result = handle_reject_story(&args, "owner:human", Some(&orch)).await;
    assert!(result.is_ok(), "reject should succeed: {result:?}");

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();

    let labels = adapter.labels_for(469);
    assert!(
        !labels.iter().any(|l| l == "accepted"),
        "reject_story on previously-Accepted must leave the adapter without the stale accepted label (the transition export strips it): {labels:?}"
    );

    let story_reopened = pushed
        .iter()
        .any(|e| matches!(e, StoryEvent::StoryReopened { id, .. } if *id == StoryId::new(469)));
    assert!(
        story_reopened,
        "reject_story on previously-Accepted must emit StoryReopened: {pushed:?}"
    );

    let story = orch.story(469).unwrap();
    assert_eq!(story.state, BoardState::NotStarted);
}

#[tokio::test]
async fn reject_on_non_accepted_skips_tag_remove_accepted_and_story_reopened() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(470),
        "[Bug] reject-from-delivered".into(),
        BoardState::Acceptance,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(470, vec!["bug".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(470));
    args.insert("reason".into(), serde_json::json!("not done"));
    handle_reject_story(&args, "owner:human", Some(&orch))
        .await
        .unwrap();

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    let any_reopen = pushed
        .iter()
        .any(|e| matches!(e, StoryEvent::StoryReopened { id, .. } if *id == StoryId::new(470)));
    assert!(
        !any_reopen,
        "reject on non-Accepted must NOT emit StoryReopened: {pushed:?}"
    );
}

fn current_iteration_of(members: &[u64]) -> Vec<crate::product_management::iteration::Iteration> {
    use crate::product_management::iteration::{Iteration, IterationId};
    use chrono::{TimeZone, Utc};
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: members.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }]
}

#[tokio::test]
async fn a_transition_out_of_the_board_pushes_no_state_label_and_strips_the_stale_one() {
    use crate::integrations::github::gh_label_codec::GhLabelCodec;
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::label_codec::LabelCodec;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(471),
        "[Bug] unschedule-label-mirror".into(),
        BoardState::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(471, vec!["bug".to_string(), "ready".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], current_iteration_of(&[]), vec![]);
    orch.refresh_all().await.unwrap();

    orch.mutate_sync(StoryEvent::StateTransitioned {
        meta: crate::sync::cache_fill_meta(),
        id: StoryId::new(471),
        new_state: BoardState::NotStarted,
        placement: None,
    })
    .unwrap();
    orch.drain_pending().await.unwrap();

    let labels = adapter.labels_for(471);
    assert!(
        !labels.iter().any(|l| l == "ready"),
        "the stale ready label is stripped when the story leaves the board; got {labels:?}"
    );
    assert!(
        !labels
            .iter()
            .any(|l| GhLabelCodec.is_workflow_state_label(l)),
        "icebox is label-absence — the mirror pushes NO state label for an off-board story; got {labels:?}"
    );
    assert!(
        !labels.iter().any(|l| l.is_empty()),
        "the label-less transition must never leak an empty-string label to the mirror; got {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == "bug"),
        "non-state labels survive the strip; got {labels:?}"
    );
}

#[tokio::test]
async fn a_transition_to_ready_for_a_current_iteration_member_pushes_the_ready_label() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(472),
        "[Feature] schedule-label-mirror".into(),
        BoardState::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(472, vec!["feature".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], current_iteration_of(&[472]), vec![]);
    orch.refresh_all().await.unwrap();

    orch.mutate_sync(StoryEvent::StateTransitioned {
        meta: crate::sync::cache_fill_meta(),
        id: StoryId::new(472),
        new_state: BoardState::NotStarted,
        placement: None,
    })
    .unwrap();
    orch.drain_pending().await.unwrap();

    let labels = adapter.labels_for(472);
    assert!(
        labels.iter().any(|l| l == "ready"),
        "a current-iteration member entering ready carries the human-readable ready mirror label; got {labels:?}"
    );
}

#[tokio::test]
async fn deliver_story_returns_optimistically_and_defers_push_failure_to_the_async_drain() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Stage, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(450),
        "[Chore] label-leak repro".into(),
        BoardState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(450, vec!["chore".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    adapter.set_push_failures(true);

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(450));
    args.insert(
        "verified_observation".into(),
        serde_json::json!("drove the label-leak failure-injection scenario"),
    );
    let result = handle_deliver_story(&args, "dev:claude-code", Some(&orch)).await;
    assert!(
        result.is_ok(),
        "deliver_story returns optimistically — the write succeeds locally even when the GH push will fail; the failure is reconciled on the async drain, not surfaced at return: {result:?}"
    );

    let drain = orch.drain_pending().await;
    assert!(
        drain.is_err(),
        "the deferred push failure surfaces on the async drain, not at deliver-return: {drain:?}"
    );
}
