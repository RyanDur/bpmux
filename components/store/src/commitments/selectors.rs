use serde_json::Value;

use message::payload_matches_filter;

use super::CommitmentsState;

pub fn matching_users(state: &CommitmentsState, event_class: &str, payload: &Value) -> Vec<String> {
    state
        .by_user
        .iter()
        .filter(|(_user_id, commitments)| {
            commitments.iter().any(|commitment| {
                commitment.event_class == event_class
                    && payload_matches_filter(commitment.filter.as_ref(), payload)
            })
        })
        .map(|(user_id, _commitments)| user_id.clone())
        .collect()
}
