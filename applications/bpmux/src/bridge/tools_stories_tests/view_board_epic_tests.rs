#![allow(unused_imports)]

use super::super::tools_stories::*;
use super::test_ci_state_store_green;
use crate::integrations::github::mapper::{decode_epic_from_story, is_epic_story};
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::product_management::epic::{Epic, EpicId};
use crate::product_management::story::{Story, StoryId};
use crate::transport::protocol::ClientMessage;

fn td() -> crate::store::Dispatch {
    crate::store::Dispatch::new(tokio::sync::mpsc::unbounded_channel().0)
}

fn seed_adapter(adapter: &InMemorySourceAdapter, seeded: &[(Story, Vec<String>)]) {
    for (story, labels) in seeded {
        adapter.seed_labels(story.id.value(), labels.clone());
    }
    adapter.seed_stories(seeded.iter().map(|(s, _)| s.clone()).collect());
}

fn bind_epics(
    orch: &std::sync::Arc<dyn crate::sync::RefreshDriver>,
    seeded: &[(Story, Vec<String>)],
) {
    let epics: Vec<Epic> = seeded
        .iter()
        .filter(|(story, labels)| is_epic_story(story, labels))
        .map(|(story, _)| decode_epic_from_story(story, false))
        .collect();
    crate::store::bind_seeded_refresh(orch, vec![], vec![], epics);
}

#[tokio::test]
async fn set_parent_epic_migrated_path_emits_parent_epic_set_event() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        StoryId::new(300),
        "child".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let epic = Story::new_minimal(
        StoryId::new(457),
        "epic story".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let seeded = vec![(child, Vec::new()), (epic, vec!["epic".to_string()])];
    seed_adapter(&adapter, &seeded);
    let orchestrator: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    bind_epics(&orchestrator, &seeded);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(300));
    args.insert("epic_id".into(), serde_json::json!(457));
    let result = handle_set_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    assert_eq!(
        orchestrator.story(300).unwrap().parent_epic(),
        Some(EpicId::new(457))
    );

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        StoryEvent::ParentEpicSet {
            id,
            epic_id,
            ..
        } if *id == StoryId::new(300) && *epic_id == EpicId::new(457)
    ));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_existent_epic() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        StoryId::new(310),
        "orphan".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![child]);
    let orchestrator: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(310));
    args.insert("epic_id".into(), serde_json::json!(9999));
    let result = handle_set_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not found"));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_epic_parent() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let child = Story::new_minimal(
        StoryId::new(311),
        "child".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let not_epic = Story::new_minimal(
        StoryId::new(458),
        "ordinary feature".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![child, not_epic]);
    adapter.seed_labels(458, vec!["feature".to_string()]);
    let orchestrator: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(311));
    args.insert("epic_id".into(), serde_json::json!(458));
    let result = handle_set_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("not an Epic"));
}

#[tokio::test]
async fn unset_parent_epic_migrated_path_emits_parent_epic_unset_event() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let seeded = Story::new_minimal(
        StoryId::new(301),
        "child".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![seeded]);
    adapter.seed_labels(301, vec!["parent-epic-#457".to_string()]);
    let orchestrator: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();
    assert_eq!(
        orchestrator.story(301).unwrap().parent_epic(),
        Some(EpicId::new(457)),
        "precondition: the seeded parent-epic decodes to the typed field on ingest"
    );

    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(301));
    let result = handle_unset_parent_epic(&args, "pm:claude-code", Some(&orchestrator)).await;
    assert!(result.is_ok(), "migrated path should succeed: {result:?}");

    assert_eq!(orchestrator.story(301).unwrap().parent_epic(), None);

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 1);
    let pushed = adapter.pushed_events();
    assert_eq!(pushed.len(), 1);
    assert!(matches!(
        &pushed[0],
        StoryEvent::ParentEpicUnset { id, .. } if *id == StoryId::new(301)
    ));
}

