#![allow(clippy::unwrap_used, clippy::expect_used)]

mod hydration_reduce_tests;
mod label_projection_tests;
mod lifecycle_reduce_tests;
mod parent_epic_reduce_tests;
mod write_path_tests;

use chrono::{TimeZone, Utc};

use product_management::story::events::{EventMeta, StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{Story, StoryId, StoryKey, Work};
use store::AppAction;
use store_test_support::SeededStore;

fn seeded() -> SeededStore {
    SeededStore::new(Vec::new(), Vec::new(), Vec::new())
}

fn meta(actor: &str, ts_year: i32) -> EventMeta {
    EventMeta {
        actor: actor.to_string(),
        actor_id: None,
        ts: Utc.with_ymd_and_hms(ts_year, 1, 1, 0, 0, 0).unwrap(),
    }
}

fn apply(store: &SeededStore, event: StoryEvent) {
    store.dispatch(AppAction::Story(StoryAction::Event(event)));
}

fn seed(store: &SeededStore, stories: Vec<Story>) {
    store.dispatch(AppAction::Story(StoryAction::Seeded(stories)));
}

fn story_at(store: &SeededStore, id: u64) -> Option<Story> {
    store.select().select(|state| {
        state
            .stories
            .get(&StoryKey::Saved(StoryId::new(id)))
            .cloned()
    })
}

fn minimal(id: u64, title: &str, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn created(store: &SeededStore, id: u64, title: &str, state: Work) {
    apply(
        store,
        StoryEvent::new(
            meta("pm", 2026),
            StoryId::new(id),
            StoryEventKind::StoryCreated {
                title: title.to_string(),
                body: "b".to_string(),
                initial_state: state,
                story_type: product_management::story::StoryKind::Feature,
            },
        ),
    );
}
