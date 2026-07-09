#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::owner_park::owner_park_presence;
use crate::product_management::iteration::{Iteration, IterationId};
use crate::product_management::story::{Acceptor, Story, StoryId, Work};
use crate::store::{Presence, RosterMember};
use chrono::{TimeZone, Utc};

fn story(id: u64, state: Work) -> Story {
    Story::new_minimal(
        StoryId::new(id),
        format!("story {id}"),
        state,
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
        "pm".to_string(),
    )
}

fn owner_parked_delivery(id: u64) -> Story {
    let mut s = story(id, Work::Acceptance);
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
        identity: "dev:claude-code".to_string(),
        presence,
    }
}

fn member(routing_id: &str, identity: &str, presence: Presence) -> RosterMember {
    RosterMember {
        routing_id: routing_id.to_string(),
        identity: identity.to_string(),
        presence,
    }
}

#[test]
fn with_no_current_iteration_a_not_started_story_does_not_block_the_owner_park() {
    let stories = vec![owner_parked_delivery(100), story(200, Work::NotStarted)];
    let changes = owner_park_presence(&stories, &[], &[dev_member(Presence::Online)], Some("dev"));
    assert_eq!(
        changes,
        vec![("dev-routing".to_string(), Presence::Blocked)],
        "without a current iteration nothing projects Ready, so the ready queue is empty and the owner-park engages for the delivering role"
    );
}

#[test]
fn a_projected_ready_member_keeps_the_ready_queue_non_empty_and_lifts_the_park() {
    let stories = vec![owner_parked_delivery(100), story(200, Work::NotStarted)];
    let changes = owner_park_presence(
        &stories,
        &current_iteration_of(&[200]),
        &[dev_member(Presence::Blocked)],
        Some("dev"),
    );
    assert_eq!(
        changes,
        vec![("dev-routing".to_string(), Presence::Online)],
        "a not-started current-iteration member projects Ready, so the queue is non-empty, the park lifts, and the blocked member comes back online"
    );
}

#[test]
fn park_blocks_only_the_delivering_member_never_a_same_role_sibling() {
    let stories = vec![owner_parked_delivery(100)];
    let roster = vec![
        member("routing-aaa", "agent-aaa:ws", Presence::Online),
        member("routing-bbb", "agent-bbb:ws", Presence::Online),
    ];
    let changes = owner_park_presence(&stories, &[], &roster, Some("agent-aaa"));
    assert_eq!(
        changes,
        vec![("routing-aaa".to_string(), Presence::Blocked)],
        "the park blocks exactly the delivering member; two peers that share a derived RosterRole (both Other) but hold distinct identities must not collapse into one match — matching on the derived role instead of the delivering identity would over-block routing-bbb, a reachable regression the roster's per-id keying makes possible"
    );
}
