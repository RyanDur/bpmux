use serde_json::Value;

use crate::message::payload_matches_filter;

use super::CommitmentsState;

pub fn matching_roles(state: &CommitmentsState, event_class: &str, payload: &Value) -> Vec<String> {
    state
        .by_role
        .iter()
        .filter(|(_role, commitments)| {
            commitments.iter().any(|commitment| {
                commitment.event_class == event_class
                    && payload_matches_filter(commitment.filter.as_ref(), payload)
            })
        })
        .map(|(role, _commitments)| role.clone())
        .collect()
}
