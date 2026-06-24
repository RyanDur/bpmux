#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::order_backfill::*;
use crate::product_management::board::RefreshDriver;
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::domain::{Story, StoryState, StoryType};
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use chrono::{TimeZone, Utc};
use std::sync::Arc;

fn iteration(number: u64, order: Option<u32>) -> Story {
    let mut s = Story::new_minimal(
        number,
        format!("Iteration {number}"),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 6, 15, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    s.story_type = StoryType::Iteration;
    s.iteration_order = order;
    s.raw_labels = match order {
        Some(p) => vec!["iteration".to_string(), format!("iteration-order.{p}")],
        None => vec!["iteration".to_string()],
    };
    s
}

async fn refresh_with(stories: Vec<Story>) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let orch = SyncOrchestrator::new(store, adapter);
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch);
    orch.refresh_all().await.unwrap();
    orch
}

#[tokio::test]
async fn backfills_order_less_iterations_in_issue_number_order_1735() {
    let orch = refresh_with(vec![
        iteration(1678, None),
        iteration(1545, None),
        iteration(1575, None),
    ])
    .await;
    backfill_iteration_order(orch.as_ref());
    let mut got: Vec<(u64, u32)> = orch
        .all_stories()
        .into_iter()
        .filter(|s| s.story_type == StoryType::Iteration)
        .map(|s| (s.number, s.iteration_order.expect("backfilled")))
        .collect();
    got.sort_by_key(|(_, o)| *o);
    assert_eq!(got, vec![(1545, 100), (1575, 200), (1678, 300)]);
}

#[tokio::test]
async fn idempotent_skips_already_ordered_and_appends_after_max_1735() {
    let orch = refresh_with(vec![iteration(1545, Some(100)), iteration(1678, None)]).await;
    backfill_iteration_order(orch.as_ref());
    let stories = orch.all_stories();
    let ordered = stories.iter().find(|s| s.number == 1545).unwrap();
    let healed = stories.iter().find(|s| s.number == 1678).unwrap();
    assert_eq!(
        ordered.iteration_order,
        Some(100),
        "already-ordered untouched"
    );
    assert_eq!(
        healed.iteration_order,
        Some(200),
        "order-less iteration appended AFTER the existing max"
    );
}

#[tokio::test]
async fn no_op_when_all_ordered_1735() {
    let orch = refresh_with(vec![iteration(1545, Some(100))]).await;
    backfill_iteration_order(orch.as_ref());
    let s = orch
        .all_stories()
        .into_iter()
        .find(|s| s.number == 1545)
        .unwrap();
    assert_eq!(s.iteration_order, Some(100));
}
