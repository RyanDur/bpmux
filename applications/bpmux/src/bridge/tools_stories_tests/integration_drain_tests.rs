#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::product_management::story::StoryId;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn start_story_blocks_when_acceptance_test_path_does_not_exist() {
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(882),
        "[Feature] missing-test-file".into(),
        BoardState::Ready,
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
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(883),
        "[Chore] no test required".into(),
        BoardState::Ready,
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
async fn reject_story_drains_synchronously_so_gh_side_propagates_before_return() {
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        StoryId::new(4690),
        "[Bug] cascade-drain-on-reject".into(),
        BoardState::Accepted,
        Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    seeded.is_closed = true;
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(4690, vec!["bug".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(4690));
    args.insert("reason".into(), serde_json::json!("cascade drain test"));
    handle_reject_story(&args, "owner:human", Some(&orch))
        .await
        .unwrap();

    let pushed = adapter.pushed_events();
    assert!(
        pushed.iter().any(
            |e| matches!(e, StoryEvent::StoryReopened { id, .. } if *id == StoryId::new(4690))
        ),
        "reject_story must drain StoryReopened to adapter BEFORE returning (#469 cascade-fix). Without internal drain, the GH-side reopen does not propagate until next handler triggers drain. Pushed: {pushed:?}"
    );
    let labels = adapter.labels_for(4690);
    assert!(
        !labels.iter().any(|l| l == "accepted"),
        "the state-transition export strips the stale accepted label at the adapter BEFORE the handler returns; got {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == "ready"),
        "the state-transition export applies the new state label at the adapter; got {labels:?}"
    );
}

#[tokio::test]
async fn fail_desk_check_drains_synchronously_so_gh_side_propagates_before_return() {
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
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
        BoardState::InProgress(Stage::DeskCheck.id()),
        Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(4691, vec!["bug".to_string()]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(4691));
    args.insert("reason".into(), serde_json::json!("cascade drain test"));
    handle_fail_desk_check(&args, "designer:test", Some(&orch))
        .await
        .unwrap();

    let pushed = adapter.pushed_events();
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            StoryEvent::StateTransitioned { id, new_state, .. } if *id == StoryId::new(4691) && *new_state == BoardState::InProgress(Stage::Dev.id())
        )),
        "fail_desk_check must drain StateTransitioned(InProgress) to adapter BEFORE returning. Pushed: {pushed:?}"
    );
}

#[tokio::test]
async fn reject_after_accept_strips_the_accepted_label_and_reopens() {
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{BoardState, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        StoryId::new(469),
        "[Bug] rework-after-accept".into(),
        BoardState::Accepted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    seeded.is_closed = true;
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
    assert!(
        !story.is_closed,
        "store must reflect re-opened state post-reject"
    );
    assert_eq!(story.state, BoardState::Ready);
}

#[tokio::test]
async fn reject_on_non_accepted_skips_tag_remove_accepted_and_story_reopened() {
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
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

#[tokio::test]
async fn deliver_story_propagates_drain_failure_at_label_remove() {
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{BoardState, Stage, Story};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(450),
        "[Chore] label-leak repro".into(),
        BoardState::InProgress(Stage::Dev.id()),
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
        result.is_err(),
        "deliver_story must surface drain failure load-bearing: {result:?}"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("drain after deliver failed"),
        "error must name the deliver-drain stage: {err}"
    );
}
