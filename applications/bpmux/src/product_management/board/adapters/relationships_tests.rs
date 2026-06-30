#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::relationships::{
    EpicRelationships, IterationRelationships, OutcomeRelationships, Relationships,
    RoadmapRelationships, StoryRelationships,
};
use crate::product_management::epic::EpicId;
use crate::product_management::iteration::IterationId;
use crate::product_management::outcome::OutcomeId;
use crate::product_management::roadmap::RoadmapId;
use crate::product_management::story::StoryId;

#[test]
fn iteration_relationships_round_trip_through_body_preserving_order() {
    let rel = IterationRelationships {
        outcome: Some(OutcomeId::new(700)),
        stories: vec![StoryId::new(1399), StoryId::new(1397), StoryId::new(1398)],
    };
    let body = rel.write_into_body("Iteration prose.");
    assert_eq!(IterationRelationships::read_from_body(&body), Some(rel));
}

#[test]
fn story_relationships_omit_epic_when_absent() {
    let rel = StoryRelationships {
        iteration: Some(IterationId::new(1397)),
        epic: None,
    };
    let body = rel.write_into_body("Story prose.");
    assert!(
        !body.contains("epic"),
        "an absent epic must not appear in the body block"
    );
    assert_eq!(StoryRelationships::read_from_body(&body), Some(rel));
}

#[test]
fn story_relationships_carry_both_parents_when_present() {
    let rel = StoryRelationships {
        iteration: Some(IterationId::new(1397)),
        epic: Some(EpicId::new(457)),
    };
    let body = rel.write_into_body("prose");
    assert_eq!(StoryRelationships::read_from_body(&body), Some(rel));
}

#[test]
fn read_returns_none_when_body_has_no_block() {
    assert_eq!(
        IterationRelationships::read_from_body("just prose, no block"),
        None
    );
}

#[test]
fn roadmap_relationships_round_trip_preserving_outcome_order() {
    let rel = RoadmapRelationships {
        outcomes: vec![
            OutcomeId::new(702),
            OutcomeId::new(700),
            OutcomeId::new(701),
        ],
    };
    let body = rel.write_into_body("");
    assert_eq!(RoadmapRelationships::read_from_body(&body), Some(rel));
}

#[test]
fn epic_relationships_carry_member_stories() {
    let rel = EpicRelationships {
        stories: vec![StoryId::new(1397), StoryId::new(1500)],
    };
    let body = rel.write_into_body("Epic prose.");
    assert_eq!(EpicRelationships::read_from_body(&body), Some(rel));
}

#[test]
fn outcome_relationships_round_trip_through_body() {
    let rel = OutcomeRelationships {
        roadmap: Some(RoadmapId::new(9000)),
        iterations: vec![IterationId::new(710), IterationId::new(720)],
    };
    let body = rel.write_into_body("Outcome prose.");
    assert_eq!(OutcomeRelationships::read_from_body(&body), Some(rel));
}
