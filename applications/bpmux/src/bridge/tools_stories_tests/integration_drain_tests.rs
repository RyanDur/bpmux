#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::product_management::story::StoryType;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn start_story_blocks_when_acceptance_test_path_does_not_exist() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        882,
        "[Feature] missing-test-file".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Feature;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(882));
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
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        883,
        "[Chore] no test required".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(883));
    args.insert("pickup_paraphrase".into(), serde_json::json!("p"));
    args.insert("falsifier_reframing".into(), serde_json::json!("f"));
    let result = handle_start_story(&args, "dev:claude-code", Some(&orch)).await;
    assert!(result.is_ok(), "Chore exempt: {result:?}");
}

#[tokio::test]
async fn reject_story_drains_synchronously_so_gh_side_propagates_before_return() {
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
    let mut seeded = Story::new_minimal(
        4690,
        "[Bug] cascade-drain-on-reject".into(),
        StoryState::Accepted,
        Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    seeded.story_type = StoryType::Bug;
    seeded.is_closed = true;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(4690));
    args.insert("reason".into(), serde_json::json!("cascade drain test"));
    handle_reject_story(&args, "owner:human", Some(&orch))
        .await
        .unwrap();

    let pushed = adapter.pushed_events();
    assert!(
        pushed
            .iter()
            .any(|e| matches!(e, BoardEvent::StoryReopened { number: 4690, .. })),
        "reject_story must drain StoryReopened to adapter BEFORE returning (#469 cascade-fix). Without internal drain, the GH-side reopen does not propagate until next handler triggers drain. Pushed: {pushed:?}"
    );
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::TagRemoved { number: 4690, label, .. } if label == "accepted"
        )),
        "reject_story must drain TagRemoved(accepted) to adapter BEFORE returning. Pushed: {pushed:?}"
    );
}

#[tokio::test]
async fn fail_desk_check_drains_synchronously_so_gh_side_propagates_before_return() {
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
    let mut seeded = Story::new_minimal(
        4691,
        "[Bug] cascade-drain-on-fail-desk-check".into(),
        StoryState::DeskCheck,
        Utc.with_ymd_and_hms(2026, 4, 27, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    seeded.story_type = StoryType::Bug;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(4691));
    args.insert("reason".into(), serde_json::json!("cascade drain test"));
    handle_fail_desk_check(&args, "designer:test", Some(&orch))
        .await
        .unwrap();

    let pushed = adapter.pushed_events();
    assert!(
        pushed.iter().any(|e| matches!(
            e,
            BoardEvent::StateTransitioned { number: 4691, new_state, .. } if *new_state == StoryState::InProgress
        )),
        "fail_desk_check must drain StateTransitioned(InProgress) to adapter BEFORE returning. Pushed: {pushed:?}"
    );
}

#[tokio::test]
async fn reject_after_accept_emits_tag_remove_accepted_and_story_reopened() {
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
    let mut seeded = Story::new_minimal(
        469,
        "[Bug] rework-after-accept".into(),
        StoryState::Accepted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    seeded.story_type = StoryType::Bug;
    seeded.is_closed = true;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(469));
    args.insert("reason".into(), serde_json::json!("rework needed"));
    let result = handle_reject_story(&args, "owner:human", Some(&orch)).await;
    assert!(result.is_ok(), "reject should succeed: {result:?}");

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();

    let tag_removed_accepted = pushed.iter().any(|e| {
        matches!(
            e,
            BoardEvent::TagRemoved { number: 469, label, .. } if label == "accepted"
        )
    });
    assert!(
        tag_removed_accepted,
        "reject_story on previously-Accepted must emit TagRemoved(accepted): {pushed:?}"
    );

    let story_reopened = pushed
        .iter()
        .any(|e| matches!(e, BoardEvent::StoryReopened { number: 469, .. }));
    assert!(
        story_reopened,
        "reject_story on previously-Accepted must emit StoryReopened: {pushed:?}"
    );

    let story = store.story(469).unwrap();
    assert!(
        !story.is_closed,
        "store must reflect re-opened state post-reject"
    );
    assert_eq!(story.state, StoryState::Ready);
}

#[tokio::test]
async fn reject_on_non_accepted_skips_tag_remove_accepted_and_story_reopened() {
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
    let mut seeded = Story::new_minimal(
        470,
        "[Bug] reject-from-delivered".into(),
        StoryState::Delivered,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "owner".into(),
    );
    seeded.story_type = StoryType::Bug;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(470));
    args.insert("reason".into(), serde_json::json!("not done"));
    handle_reject_story(&args, "owner:human", Some(&orch))
        .await
        .unwrap();

    orch.drain_pending().await.unwrap();
    let pushed = adapter.pushed_events();
    let any_reopen = pushed
        .iter()
        .any(|e| matches!(e, BoardEvent::StoryReopened { number: 470, .. }));
    assert!(
        !any_reopen,
        "reject on non-Accepted must NOT emit StoryReopened: {pushed:?}"
    );
}

#[tokio::test]
async fn deliver_story_propagates_drain_failure_at_label_remove() {
    use crate::product_management::board::BoardStore;
    use crate::product_management::board::RefreshDriver;
    use crate::product_management::board::SyncOrchestrator;
    use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        450,
        "[Chore] label-leak repro".into(),
        StoryState::InProgress,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "dev".into(),
    );
    seeded.story_type = StoryType::Chore;
    adapter.seed_stories(vec![seeded]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();

    adapter.set_push_failures(true);

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(450));
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
