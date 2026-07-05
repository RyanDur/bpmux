#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::view_backlog::*;
use crate::product_management::story::BoardState;
use crate::product_management::story::Story;
use crate::product_management::story::StoryId;
use chrono::{TimeZone, Utc};

fn story(num: u64, title: &str, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(num),
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

#[test]
fn empty_stories_returns_all_zero_counts() {
    let summary = build_backlog_summary(&[]);
    assert_eq!(summary["ready_count"], 0);
    assert_eq!(summary["in_progress_count"], 0);
    assert_eq!(summary["acceptance_count"], 0);
    assert_eq!(summary["accepted_count"], 0);
    assert_eq!(summary["icebox_count"], 0);
}

#[test]
fn accepted_state_counted_as_accepted() {
    let stories = vec![story(100, "done", BoardState::Accepted)];
    assert_eq!(build_backlog_summary(&stories)["accepted_count"], 1);
}

#[test]
fn ready_state_populates_samples() {
    let stories = vec![story(101, "ready story", BoardState::Ready)];
    let summary = build_backlog_summary(&stories);
    assert_eq!(summary["ready_count"], 1);
    assert_eq!(summary["ready_samples"][0], "#101 ready story");
}

#[test]
fn in_progress_state_counted_correctly() {
    let stories = vec![
        story(102, "a", BoardState::InProgress),
        story(103, "b", BoardState::InProgress),
    ];
    assert_eq!(build_backlog_summary(&stories)["in_progress_count"], 2);
}

#[test]
fn in_work_middle_stages_count_as_in_progress_acceptance_is_the_rail_only() {
    let stories = vec![
        story(104, "d", BoardState::Acceptance),
        story(105, "dc", BoardState::InProgress),
    ];
    let summary = build_backlog_summary(&stories);
    assert_eq!(summary["acceptance_count"], 1);
    assert_eq!(summary["in_progress_count"], 1);
}

#[test]
fn icebox_state_counted_as_icebox() {
    let stories = vec![story(107, "icy", BoardState::Icebox)];
    assert_eq!(build_backlog_summary(&stories)["icebox_count"], 1);
}

#[test]
fn closed_non_accepted_story_lands_in_retired_count() {
    let mut s = story(109, "retired", BoardState::Ready);
    s.is_closed = true;
    let summary = build_backlog_summary(&[s]);
    assert_eq!(summary["retired_count"], 1);
    assert_eq!(summary["ready_count"], 0);
    assert_eq!(summary["accepted_count"], 0);
}

#[test]
fn samples_cap_at_sample_limit() {
    let stories: Vec<Story> = (0..8)
        .map(|i| story(200 + i, &format!("ready {i}"), BoardState::Ready))
        .collect();
    let summary = build_backlog_summary(&stories);
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
