#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
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
        ..IterationMarker::default()
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
async fn guard_blocks_when_current_iteration_has_delivered_story() {
    let refresh = build_refresh_with_stories(vec![seed_story(1234, StoryState::Delivered)]).await;
    let blockers = collect_iteration_close_blockers(Some(&refresh), &[1234]).await;
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].number, 1234);
    assert_eq!(blockers[0].state, "delivered");
}

#[tokio::test]
async fn guard_blocks_for_each_non_accepted_in_flight_state_within_iteration() {
    let refresh = build_refresh_with_stories(vec![
        seed_story(100, StoryState::Ready),
        seed_story(200, StoryState::InProgress),
        seed_story(300, StoryState::DeskCheck),
        seed_story(400, StoryState::Delivered),
    ])
    .await;
    let blockers = collect_iteration_close_blockers(Some(&refresh), &[100, 200, 300, 400]).await;
    assert_eq!(blockers.len(), 4);
    let numbers: Vec<u64> = blockers.iter().map(|b| b.number).collect();
    assert!(numbers.contains(&100));
    assert!(numbers.contains(&200));
    assert!(numbers.contains(&300));
    assert!(numbers.contains(&400));
}

#[tokio::test]
async fn guard_ignores_stories_outside_current_iteration_membership() {
    let refresh = build_refresh_with_stories(vec![
        seed_story(100, StoryState::Delivered),
        seed_story(200, StoryState::InProgress),
        seed_story(300, StoryState::DeskCheck),
        seed_story(400, StoryState::Delivered),
        seed_story(500, StoryState::Ready),
    ])
    .await;
    let blockers = collect_iteration_close_blockers(Some(&refresh), &[]).await;
    assert!(
        blockers.is_empty(),
        "empty current_iteration_stories means no blockers regardless of project-wide in-flight stories"
    );
}

#[tokio::test]
async fn guard_blocks_only_named_iteration_members_not_project_wide() {
    let refresh = build_refresh_with_stories(vec![
        seed_story(100, StoryState::Delivered),
        seed_story(200, StoryState::InProgress),
        seed_story(300, StoryState::Delivered),
    ])
    .await;
    let blockers = collect_iteration_close_blockers(Some(&refresh), &[200]).await;
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].number, 200);
}

#[tokio::test]
async fn guard_passes_when_only_accepted_and_icebox_stories_exist() {
    let refresh = build_refresh_with_stories(vec![
        seed_story(500, StoryState::Accepted),
        seed_story(600, StoryState::Icebox),
    ])
    .await;
    let blockers = collect_iteration_close_blockers(Some(&refresh), &[500, 600]).await;
    assert!(blockers.is_empty());
}

#[tokio::test]
async fn guard_passes_when_board_is_empty() {
    let refresh = build_refresh_with_stories(vec![]).await;
    let blockers = collect_iteration_close_blockers(Some(&refresh), &[]).await;
    assert!(blockers.is_empty());
}

#[tokio::test]
async fn promote_stories_to_ready_fires_state_transitioned_via_canonical_chain() {
    use crate::bridge::tools_stories::promote_stories_to_ready_via_canonical_chain;
    let refresh = build_refresh_with_stories(vec![
        seed_story(100, StoryState::Icebox),
        seed_story(200, StoryState::Icebox),
    ])
    .await;
    promote_stories_to_ready_via_canonical_chain(&refresh, &[100, 200], "pm".to_string()).unwrap();
    refresh.drain_pending().await.unwrap();
    let stories = refresh.all_stories();
    let s100 = stories.iter().find(|s| s.number == 100).unwrap();
    let s200 = stories.iter().find(|s| s.number == 200).unwrap();
    assert_eq!(s100.state, StoryState::Ready);
    assert_eq!(s200.state, StoryState::Ready);
}

#[tokio::test]
async fn promote_stories_to_ready_no_op_on_empty_list() {
    use crate::bridge::tools_stories::promote_stories_to_ready_via_canonical_chain;
    let refresh = build_refresh_with_stories(vec![]).await;
    let result = promote_stories_to_ready_via_canonical_chain(&refresh, &[], "pm".to_string());
    assert!(result.is_ok());
}

