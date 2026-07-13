#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;

use crate::bridge::dispatch::dispatch_list_ready;
use crate::integrations::in_memory::InMemorySourceAdapter;
use crate::sync::{RefreshDriver, SyncOrchestrator};
use chrono::{TimeZone, Utc};
use product_management::iteration::{Iteration, IterationId};
use product_management::story::{Story, StoryId, Work};

fn member(id: u64) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("member-{id}"),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 7, 7, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn iteration(id: u64, is_current: bool, complete: bool, stories: Vec<u64>) -> Iteration {
    Iteration {
        id: IterationId::new(id),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 7, 7, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: stories.into_iter().map(StoryId::new).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current,
        complete,
    }
}

async fn ready_ids(stories: Vec<Story>, iterations: Vec<Iteration>) -> Vec<u64> {
    let adapter = Arc::new(InMemorySourceAdapter::new());
    adapter.seed_stories(stories);
    let refresh: Arc<dyn RefreshDriver> = Arc::new(SyncOrchestrator::new(adapter));
    crate::store::bind_seeded_refresh(&refresh, vec![], iterations, vec![]);
    refresh.refresh_all().await.unwrap();
    let raw = dispatch_list_ready(Some(&refresh)).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&raw).unwrap();
    parsed
        .as_array()
        .expect("list_ready returns an array")
        .iter()
        .map(|row| row["id"].as_u64().expect("row id is numeric"))
        .collect()
}

#[tokio::test]
async fn a_completed_iteration_flagged_current_never_lists_its_members_as_ready() {
    let ids = ready_ids(
        vec![member(5001)],
        vec![iteration(9000, true, true, vec![5001])],
    )
    .await;

    assert!(
        ids.is_empty(),
        "a completed iteration's member is Icebox, never ready; got {ids:?}"
    );
}

#[tokio::test]
async fn a_stale_completed_current_does_not_usurp_the_open_iterations_ready_order() {
    let ids = ready_ids(
        vec![member(5001), member(5002)],
        vec![
            iteration(9000, true, true, vec![5001]),
            iteration(9001, true, false, vec![5002, 5001]),
        ],
    )
    .await;

    assert_eq!(
        ids,
        vec![5002, 5001],
        "the open current iteration's membership order wins; the stale completed-current is \
         ignored as the order source; got {ids:?}"
    );
}
