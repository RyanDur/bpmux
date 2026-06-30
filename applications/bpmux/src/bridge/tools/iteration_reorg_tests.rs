#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::bridge::tools_stories::{
    handle_move_iteration_member, handle_remove_iteration_member, handle_rename_iteration,
    handle_reorder_iteration,
};
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::adapters::relationships::{
    IterationRelationships, Relationships,
};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::iteration::IterationId;
use crate::product_management::story::{Story, StoryId, StoryState};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn iteration(id: u64, name: &str) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("Iteration {name}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels.push("iteration".to_string());
    s.story_type = crate::product_management::story::StoryType::Iteration;
    s
}

fn iteration_at(id: u64, name: &str, order: u32) -> Story {
    let mut s = iteration(id, name);
    s.raw_labels
        .push(GhLabelCodec.encode_iteration_order(order));
    s
}

fn iteration_with_members(id: u64, name: &str, members: Vec<u64>) -> Story {
    let mut s = iteration(id, name);
    s.body = Some(
        IterationRelationships {
            outcome: None,
            stories: members.into_iter().map(StoryId::new).collect(),
        }
        .write_into_body("## composed"),
    );
    s
}

fn member_story(id: u64, state: StoryState) -> Story {
    let mut s = Story::new_minimal(
        StoryId::new(id),
        format!("[Feature] member-{id}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.raw_labels.push("feature".to_string());
    s
}

async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(store, adapter));
    orch.refresh_all().await.unwrap();
    orch
}

fn remove_args(iteration: u64, story: u64, reason: Option<&str>) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("iteration_id".into(), json!(iteration));
    m.insert("story_id".into(), json!(story));
    if let Some(re) = reason {
        m.insert("reason".into(), json!(re));
    }
    m
}