#[tokio::test]
async fn set_parent_epic_rejects_non_pm_role() {
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(1));
    args.insert("epic_id".into(), serde_json::json!(2));
    let result = handle_set_parent_epic(&args, "dev:claude-code", None).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("restricted to pm"));
}

#[tokio::test]
async fn set_parent_epic_rejects_self_parent() {
    let mut args = serde_json::Map::new();
    args.insert("id".into(), serde_json::json!(42));
    args.insert("epic_id".into(), serde_json::json!(42));
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::events::StoryEvent;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let epic = Story::new_minimal(
        StoryId::new(500),
        "epic story".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    let seeded = vec![(epic, vec!["epic".to_string()])];
    seed_adapter(&adapter, &seeded);
    let orchestrator: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    bind_epics(&orchestrator, &seeded);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("title".into(), serde_json::json!("child of epic"));
    args.insert("user_job".into(), serde_json::json!("user job here"));
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("would reject if X"),
    );
    args.insert("parent_epic".into(), serde_json::json!(500));
    let result = handle_create_story(&args, "pm:claude-code", Some(&orchestrator), &td()).await;
    assert!(
        result.is_ok(),
        "create-with-parent should succeed: {result:?}"
    );
    assert!(result.unwrap().contains("under epic #500"));

    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 2, "StoryCreated + ParentEpicSet both drain");
    let pushed = adapter.pushed_events();
    assert!(matches!(pushed[0], StoryEvent::StoryCreated { .. }));
    assert!(matches!(
        pushed[1],
        StoryEvent::ParentEpicSet {
            epic_id,
            ..
        } if epic_id == EpicId::new(500)
    ));
}

#[tokio::test]
async fn create_story_with_non_epic_parent_rejects_before_create() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let not_epic = Story::new_minimal(
        StoryId::new(501),
        "not epic".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![not_epic]);
    adapter.seed_labels(501, vec!["feature".to_string()]);
    let orchestrator: Arc<dyn RefreshDriver> =
        Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orchestrator, vec![], vec![], vec![]);
    orchestrator.refresh_all().await.unwrap();

    let mut args = serde_json::Map::new();
    args.insert("title".into(), serde_json::json!("orphan"));
    args.insert("user_job".into(), serde_json::json!("user job here"));
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("would reject if X"),
    );
    args.insert("parent_epic".into(), serde_json::json!(501));
    let result = handle_create_story(&args, "pm:claude-code", Some(&orchestrator), &td()).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("not an Epic"));
    let drained = orchestrator.drain_pending().await.unwrap();
    assert_eq!(drained, 0, "no events emitted on parent-validation reject");
}

