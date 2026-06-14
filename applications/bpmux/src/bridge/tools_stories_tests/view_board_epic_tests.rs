#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::board::domain::StoryType;
use crate::session::velocity::classify_story;
use crate::transport::protocol::ClientMessage;

#[tokio::test]
async fn set_parent_epic_migrated_path_emits_parent_epic_set_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        300,
        "child".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let mut epic = Story::new_minimal(
        457,
        "epic story".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    adapter.seed_stories(vec![child, epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(300));
    args.insert("epic_number".into(), serde_json::json!(457));
    let result = handle_set_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(300).unwrap();
    assert_eq!(story.parent_epic, Some(457));

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::ParentEpicSet {
            number: 300,
            epic_number: 457,
            ..
        }
    ));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_existent_epic() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        310,
        "orphan".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![child]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(310));
    args.insert("epic_number".into(), serde_json::json!(9999));
    let result = handle_set_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_epic_parent() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        311,
        "child".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let mut not_epic = Story::new_minimal(
        458,
        "ordinary feature".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    not_epic.story_type = StoryType::Feature;
    adapter.seed_stories(vec![child, not_epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(311));
    args.insert("epic_number".into(), serde_json::json!(458));
    let result = handle_set_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not an Epic"));
}

#[tokio::test]
async fn unset_parent_epic_migrated_path_emits_parent_epic_unset_event() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut seeded = Story::new_minimal(
        301,
        "child".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    seeded.parent_epic = Some(457);
    adapter.seed_stories(vec![seeded]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(301));
    let result = handle_unset_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    let story = store.story(301).unwrap();
    assert_eq!(story.parent_epic, None);

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        BoardEvent::ParentEpicUnset { number: 301, .. }
    ));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_pm_role() {
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(1));
    args.insert("epic_number".into(), serde_json::json!(2));
    let result = handle_set_parent_epic(&args, "dev:claude-code", None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[tokio::test]
async fn set_parent_epic_rejects_self_parent() {
    let mut args = serde_json::Map::new();
    args.insert("number".into(), serde_json::json!(42));
    args.insert("epic_number".into(), serde_json::json!(42));
    let result = handle_set_parent_epic(&args, "pm:claude-code", None).await;
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .contains("cannot be its own parent epic")
    );
}

#[tokio::test]
async fn create_story_with_parent_epic_emits_story_created_then_parent_epic_set() {
    use crate::board::BoardEvent;
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut epic = Story::new_minimal(
        500,
        "epic story".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    adapter.seed_stories(vec![epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("title".into(), serde_json::json!("child of epic"));
    args.insert("user_job".into(), serde_json::json!("user job here"));
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("would reject if X"),
    );
    args.insert("parent_epic".into(), serde_json::json!(500));
    let result = handle_create_story(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(
        result.is_ok(),
        "create-with-parent should succeed: {result:?}"
    );
    assert!(result.unwrap().contains("under epic #500"));

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 2, "StoryCreated + ParentEpicSet both drain");
    let pushed = adapter.pushed_events();
    assert!(matches!(pushed[0], BoardEvent::StoryCreated { .. }));
    assert!(matches!(
        pushed[1],
        BoardEvent::ParentEpicSet {
            epic_number: 500,
            ..
        }
    ));
}

#[tokio::test]
async fn create_story_with_non_epic_parent_rejects_before_create() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let mut not_epic = Story::new_minimal(
        501,
        "not epic".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    not_epic.story_type = StoryType::Feature;
    adapter.seed_stories(vec![not_epic]);
    let orchestrator: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("title".into(), serde_json::json!("orphan"));
    args.insert("user_job".into(), serde_json::json!("user job here"));
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("would reject if X"),
    );
    args.insert("parent_epic".into(), serde_json::json!(501));
    let result = handle_create_story(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not an Epic"));
    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 0, "no events emitted on parent-validation reject");
}

#[tokio::test]
async fn view_board_renders_epic_summary_section_with_child_progress() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        700,
        "umbrella story".into(),
        StoryState::Ready,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    let mut child_a =
        Story::new_minimal(701, "child a".into(), StoryState::Accepted, ts, "pm".into());
    child_a.parent_epic = Some(700);
    child_a.is_closed = true;
    let mut child_b = Story::new_minimal(
        702,
        "child b".into(),
        StoryState::Delivered,
        ts,
        "pm".into(),
    );
    child_b.parent_epic = Some(700);
    let mut child_c = Story::new_minimal(
        703,
        "child c".into(),
        StoryState::InProgress,
        ts,
        "pm".into(),
    );
    child_c.parent_epic = Some(700);
    adapter.seed_stories(vec![epic, child_a, child_b, child_c]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("EPICS (1)"),
        "expected EPICS section header in:\n{out}"
    );
    assert!(
        out.contains(
            "#700 [in-progress] umbrella story (3 children: 1 accepted, 0 retired, 2 active, 0 icebox)"
        ),
        "expected Epic-summary line with derived state + progress in:\n{out}"
    );
    assert!(
        out.contains("↳ #702 child b (epic #700)"),
        "expected child glyph + epic-membership tag in:\n{out}"
    );
    assert!(
        out.contains("↳ #703 child c (epic #700)"),
        "expected in-progress child glyph in:\n{out}"
    );
}