#[tokio::test]
async fn guard_no_refresh_handle_treated_as_no_blockers() {
    let blockers = collect_iteration_close_blockers(None, &[100, 200]).await;
    assert!(blockers.is_empty());
}

#[test]
fn close_iteration_writes_promoted_ids_to_current_iteration_stories() {
    let mut index = PriorityIndex::default();
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Story { number: 200 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(index.current_iteration_stories, vec![100, 200]);
}

#[test]
fn close_iteration_replaces_previous_iteration_stories() {
    let mut index = PriorityIndex {
        current_iteration_stories: vec![1, 2, 3],
        ..PriorityIndex::default()
    };
    index.icebox_unified.push(IceboxItem::Story { number: 100 });
    index.icebox_unified.push(IceboxItem::Marker { id: 7 });
    let mut markers = vec![marker(7, 0)];
    close_iteration_atomic(&mut index, &mut markers);
    assert_eq!(
        index.current_iteration_stories,
        vec![100],
        "previous iteration's members replaced by current promote-batch"
    );
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

// #1300 slice 4: canonical-via-GH close_iteration ceremony tests.
// The new path operates on Iteration-issue (story_type=Iteration) +
// promotes members queried via iteration_id; legacy markers-store
// path (tested above) preserved unchanged per parallel-substrate
// strangler-fig agreement until slice 6 #1302 retires.
mod canonical_via_gh_1300 {
    use crate::board::RefreshDriver;
    use crate::board::adapters::in_memory::InMemorySourceAdapter;
    use crate::board::domain::{Story, StoryState, StoryType};
    use crate::board::orchestrator::SyncOrchestrator;
    use crate::board::store::BoardStore;
    use crate::transport::rest::{
        collect_ready_queue_blockers, promote_members_then_close_iteration,
    };
    use chrono::{TimeZone, Utc};
    use std::sync::Arc;

    fn seed_member(number: u64, state: StoryState, iter_id: Option<u64>) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("[Feature] member-{number}"),
            state,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        if let Some(iter) = iter_id {
            s.iteration_id = Some(iter);
            s.raw_labels = vec![
                format!("{state:?}").to_lowercase(),
                format!("iteration-#{iter}"),
            ];
        } else {
            s.raw_labels = vec![format!("{state:?}").to_lowercase()];
        }
        s
    }

    fn seed_iteration_issue(number: u64, theme: &str) -> Story {
        let mut s = Story::new_minimal(
            number,
            format!("Iteration {theme}"),
            StoryState::Icebox,
            Utc.with_ymd_and_hms(2026, 5, 12, 0, 0, 0).unwrap(),
            "pm".to_string(),
        );
        s.story_type = StoryType::Iteration;
        s.raw_labels = vec!["iteration".to_string()];
        s
    }

    async fn build_refresh(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
        let store = Arc::new(BoardStore::new());
        let adapter = Arc::new(InMemorySourceAdapter::new());
        adapter.seed_stories(stories);
        let orch = SyncOrchestrator::new(store, adapter);
        let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
        orch.refresh_all().await.unwrap();
        orch
    }

    #[tokio::test]
    async fn promotes_each_member_to_ready_and_closes_iteration_issue() {
        // AC1 + AC2 + AC3: close GH issue + promote members + iteration_id cleared.
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(1400)),
            seed_member(201, StoryState::Icebox, Some(1400)),
        ])
        .await;

        promote_members_then_close_iteration(&refresh, 1400, &[200, 201])
            .await
            .expect("canonical-via-GH close_iteration succeeds");

        let iter = refresh.story(1400).expect("iteration-issue still exists");
        assert!(iter.is_closed, "iteration-issue closed (AC1)");

        let s200 = refresh.story(200).expect("member 200 exists");
        assert_eq!(s200.state, StoryState::Ready, "AC2: member 200 promoted");
        assert_eq!(s200.iteration_id, None, "AC3: iteration_id cleared");

        let s201 = refresh.story(201).expect("member 201 exists");
        assert_eq!(s201.state, StoryState::Ready, "AC2: member 201 promoted");
        assert_eq!(s201.iteration_id, None, "AC3: iteration_id cleared");
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_blocks_close_when_stories_in_ready() {
        // AC5: #1258 r3 ready-empty guard preserved at canonical-via-GH axis.
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(900, StoryState::Ready, None),
            seed_member(200, StoryState::Icebox, Some(1400)),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
        assert_eq!(blockers.len(), 1, "ready-queue blocker surfaces");
        assert_eq!(blockers[0].number, 900);
        assert_eq!(blockers[0].state, "ready");
    }

    #[tokio::test]
    async fn ready_queue_empty_guard_passes_when_no_ready_stories() {
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(1400)),
        ])
        .await;
        let blockers = collect_ready_queue_blockers(refresh.as_ref()).await;
        assert!(
            blockers.is_empty(),
            "no ready blockers — close proceeds: {blockers:?}"
        );
    }

    #[tokio::test]
    async fn closed_iteration_issue_remains_queryable_for_history_preservation() {
        // AC7 + falsifier (h): #1278 sister — closed Iteration-issue stays
        // accessible as historical-artifact (not deleted from store).
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(1400)),
        ])
        .await;
        promote_members_then_close_iteration(&refresh, 1400, &[200])
            .await
            .unwrap();
        let iter = refresh.story(1400).expect("history-preserved iteration");
        assert!(iter.is_closed);
        assert!(matches!(iter.story_type, StoryType::Iteration));
        assert_eq!(iter.title, "Iteration Pegasus");
    }

    #[tokio::test]
    async fn empty_member_list_closes_iteration_issue_gracefully() {
        // Falsifier (g): graceful failure on no-members. Empty member-list
        // is the boundary case: iteration-issue closes; no member-promotions.
        let refresh = build_refresh(vec![seed_iteration_issue(1400, "Pegasus")]).await;
        promote_members_then_close_iteration(&refresh, 1400, &[])
            .await
            .expect("empty-member close succeeds");
        let iter = refresh.story(1400).unwrap();
        assert!(iter.is_closed);
    }

    #[tokio::test]
    async fn iteration_close_uses_canonical_mutation_chain_for_member_promotion() {
        // Falsifier (b): member-stories promoted via canonical mutation
        // chain (not direct store-write). Verified by subscribing to
        // BoardStore events: per #1282 fire-ordering, each member-promotion
        // emits TagAdded("ready") + StateTransitioned(Ready) + TagRemoved
        // ("iteration-#NNNN"), and the iteration-issue close emits
        // StoryClosed — all visible to subscribers.
        let refresh = build_refresh(vec![
            seed_iteration_issue(1400, "Pegasus"),
            seed_member(200, StoryState::Icebox, Some(1400)),
        ])
        .await;
        let store = refresh.store();
        let mut rx = store.subscribe();
        promote_members_then_close_iteration(&refresh, 1400, &[200])
            .await
            .unwrap();
        let mut events = Vec::new();
        while let Ok(e) = rx.try_recv() {
            events.push(e);
        }
        let names: Vec<&str> = events
            .iter()
            .map(|e| match e {
                crate::board::BoardEvent::TagAdded { .. } => "TagAdded",
                crate::board::BoardEvent::TagRemoved { .. } => "TagRemoved",
                crate::board::BoardEvent::StateTransitioned { .. } => "StateTransitioned",
                crate::board::BoardEvent::StoryClosed { .. } => "StoryClosed",
                _ => "other",
            })
            .collect();
        assert!(
            names.contains(&"TagAdded"),
            "TagAdded fired (ready label): {names:?}"
        );
        assert!(
            names.contains(&"StateTransitioned"),
            "StateTransitioned fired: {names:?}"
        );
        assert!(
            names.contains(&"TagRemoved"),
            "TagRemoved fired (iteration label cleared): {names:?}"
        );
        assert!(
            names.contains(&"StoryClosed"),
            "StoryClosed fired (iteration-issue): {names:?}"
        );
    }
}
