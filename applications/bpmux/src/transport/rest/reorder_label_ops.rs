#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::labels::*;
use crate::product_management::board::adapters::github::iteration_membership_from_labels;
use crate::product_management::board::{BoardEvent, BoardStore};
use crate::product_management::story::{Story, StoryState};

fn membered_fixture(number: u64, label: &str) -> Story {
    let mut story = Story::new_minimal(
        number,
        "reorder fixture".to_string(),
        StoryState::Ready,
        chrono::Utc::now(),
        "tester".to_string(),
    );
    story.raw_labels = vec![label.to_string()];
    story
}

fn membership_of(store: &BoardStore, number: u64) -> Option<(u64, u32)> {
    store
        .story(number)
        .and_then(|s| iteration_membership_from_labels(&s.raw_labels))
}

#[test]
fn reorder_keeps_story_membered_at_every_step_of_the_label_swap() {
    let store = BoardStore::new();
    store.seed_all(vec![membered_fixture(1687, "parent-iteration-#1655.125")]);
    assert!(
        membership_of(&store, 1687).is_some(),
        "fixture must start membered"
    );

    let payload = LabelMutationPayload {
        add: vec!["parent-iteration-#1655.106".to_string()],
        remove: vec!["parent-iteration-#1655.125".to_string()],
    };

    for op in reorder_label_ops(&payload) {
        let meta = crate::product_management::board::orchestrator::cache_fill_meta();
        let event = match op {
            LabelOp::Add(label) => BoardEvent::TagAdded {
                meta,
                number: 1687,
                label,
            },
            LabelOp::Remove(label) => BoardEvent::TagRemoved {
                meta,
                number: 1687,
                label,
            },
        };
        store.apply(&event);
        assert!(
            membership_of(&store, 1687).is_some(),
            "a reorder must never transiently orphan the story: removing the old parent-iteration label before adding the new one strips its only membership label, so a concurrent board read drops the row from the Now tab until refresh (the #1524 disappear)"
        );
    }

    assert_eq!(
        membership_of(&store, 1687),
        Some((1655, 106)),
        "after the swap the story is membered at the new position"
    );
}
