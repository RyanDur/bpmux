use std::sync::Arc;

use chrono::{TimeZone, Utc};

use super::{meta, seed_story};
use crate::product_management::board::adapters::in_memory::InMemorySourceAdapter;
use crate::product_management::board::events::BoardEvent;
use crate::product_management::board::item::ProductItem;
use crate::product_management::board::orchestrator::SyncOrchestrator;
use crate::product_management::board::store::BoardStore;
use crate::product_management::outcome::{Outcome, OutcomeAction};
use crate::product_management::story::{Stage, Story, StoryId, StoryState};
use crate::store::StoreAction;

fn seed_outcome(id: u64, title: &str) -> ProductItem {
    ProductItem::Outcome(Outcome::from_story(&Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        StoryState::Icebox,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )))
}

#[tokio::test]
async fn mutating_an_outcome_dispatches_upserted_with_the_updated_outcome() {
    let store = Arc::new(BoardStore::new());
    store.seed_items(vec![seed_outcome(700, "north star")]);
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StoreAction>();
    let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter)).with_action_tx(tx);

    orch.mutate(BoardEvent::TagAdded {
        meta: meta(),
        id: StoryId::new(700),
        label: "bpmux-horizon-now".to_string(),
    })
    .unwrap();

    let mut upserted: Option<Vec<Outcome>> = None;
    while let Ok(action) = rx.try_recv() {
        if let StoreAction::Outcome(OutcomeAction::Upserted(outcomes)) = action {
            upserted = Some(outcomes);
        }
    }
    let outcomes = upserted.expect("an outcome mutate dispatches OutcomeAction::Upserted");
    assert_eq!(outcomes.len(), 1, "the single mutated outcome is upserted");
    assert_eq!(
        outcomes[0].id().saved_id(),
        Some(StoryId::new(700)),
        "the upserted outcome is the one that was mutated"
    );
}

#[tokio::test]
async fn mutating_a_plain_story_dispatches_no_outcome_action() {
    let store = Arc::new(BoardStore::new());
    store.seed_all(vec![seed_story(101, StoryState::Ready)]);
    let adapter = Arc::new(InMemorySourceAdapter::new());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<StoreAction>();
    let orch = SyncOrchestrator::new(Arc::clone(&store), Arc::clone(&adapter)).with_action_tx(tx);

    orch.mutate(BoardEvent::StateTransitioned {
        meta: meta(),
        id: StoryId::new(101),
        new_state: StoryState::InProgress(Stage::Dev),
    })
    .unwrap();

    while let Ok(action) = rx.try_recv() {
        assert!(
            !matches!(action, StoreAction::Outcome(_)),
            "a plain-story mutate must not touch the outcome slice"
        );
    }
}
