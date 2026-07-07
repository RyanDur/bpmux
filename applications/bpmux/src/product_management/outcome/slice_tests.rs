#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::{Outcome, OutcomeAction, OutcomeState, all, reduce};
use crate::integrations::github::mapper::decode_outcome_from_story;
use crate::product_management::story::{Story, StoryId, Work};
use chrono::{TimeZone, Utc};

fn outcome(id: u64, title: &str) -> Outcome {
    let story = Story::new_minimal(
        StoryId::new(id),
        title.to_string(),
        Work::NotStarted,
        Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    );
    decode_outcome_from_story(&story, &[])
}

#[test]
fn seeded_replaces_the_outcome_collection() {
    let next = reduce(
        &OutcomeState::default(),
        &OutcomeAction::Seeded(vec![outcome(700, "now thing")]),
    )
    .expect("seeding into empty state changes it");
    assert_eq!(all(&next).len(), 1);
    assert_eq!(all(&next)[0].title, "now thing");
}

#[test]
fn upsert_replaces_a_matching_outcome_and_appends_a_new_one() {
    let state = OutcomeState {
        outcomes: vec![outcome(700, "before")],
    };
    let next = reduce(
        &state,
        &OutcomeAction::Upserted(vec![outcome(700, "after"), outcome(701, "fresh")]),
    )
    .expect("upsert changes the collection");
    let all = all(&next);
    assert_eq!(
        all.len(),
        2,
        "matching id replaced in place, new id appended"
    );
    assert_eq!(
        all.iter()
            .find(|o| o.id == outcome(700, "x").id)
            .unwrap()
            .title,
        "after"
    );
}

#[test]
fn reseeding_identical_outcomes_is_a_noop() {
    let seeded = vec![outcome(700, "now thing")];
    let state = OutcomeState {
        outcomes: seeded.clone(),
    };
    assert!(
        reduce(&state, &OutcomeAction::Seeded(seeded)).is_none(),
        "re-seeding identical state must be a no-op so the slice listener does not spuriously broadcast"
    );
}