#[tokio::test]
async fn view_board_omits_epic_section_when_no_epics_present() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let plain = Story::new_minimal(
        800,
        "plain feature".into(),
        StoryState::Ready,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![plain]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        !out.contains("EPICS"),
        "EPICS section must NOT appear when no Epics: {out}"
    );
}

#[tokio::test]
async fn view_board_filters_epic_stories_out_of_state_lanes() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        710,
        "another umbrella".into(),
        StoryState::Ready,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    adapter.seed_stories(vec![epic]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("EPICS (1)") && out.contains("#710"),
        "Epic must appear in EPICS section: {out}"
    );
    assert!(
        !out.contains("  #710 another umbrella")
            || out
                .lines()
                .filter(|line| line.contains("another umbrella"))
                .count()
                == 1,
        "Epic story must appear once (in EPICS only), not in state-lane: {out}"
    );
    let ready_section = out
        .split("READY (")
        .nth(1)
        .unwrap_or("")
        .split("\n\n")
        .next()
        .unwrap_or("");
    assert!(
        !ready_section.contains("710"),
        "Epic must be filtered OUT of READY lane: ready_section={ready_section}"
    );
}

#[tokio::test]
async fn view_board_epic_with_delivered_children_derives_in_progress_post_1450() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        720,
        "delivered umbrella".into(),
        StoryState::Ready,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    let mut a = Story::new_minimal(721, "a".into(), StoryState::Delivered, ts, "pm".into());
    a.parent_epic = Some(720);
    let mut b = Story::new_minimal(722, "b".into(), StoryState::Delivered, ts, "pm".into());
    b.parent_epic = Some(720);
    adapter.seed_stories(vec![epic, a, b]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("#720 [in-progress] delivered umbrella"),
        "Post-#1450 truth-table: Delivered children are active-state class → Epic derives in-progress; got: {out}"
    );
}

#[tokio::test]
async fn view_board_epic_ignores_explicit_state_when_children_present_post_1450() {
    use crate::board::BoardStore;
    use crate::board::RefreshDriver;
    use crate::board::SyncOrchestrator;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let mut epic = Story::new_minimal(
        730,
        "owner-accepted umbrella".into(),
        StoryState::Accepted,
        ts,
        "pm".into(),
    );
    epic.story_type = StoryType::Epic;
    let mut a = Story::new_minimal(731, "a".into(), StoryState::InProgress, ts, "pm".into());
    a.parent_epic = Some(730);
    adapter.seed_stories(vec![epic, a]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(
        Arc::clone(&store),
        Arc::clone(&adapter),
    ));
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("#730 [in-progress] owner-accepted umbrella"),
        "Post-#1450: explicit_state ignored — substrate IS source-of-truth (META-CANON #1); active child wins; got: {out}"
    );
}