#[tokio::test]
async fn remove_member_clears_membership_so_the_board_no_longer_shows_it_1762() {
    let refresh = build_refresh(vec![
        iteration_with_members(1700, "Circinus", vec![710]),
        member_story(710, StoryState::Ready),
    ])
    .await;
    let result = handle_remove_iteration_member(
        &remove_args(1700, 710, Some("descoped this iteration")),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm removes the member");
    assert!(
        refresh
            .iteration(IterationId::new(1700))
            .expect("present")
            .stories
            .is_empty(),
        "the membership is cleared (the only intentional vanish-path) — board no longer renders it under the iteration"
    );
    assert!(
        result.contains("removed from iteration #1700") && result.contains("descoped"),
        "the confirm states what + the reason (audit): {result}"
    );
}

#[tokio::test]
async fn remove_member_flags_an_in_flight_story_state_not_a_silent_scope_cut_1762() {
    let refresh = build_refresh(vec![
        iteration_with_members(1700, "Circinus", vec![710]),
        member_story(710, StoryState::InProgress),
    ])
    .await;
    let result = handle_remove_iteration_member(
        &remove_args(1700, 710, Some("pivot")),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm removes an in-flight member");
    assert!(
        result.contains("(was: in-progress)"),
        "removing an in-flight story FLAGS its state — not a silent scope-cut: {result}"
    );
}

#[tokio::test]
async fn remove_member_with_no_reason_is_rejected_1762() {
    let refresh = build_refresh(vec![
        iteration_with_members(1700, "Circinus", vec![710]),
        member_story(710, StoryState::Ready),
    ])
    .await;
    let err = handle_remove_iteration_member(
        &remove_args(1700, 710, None),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect_err("remove is destructive — a reason is required");
    assert!(
        err.contains("reason"),
        "the error names the required reason: {err}"
    );
    assert!(
        refresh
            .iteration(IterationId::new(1700))
            .expect("present")
            .stories
            .contains(&StoryId::new(710)),
        "a reasonless remove mutates nothing"
    );
}

#[tokio::test]
async fn remove_member_is_pm_lane_non_pm_refused_1762() {
    let refresh = build_refresh(vec![
        iteration_with_members(1700, "Circinus", vec![710]),
        member_story(710, StoryState::Ready),
    ])
    .await;
    let err = handle_remove_iteration_member(
        &remove_args(1700, 710, Some("r")),
        "dev:claude-code",
        Some(&refresh),
    )
    .await
    .expect_err("dev cannot remove a member (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn remove_member_rejects_a_non_member_1762() {
    let refresh = build_refresh(vec![
        iteration(1700, "Circinus"),
        member_story(710, StoryState::Ready),
    ])
    .await;
    let err = handle_remove_iteration_member(
        &remove_args(1700, 710, Some("r")),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect_err("a story not in the iteration cannot be removed from it");
    assert!(
        err.contains("not a member"),
        "the error explains it's not a member: {err}"
    );
}

#[tokio::test]
async fn move_member_reorders_within_the_iteration_1762() {
    let refresh = build_refresh(vec![
        iteration_with_members(1700, "Circinus", vec![710, 720]),
        member_story(710, StoryState::Ready),
        member_story(720, StoryState::Ready),
    ])
    .await;
    let args = json!({ "iteration_id": 1700, "story_id": 720, "position": 1 })
        .as_object()
        .cloned()
        .unwrap();
    handle_move_iteration_member(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm reorders the member");
    assert_eq!(
        refresh
            .iteration(IterationId::new(1700))
            .expect("present")
            .stories,
        vec![StoryId::new(720), StoryId::new(710)],
        "the member moves to the requested 1-indexed position within the iteration"
    );
}

#[tokio::test]
async fn reorder_iteration_insert_shifts_the_planning_order_1762() {
    let refresh = build_refresh(vec![
        iteration_at(1700, "Circinus", 100),
        iteration_at(1710, "Dorado", 200),
        iteration_at(1720, "Eridanus", 300),
    ])
    .await;
    let args = json!({ "iteration_id": 1720, "position": 1 })
        .as_object()
        .cloned()
        .unwrap();
    handle_reorder_iteration(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm reorders the iteration to the front");
    let order_of = |n: u64| {
        refresh
            .iteration(IterationId::new(n))
            .expect("present")
            .iteration_order
            .expect("ordered")
    };
    assert_eq!(
        (order_of(1720), order_of(1700), order_of(1710)),
        (1, 2, 3),
        "dragging Eridanus to the front renumbers the planning sequence 1..N by insert-shift"
    );
}

#[tokio::test]
async fn reorder_iteration_is_pm_lane_non_pm_refused_1762() {
    let refresh = build_refresh(vec![iteration_at(1700, "Circinus", 100)]).await;
    let err = handle_reorder_iteration(
        &json!({ "iteration_id": 1700, "position": 1 })
            .as_object()
            .cloned()
            .unwrap(),
        "dev:claude-code",
        Some(&refresh),
    )
    .await
    .expect_err("dev cannot reorder iterations (pm-lane)");
    assert!(err.contains("pm"), "the refusal names the pm-lane: {err}");
}

#[tokio::test]
async fn rename_updates_the_iteration_title_1762() {
    let refresh = build_refresh(vec![iteration(1700, "Circinus")]).await;
    let args = json!({ "iteration_id": 1700, "name": "Vela" })
        .as_object()
        .cloned()
        .unwrap();
    handle_rename_iteration(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm renames the iteration");
    assert_eq!(
        refresh
            .iteration(IterationId::new(1700))
            .expect("present")
            .content
            .title,
        "Iteration Vela",
        "the rename updates the iteration title (surfaced in board/roadmap)"
    );
}

#[tokio::test]
async fn renaming_a_current_iteration_preserves_its_current_designation() {
    use crate::bridge::tools_stories::{handle_designate_current_iteration, handle_edit_story};

    let refresh = build_refresh(vec![iteration(1700, "Vela")]).await;

    let designate = json!({ "iteration_id": 1700 })
        .as_object()
        .cloned()
        .unwrap();
    handle_designate_current_iteration(&designate, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm designates the iteration current");
    assert!(
        refresh
            .iteration(IterationId::new(1700))
            .expect("present")
            .is_current,
        "sanity: the iteration is current right after designation"
    );

    let rename = json!({ "id": 1700, "title": "Iteration Vela-RENAMED" })
        .as_object()
        .cloned()
        .unwrap();
    handle_edit_story(&rename, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm renames the iteration");

    assert!(
        refresh
            .iteration(IterationId::new(1700))
            .expect("present")
            .is_current,
        "renaming a current iteration must NOT clear its current designation"
    );
}

#[tokio::test]
async fn iteration_reorg_verbs_teach_the_boundary_on_a_non_iteration_1762() {
    let mut feature = Story::new_minimal(
        StoryId::new(700),
        "[Feature] plain".into(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    feature.raw_labels.push("feature".to_string());
    let refresh = build_refresh(vec![feature]).await;
    let args = json!({ "iteration_id": 700, "name": "X" })
        .as_object()
        .cloned()
        .unwrap();
    let err = handle_rename_iteration(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect_err("a non-iteration is not name-editable");
    assert!(
        err.contains("not an Iteration"),
        "the error teaches the boundary: {err}"
    );
}
