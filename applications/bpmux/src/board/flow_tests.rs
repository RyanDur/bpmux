#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};

use super::flow::{Position, ReworkBreakdown, desk_check_rework, sub_section_timings};
use super::history::{PlacementHistory, react_to_story};
use crate::product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{Stage, StoryId, Work};

fn transition(story: u64, hour: u32, to: Work, placement: Option<Stage>) -> StoryEvent {
    StoryEvent::new(
        EventMeta {
            actor: "dev".to_string(),
            ts: Utc.with_ymd_and_hms(2026, 7, 7, hour, 0, 0).unwrap(),
        },
        StoryId::new(story),
        StoryEventKind::StateTransitioned {
            new_state: to,
            placement: placement.map(|s| s.id()),
        },
    )
}

fn history(events: Vec<StoryEvent>) -> PlacementHistory {
    react_to_story(&PlacementHistory::default(), &StoryAction::Events(events)).unwrap_or_default()
}

#[test]
fn sub_section_timings_close_one_interval_at_the_first_records_position() {
    let history = history(vec![
        transition(1, 1, Work::InProgress, Some(Stage::DeskCheck)),
        transition(1, 4, Work::Acceptance, None),
    ]);
    assert_eq!(
        sub_section_timings(&history),
        vec![(Position::Placement(Stage::DeskCheck.id()), 3.0)],
        "the closed span is attributed to the position the story sat in, for three hours"
    );
}

#[test]
fn sub_section_timings_exclude_the_open_final_interval() {
    let history = history(vec![transition(1, 1, Work::InProgress, Some(Stage::Dev))]);
    assert!(
        sub_section_timings(&history).is_empty(),
        "a story's last position has no closing record, so it contributes no span and needs no now-clock"
    );
}

#[test]
fn sub_section_timings_attribute_a_placementless_span_to_the_coarse_state() {
    let history = history(vec![
        transition(1, 1, Work::InProgress, None),
        transition(1, 3, Work::Acceptance, None),
    ]);
    assert_eq!(
        sub_section_timings(&history),
        vec![(Position::Coarse(Work::InProgress), 2.0)],
        "with no fine placement the span keys on the coarse work state"
    );
}

#[test]
fn sub_section_timings_report_the_median_span_per_position() {
    let history = history(vec![
        transition(1, 1, Work::InProgress, Some(Stage::Dev)),
        transition(1, 3, Work::Acceptance, None),
        transition(2, 1, Work::InProgress, Some(Stage::Dev)),
        transition(2, 5, Work::Acceptance, None),
    ]);
    assert_eq!(
        sub_section_timings(&history),
        vec![(Position::Placement(Stage::Dev.id()), 3.0)],
        "two Dev spans of two and four hours have a median of three"
    );
}

#[test]
fn desk_check_rework_counts_a_reentry_into_a_previously_left_stage() {
    let history = history(vec![
        transition(1, 1, Work::InProgress, Some(Stage::Dev)),
        transition(1, 2, Work::InProgress, Some(Stage::DeskCheck)),
        transition(1, 3, Work::InProgress, Some(Stage::Dev)),
    ]);
    assert_eq!(
        desk_check_rework(&history),
        ReworkBreakdown {
            total: 1,
            per_story: vec![(StoryId::new(1), 1)],
        },
        "returning to Dev after leaving it for desk-check is one bounce-back"
    );
}

#[test]
fn desk_check_rework_is_zero_for_a_forward_only_walk() {
    let history = history(vec![
        transition(1, 1, Work::InProgress, Some(Stage::Dev)),
        transition(1, 2, Work::InProgress, Some(Stage::DeskCheck)),
        transition(1, 3, Work::Acceptance, None),
    ]);
    assert_eq!(
        desk_check_rework(&history),
        ReworkBreakdown {
            total: 0,
            per_story: Vec::new(),
        },
        "a walk that never revisits a left position has no rework"
    );
}