#[tokio::test]
async fn view_board_renders_epic_summary_section_with_child_progress() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Stage, Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let epic = Story::new_minimal(
        StoryId::new(700),
        "umbrella story".into(),
        Work::NotStarted,
        ts,
        "pm".into(),
    );
    let mut child_a = Story::new_minimal(
        StoryId::new(701),
        "child a".into(),
        Work::Accepted,
        ts,
        "pm".into(),
    );
    child_a.parent_epic = Some(EpicId::new(700));
    let mut child_b = Story::new_minimal(
        StoryId::new(702),
        "child b".into(),
        Work::Acceptance,
        ts,
        "pm".into(),
    );
    child_b.parent_epic = Some(EpicId::new(700));
    let mut child_c = Story::new_minimal(
        StoryId::new(703),
        "child c".into(),
        Work::InProgress,
        ts,
        "pm".into(),
    );
    child_c.parent_epic = Some(EpicId::new(700));
    let member_labels = vec!["parent-epic-#700".to_string()];
    let seeded = vec![
        (epic, vec!["epic".to_string()]),
        (child_a, member_labels.clone()),
        (child_b, member_labels.clone()),
        (child_c, member_labels),
    ];
    seed_adapter(&adapter, &seeded);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    bind_epics(&orch, &seeded);
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let plain = Story::new_minimal(
        StoryId::new(800),
        "plain feature".into(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    adapter.seed_stories(vec![plain]);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let epic = Story::new_minimal(
        StoryId::new(710),
        "another umbrella".into(),
        Work::NotStarted,
        ts,
        "pm".into(),
    );
    let seeded = vec![(epic, vec!["epic".to_string()])];
    seed_adapter(&adapter, &seeded);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    bind_epics(&orch, &seeded);
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let epic = Story::new_minimal(
        StoryId::new(720),
        "delivered umbrella".into(),
        Work::NotStarted,
        ts,
        "pm".into(),
    );
    let mut a = Story::new_minimal(
        StoryId::new(721),
        "a".into(),
        Work::Acceptance,
        ts,
        "pm".into(),
    );
    a.parent_epic = Some(EpicId::new(720));
    let mut b = Story::new_minimal(
        StoryId::new(722),
        "b".into(),
        Work::Acceptance,
        ts,
        "pm".into(),
    );
    b.parent_epic = Some(EpicId::new(720));
    let member_labels = vec!["parent-epic-#720".to_string()];
    let seeded = vec![
        (epic, vec!["epic".to_string()]),
        (a, member_labels.clone()),
        (b, member_labels),
    ];
    seed_adapter(&adapter, &seeded);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    bind_epics(&orch, &seeded);
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
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::story::{Stage, Story, Work};
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let ts = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let epic = Story::new_minimal(
        StoryId::new(730),
        "owner-accepted umbrella".into(),
        Work::Accepted,
        ts,
        "pm".into(),
    );
    let mut a = Story::new_minimal(
        StoryId::new(731),
        "a".into(),
        Work::InProgress,
        ts,
        "pm".into(),
    );
    a.parent_epic = Some(EpicId::new(730));
    let seeded = vec![
        (epic, vec!["epic".to_string()]),
        (a, vec!["parent-epic-#730".to_string()]),
    ];
    seed_adapter(&adapter, &seeded);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    bind_epics(&orch, &seeded);
    orch.refresh_all().await.unwrap();
    let out = handle_view_board(&serde_json::Map::new(), Some(&orch))
        .await
        .unwrap();
    assert!(
        out.contains("#730 [in-progress] owner-accepted umbrella"),
        "Post-#1450: explicit_state ignored — substrate IS source-of-truth (META-CANON #1); active child wins; got: {out}"
    );
}

#[tokio::test]
async fn create_story_for_epic_title_dispatches_epic_upsert_and_stays_out_of_story_lanes() {
    use crate::integrations::in_memory::InMemorySourceAdapter;
    use crate::product_management::epic::EpicAction;
    use crate::sync::RefreshDriver;
    use crate::sync::SyncOrchestrator;
    use std::sync::Arc;

    let adapter = Arc::new(InMemorySourceAdapter::new());
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(Arc::clone(&adapter)));
    crate::store::bind_seeded_refresh(&orch, vec![], vec![], vec![]);
    orch.refresh_all().await.unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<crate::store::AppAction>();
    let dispatch = crate::store::Dispatch::new(tx);
    let mut args = serde_json::Map::new();
    args.insert("title".into(), serde_json::json!("[Epic] umbrella"));
    args.insert("user_job".into(), serde_json::json!("user job here"));
    args.insert(
        "observation_falsifier".into(),
        serde_json::json!("would reject if X"),
    );
    let result = handle_create_story(&args, "pm:claude-code", Some(&orch), &dispatch).await;
    assert!(result.is_ok(), "epic create should succeed: {result:?}");

    let epic_id = match rx.try_recv() {
        Ok(crate::store::AppAction::Epic(EpicAction::Upserted(epics))) => {
            assert_eq!(epics.len(), 1, "one epic upserted");
            assert_eq!(epics[0].title, "[Epic] umbrella");
            epics[0].id.value()
        }
        Ok(_) => panic!("expected an EpicAction::Upserted dispatch, got a different action"),
        Err(_) => panic!("expected an EpicAction::Upserted dispatch, got none"),
    };
    assert!(
        orch.all_stories().iter().all(|s| s.id.value() != epic_id),
        "a created epic lives only in the epic slice — it must not linger in the story store / state-lanes"
    );
}
