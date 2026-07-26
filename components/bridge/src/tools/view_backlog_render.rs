#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::view_backlog::*;
use chrono::{TimeZone, Utc};
use product_management::iteration::{Iteration, IterationId};
use product_management::story::Story;
use product_management::story::StoryId;
use product_management::story::Work;
use product_management::story::slice::StoryAction;
use store::AppAction;
use store_test_support::SeededStore;

fn story(num: u64, title: &str, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(num),
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn current_iteration_of(members: &[u64]) -> Vec<Iteration> {
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
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

async fn backlog_summary_via_dispatch(
    stories: Vec<Story>,
    iterations: Vec<Iteration>,
) -> serde_json::Value {
    let store = SeededStore::new(vec![], iterations, vec![]);
    store.dispatch(AppAction::Story(StoryAction::Seeded(stories)));
    let raw = crate::dispatch::dispatch_view_backlog(&store.select())
        .await
        .unwrap();
    serde_json::from_str(&raw).unwrap()
}

#[tokio::test]
async fn a_ready_state_non_member_reports_in_icebox_not_ready() {
    let summary = backlog_summary_via_dispatch(
        vec![story(300, "unscheduled work", Work::Ready)],
        current_iteration_of(&[]),
    )
    .await;
    assert_eq!(
        summary["icebox_count"], 1,
        "a ready-state non-member derives Icebox in the backlog summary, agreeing with board/list_ready/close-blockers; got {summary}"
    );
    assert_eq!(
        summary["ready_count"], 0,
        "a ready-state non-member never counts ready; got {summary}"
    );
    assert!(
        summary["ready_samples"].as_array().unwrap().is_empty(),
        "a ready-state non-member never appears in ready_samples; got {summary}"
    );
}

#[tokio::test]
async fn a_ready_state_current_iteration_member_reports_in_ready() {
    let summary = backlog_summary_via_dispatch(
        vec![story(301, "scheduled member", Work::Ready)],
        current_iteration_of(&[301]),
    )
    .await;
    assert_eq!(
        summary["ready_count"], 1,
        "a current-iteration member projects Ready in the backlog summary regardless of its stored scheduling state; got {summary}"
    );
    assert_eq!(
        summary["icebox_count"], 0,
        "the member does not double-count in icebox; got {summary}"
    );
    assert_eq!(
        summary["ready_samples"][0], "#301 scheduled member",
        "the projected-Ready member is the ready sample; got {summary}"
    );
}

#[test]
fn empty_stories_returns_all_zero_counts() {
    let summary = build_backlog_summary(&[], &[]);
    assert_eq!(summary["ready_count"], 0);
    assert_eq!(summary["in_progress_count"], 0);
    assert_eq!(summary["acceptance_count"], 0);
    assert_eq!(summary["accepted_count"], 0);
    assert_eq!(summary["icebox_count"], 0);
}

#[test]
fn accepted_state_counted_as_accepted() {
    let stories = vec![story(100, "done", Work::Accepted)];
    assert_eq!(build_backlog_summary(&stories, &[])["accepted_count"], 1);
}

#[test]
fn ready_state_populates_samples() {
    let stories = vec![story(101, "ready story", Work::Ready)];
    let summary = build_backlog_summary(&stories, &current_iteration_of(&[101]));
    assert_eq!(summary["ready_count"], 1);
    assert_eq!(summary["ready_samples"][0], "#101 ready story");
}

#[test]
fn in_progress_state_counted_correctly() {
    let stories = vec![
        story(102, "a", Work::InProgress),
        story(103, "b", Work::InProgress),
    ];
    assert_eq!(build_backlog_summary(&stories, &[])["in_progress_count"], 2);
}

#[test]
fn in_work_middle_stages_count_as_in_progress_acceptance_is_the_rail_only() {
    let stories = vec![
        story(104, "d", Work::Acceptance),
        story(105, "dc", Work::InProgress),
    ];
    let summary = build_backlog_summary(&stories, &[]);
    assert_eq!(summary["acceptance_count"], 1);
    assert_eq!(summary["in_progress_count"], 1);
}

#[test]
fn icebox_state_counted_as_icebox() {
    let stories = vec![story(107, "icy", Work::Ready)];
    assert_eq!(build_backlog_summary(&stories, &[])["icebox_count"], 1);
}

#[test]
fn retired_story_lands_in_retired_count() {
    let s = story(109, "retired", Work::Retired);
    let summary = build_backlog_summary(&[s], &[]);
    assert_eq!(summary["retired_count"], 1);
    assert_eq!(summary["ready_count"], 0);
    assert_eq!(summary["accepted_count"], 0);
}

#[test]
fn samples_cap_at_sample_limit() {
    let stories: Vec<Story> = (0..8)
        .map(|i| story(200 + i, &format!("ready {i}"), Work::Ready))
        .collect();
    let members: Vec<u64> = (0..8).map(|i| 200 + i).collect();
    let summary = build_backlog_summary(&stories, &current_iteration_of(&members));
    assert_eq!(
        summary["ready_count"], 8,
        "every Ready story is counted; only the displayed samples are capped"
    );
    assert_eq!(
        summary["ready_samples"].as_array().unwrap().len(),
        5,
        "the sample list is capped at five so the summary stays scannable even as the Ready column grows"
    );
}
