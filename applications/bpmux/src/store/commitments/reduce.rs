use crate::message::payload_matches_filter;

use super::state::Commitment;
use super::{CommitmentsAction, CommitmentsState};

pub fn reduce(state: &CommitmentsState, action: &CommitmentsAction) -> Option<CommitmentsState> {
    match action {
        CommitmentsAction::Committed {
            user_id,
            event_class,
            filter,
        } => {
            let mut by_user = state.by_user.clone();
            by_user
                .entry(user_id.clone())
                .or_default()
                .push(Commitment {
                    event_class: event_class.clone(),
                    filter: filter.clone(),
                });
            Some(CommitmentsState { by_user })
        }
        CommitmentsAction::CommitmentsMatched {
            user_ids,
            event_class,
            payload,
        } => {
            let mut by_user = state.by_user.clone();
            let mut fired = false;
            by_user.retain(|user_id, commitments| {
                if user_ids.contains(user_id) {
                    commitments.retain(|commitment| {
                        let matches = commitment.event_class == *event_class
                            && payload_matches_filter(commitment.filter.as_ref(), payload);
                        fired |= matches;
                        !matches
                    });
                }
                !commitments.is_empty()
            });
            fired.then_some(CommitmentsState { by_user })
        }
    }
}
