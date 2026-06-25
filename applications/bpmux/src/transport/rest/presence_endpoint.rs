#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::presence::*;
use crate::store::Presence;

#[test]
fn presence_from_state_accepts_the_three_agent_pushable_states() {
    assert_eq!(presence_from_state("busy"), Some(Presence::Busy));
    assert_eq!(presence_from_state("idle"), Some(Presence::Idle));
    assert_eq!(presence_from_state("help"), Some(Presence::Help));
}

#[test]
fn presence_from_state_is_case_and_whitespace_insensitive() {
    assert_eq!(
        presence_from_state("  HELP "),
        Some(Presence::Help),
        "the verb normalizes like agent_from_headers does for the role"
    );
}

#[test]
fn presence_from_state_rejects_states_an_agent_must_not_self_assign() {
    assert_eq!(
        presence_from_state("online"),
        None,
        "online is connection-derived (join sets it), not agent-pushable"
    );
    assert_eq!(
        presence_from_state("offline"),
        None,
        "offline is connection-derived (disconnect), not agent-pushable"
    );
    assert_eq!(
        presence_from_state("blocked"),
        None,
        "blocked is the owner-park's to set, not the agent's"
    );
    assert_eq!(presence_from_state("nonsense"), None);
}
