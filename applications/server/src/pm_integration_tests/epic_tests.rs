#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use chrono::{TimeZone, Utc};
use product_management::epic::{EpicAction, EpicState, reduce};
use product_management::story::{Story, StoryId, Work};
use source_codec::mapper::decode_epic_from_story;

#[test]
fn epic_own_state_label_change_is_not_an_epic_change() {
    let filed_at = Utc.with_ymd_and_hms(2026, 5, 20, 0, 0, 0).unwrap();
    let story_ready_state = Story::new_minimal(
        StoryId::new(500),
        "epic-500".to_string(),
        Work::Ready,
        filed_at,
        "pm".to_string(),
    );
    let story_accepted = Story::new_minimal(
        StoryId::new(500),
        "epic-500".to_string(),
        Work::Accepted,
        filed_at,
        "pm".to_string(),
    );
    let epic_before = decode_epic_from_story(&story_ready_state, false);
    let epic_after = decode_epic_from_story(&story_accepted, false);
    let seeded = reduce(
        &EpicState::default(),
        &EpicAction::Seeded(vec![epic_before]),
    )
    .expect("seeding installs the epic");
    let result = reduce(&seeded, &EpicAction::Upserted(vec![epic_after]));
    assert!(
        result.is_none(),
        "an upsert differing only in the epic's own (shed) state label must reduce to None — the wire epic state derives from children, so that label never carried an observable difference"
    );
}
