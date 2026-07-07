#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::owner_park::owner_park_presence;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::{Acceptor, BoardState, Story, StoryId};
use crate::store::{Presence, RosterMember, RosterRole};
use chrono::{TimeZone, Utc};

fn story(id: u64, state: BoardState) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("story {id}"),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn owner_parked_delivery(id: u64) -> Story {
    let mut s = story(id, BoardState::Acceptance);
    s.acceptor = Some(Acceptor::Owner);
    s
}

fn current_iteration_of(members: &[u64]) -> Vec<Iteration> {
    vec![Iteration {
        id: IterationId::new(9000),
        title: String::new(),
        body: None,
        filed_at: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        filed_by: "pm".to_string(),
        outcome: None,
        iteration_order: None,
        stories: members.iter().map(|n| StoryId::new(*n)).collect(),
        review_closers: Vec::new(),
        dev_ok_by: Vec::new(),
        is_current: true,
        complete: false,
    }]
}

fn dev_member(presence: Presence) -> RosterMember {
    RosterMember {
        routing_id: "dev-routing".to_string(),
        name: "dev".to_string(),
        identity: "dev@bpmux".to_string(),
        role: RosterRole::Dev,
        presence,
    }
}

#[test]
fn with_no_current_iteration_a_stored_ready_story_does_not_block_the_owner_park() {
    let stories = vec![
        owner_parked_delivery(100),
        story(200, BoardState::NotStarted),
    ];
    let changes = owner_park_presence(&stories, &[], &[dev_member(Presence::Online)], Some("dev"));
    assert_eq!(
        changes,
        vec![("dev-routing".to_string(), Presence::Blocked)],
        "without a current iteration nothing projects Ready, so the ready queue is empty and the owner-park engages for the delivering role"
    );
}

#[test]
fn a_projected_ready_member_keeps_the_ready_queue_non_empty_and_lifts_the_park() {
    let stories = vec![
        owner_parked_delivery(100),
        story(200, BoardState::NotStarted),
    ];
    let changes = owner_park_presence(
        &stories,
        &current_iteration_of(&[200]),
        &[dev_member(Presence::Blocked)],
        Some("dev"),
    );
    assert_eq!(
        changes,
        vec![("dev-routing".to_string(), Presence::Online)],
        "a stored-Icebox current-iteration member projects Ready, so the queue is non-empty, the park lifts, and the blocked member comes back online"
    );
}
