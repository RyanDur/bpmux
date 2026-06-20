#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]

use crate::board::RefreshDriver;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState, StoryType};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::store::BoardStore;
use crate::bridge::tools_stories::{
    handle_edit_iteration_theme, handle_move_iteration_member, handle_remove_iteration_member,
};
use chrono::{TimeZone, Utc};
use serde_json::{Map, Value, json};
use std::sync::Arc;

fn iteration(number: u64, theme: &str) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration {theme}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = StoryType::Iteration;
    s
}

fn member(number: u64, iteration_number: u64, position: u32, state: StoryState) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("[Feature] member-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    s.story_type = StoryType::Feature;
    s.iteration_membership = Some((iteration_number, position));
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
    m.insert("iteration_number".into(), json!(iteration));
    m.insert("story_number".into(), json!(story));
    if let Some(re) = reason {
        m.insert("reason".into(), json!(re));
    }
    m
}

#[tokio::test]
async fn remove_member_clears_membership_so_the_board_no_longer_shows_it_1762() {
    let refresh = build_refresh(vec![
        iteration(1700, "Circinus"),
        member(710, 1700, 1, StoryState::Ready),
    ])
    .await;
    let result = handle_remove_iteration_member(
        &remove_args(1700, 710, Some("descoped this iteration")),
        "pm:claude-code",
        Some(&refresh),
    )
    .await
    .expect("pm removes the member");
    assert_eq!(
        refresh.story(710).expect("present").iteration_membership,
        None,
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
        iteration(1700, "Circinus"),
        member(710, 1700, 1, StoryState::InProgress),
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
        iteration(1700, "Circinus"),
        member(710, 1700, 1, StoryState::Ready),
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
            .story(710)
            .expect("present")
            .iteration_membership
            .is_some(),
        "a reasonless remove mutates nothing"
    );
}

#[tokio::test]
async fn remove_member_is_pm_lane_non_pm_refused_1762() {
    let refresh = build_refresh(vec![
        iteration(1700, "Circinus"),
        member(710, 1700, 1, StoryState::Ready),
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
        member(710, 9999, 1, StoryState::Ready),
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
        iteration(1700, "Circinus"),
        member(710, 1700, 1, StoryState::Ready),
    ])
    .await;
    let args = json!({ "iteration_number": 1700, "story_number": 710, "position": 3 })
        .as_object()
        .cloned()
        .unwrap();
    handle_move_iteration_member(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm reorders the member");
    assert_eq!(
        refresh.story(710).expect("present").iteration_membership,
        Some((1700, 3)),
        "the member reads its new position within the iteration"
    );
}

#[tokio::test]
async fn edit_theme_updates_the_iteration_title_1762() {
    let refresh = build_refresh(vec![iteration(1700, "Circinus")]).await;
    let args = json!({ "iteration_number": 1700, "theme": "Vela" })
        .as_object()
        .cloned()
        .unwrap();
    handle_edit_iteration_theme(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect("pm edits the theme");
    assert_eq!(
        refresh.story(1700).expect("present").title,
        "Iteration Vela",
        "the theme edit updates the iteration title (surfaced in board/roadmap)"
    );
}

#[tokio::test]
async fn iteration_reorg_verbs_teach_the_boundary_on_a_non_iteration_1762() {
    let mut feature = Story::new_minimal(
        700,
        "[Feature] plain".into(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap(),
        "pm".into(),
    );
    feature.story_type = StoryType::Feature;
    let refresh = build_refresh(vec![feature]).await;
    let args = json!({ "iteration_number": 700, "theme": "X" })
        .as_object()
        .cloned()
        .unwrap();
    let err = handle_edit_iteration_theme(&args, "pm:claude-code", Some(&refresh))
        .await
        .expect_err("a non-iteration is not theme-editable");
    assert!(
        err.contains("not an Iteration"),
        "the error teaches the boundary: {err}"
    );
}
