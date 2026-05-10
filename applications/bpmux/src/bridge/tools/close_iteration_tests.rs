#![allow(clippy::unwrap_used)]
#![allow(clippy::panic)]

use crate::session::iteration_markers::IterationMarker;
use crate::session::priority::{
    CloseIterationOutcome, IceboxItem, PriorityIndex, close_iteration_atomic,
};

fn marker(id: u64, position: u32) -> IterationMarker {
    IterationMarker {
        id,
        theme_name: format!("theme{id}"),
        position,
        created_at: "2026-05-10T00:00:00Z".to_string(),
    }
}

#[test]
fn no_markers_in_icebox_is_noop_outcome() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    let mut markers: Vec<IterationMarker> = vec![];
    let snapshot_unified = index.icebox_unified.clone();
    let outcome = close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(outcome, CloseIterationOutcome::default());
    assert_eq!(outcome.deleted_marker_id, None);
    assert!(outcome.promoted.is_empty());
    assert_eq!(index.icebox_unified, snapshot_unified);
}

#[test]
fn promotes_stories_above_first_marker_to_ready() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 300 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];
    let outcome = close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(outcome.promoted, vec![100, 200]);
    assert_eq!(
        index.columns.get("ready").cloned().unwrap_or_default(),
        vec![100, 200]
    );
}

#[test]
fn deletes_only_first_marker_keeps_remaining_markers() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 8 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];
    let outcome = close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].id, 8);
    assert!(
        index
            .icebox_unified
            .contains(&IceboxItem::Story { number: 200 })
    );
    assert!(index.icebox_unified.contains(&IceboxItem::Marker { id: 8 }));
    assert!(!index.icebox_unified.contains(&IceboxItem::Marker { id: 7 }));
}

#[test]
fn marker_at_top_of_icebox_promotes_zero_stories_but_deletes_marker() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers);
    assert!(outcome.promoted.is_empty());
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(markers.len(), 0);
    assert!(
        index
            .icebox_unified
            .contains(&IceboxItem::Story { number: 200 })
    );
}

#[test]
fn empty_icebox_with_only_markers_promotes_zero_deletes_first_marker() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    index.icebox_unified.push(IceboxItem::Marker { id: 8 });
    let mut markers = vec![marker(7, 0), marker(8, 1)];
    let outcome = close_iteration_atomic(&mut index, &mut markers);
    assert!(outcome.promoted.is_empty());
    assert_eq!(outcome.deleted_marker_id, Some(7));
    assert_eq!(markers.len(), 1);
    assert_eq!(index.icebox_unified, vec![IceboxItem::Marker { id: 8 }]);
}

use crate::board::RefreshDriver;
use crate::board::adapters::in_memory::InMemorySourceAdapter;
use crate::board::domain::{Story, StoryState};
use crate::board::orchestrator::SyncOrchestrator;
use crate::board::store::BoardStore;
use crate::transport::rest::collect_iteration_close_blockers;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn seed_story(number: u64, state: StoryState) -> Story {
    Story::new_minimal(
        number,
        format!("title-{number}"),
        state,
        Utc.with_ymd_and_hms(2026, 5, 10, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

async fn build_refresh_with_stories(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch = SyncOrchestrator::new(store, adapter);
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
    orch.refresh_all().await.unwrap();
    orch
}

#[tokio::test]
async fn guard_blocks_when_ready_queue_has_delivered_story() {
    let refresh = build_refresh_with_stories(vec![seed_story(1234, StoryState::Delivered)]).await;
    let blockers = collect_iteration_close_blockers(Some(&refresh)).await;
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].number, 1234);
    assert_eq!(blockers[0].state, "delivered");
}

#[tokio::test]
async fn guard_blocks_for_each_non_accepted_in_flight_state() {
    let refresh = build_refresh_with_stories(vec![
        seed_story(100, StoryState::Ready),
        seed_story(200, StoryState::InProgress),
        seed_story(300, StoryState::DeskCheck),
        seed_story(400, StoryState::Delivered),
    ])
    .await;
    let blockers = collect_iteration_close_blockers(Some(&refresh)).await;
    assert_eq!(blockers.len(), 4);
    let numbers: Vec<u64> = blockers.iter().map(|b| b.number).collect();
    assert!(numbers.contains(&100));
    assert!(numbers.contains(&200));
    assert!(numbers.contains(&300));
    assert!(numbers.contains(&400));
}

#[tokio::test]
async fn guard_passes_when_only_accepted_and_icebox_stories_exist() {
    let refresh = build_refresh_with_stories(vec![
        seed_story(500, StoryState::Accepted),
        seed_story(600, StoryState::Icebox),
    ])
    .await;
    let blockers = collect_iteration_close_blockers(Some(&refresh)).await;
    assert!(blockers.is_empty());
}

#[tokio::test]
async fn guard_passes_when_board_is_empty() {
    let refresh = build_refresh_with_stories(vec![]).await;
    let blockers = collect_iteration_close_blockers(Some(&refresh)).await;
    assert!(blockers.is_empty());
}

#[tokio::test]
async fn guard_no_refresh_handle_treated_as_no_blockers() {
    let blockers = collect_iteration_close_blockers(None).await;
    assert!(blockers.is_empty());
}

#[test]
fn close_iteration_does_not_touch_other_columns() {
    let mut index = PriorityIndex::default();
    index
        .columns
        .insert("in-progress".to_string(), vec![500, 501]);
    index.columns.insert("desk-check".to_string(), vec![600]);
    index
        .columns
        .insert("delivered".to_string(), vec![700, 701]);
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    let outcome = close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(outcome.promoted, vec![100]);
    assert_eq!(
        index.columns.get("in-progress").cloned(),
        Some(vec![500, 501])
    );
    assert_eq!(index.columns.get("desk-check").cloned(), Some(vec![600]));
    assert_eq!(
        index.columns.get("delivered").cloned(),
        Some(vec![700, 701])
    );
}
