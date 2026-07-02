#![allow(clippy::unwrap_used, clippy::expect_used)]

mod hydration_reduce_tests;
mod label_projection_tests;
mod lifecycle_reduce_tests;
mod parent_epic_reduce_tests;
mod write_path_tests;

use chrono::{TimeZone, Utc};

use crate::product_management::board::events::{EventMeta, StoryEvent};
use crate::product_management::board::peer::PeerId;
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{BoardState, Story, StoryId, StoryKey};
use crate::store::{AppAction, SeededStore};

fn seeded() -> SeededStore {
    SeededStore::new(Vec::new(), Vec::new(), Vec::new())
}

fn meta(actor: &str, ts_year: i32, peer: &str) -> EventMeta {
    EventMeta {
        actor: actor.to_string(),
        ts: Utc.with_ymd_and_hms(ts_year, 1, 1, 0, 0, 0).unwrap(),
        peer_id: PeerId::named(peer),
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

fn minimal(id: u64, title: &str, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn created(store: &SeededStore, id: u64, title: &str, state: BoardState) {
    apply(
        store,
        StoryEvent::StoryCreated {
            meta: meta("pm", 2026, "local"),
            id: StoryId::new(id),
            title: title.to_string(),
            body: "b".to_string(),
            initial_state: state,
            story_type: crate::product_management::story::StoryKind::Feature,
        },
    );
}
