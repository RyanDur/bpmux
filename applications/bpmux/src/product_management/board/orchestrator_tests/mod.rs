#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod codec_injection_race_window_tests;
mod ingest_drain_tests;
mod outbox_coalesce_tests;
mod outbox_persistence_tests;
mod reconcile_drain_via_refresh_tests;
mod refresh_fetch_tests;
mod slice_7_mutation_dispatch_tests;
mod staleness_tests;

use chrono::{TimeZone, Utc};

use super::events::EventMeta;
use super::peer::PeerId;
use crate::product_management::story::{BoardState, Story, StoryId};

pub(super) fn meta() -> EventMeta {
    EventMeta {
        actor: "dev".to_string(),
        ts: Utc.with_ymd_and_hms(2026, 4, 24, 16, 0, 0).unwrap(),
        peer_id: PeerId::local(),
    }
}

pub(super) fn seed_story(id: u64, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("title-{id}"),
        state,
        Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}
