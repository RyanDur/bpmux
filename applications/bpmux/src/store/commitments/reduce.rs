use crate::message::payload_matches_filter;

use super::state::Commitment;
use super::{CommitmentsAction, CommitmentsState};

pub fn reduce(state: &CommitmentsState, action: &CommitmentsAction) -> Option<CommitmentsState> {
    match action {
        CommitmentsAction::Committed {
            role,
            event_class,
            filter,
        } => {
            let mut by_role = state.by_role.clone();
            by_role.entry(role.clone()).or_default().push(Commitment {
                event_class: event_class.clone(),
                filter: filter.clone(),
            });
            Some(CommitmentsState { by_role })
        }
        CommitmentsAction::CommitmentsMatched {
            roles,
            event_class,
            payload,
        } => {
            let mut by_role = state.by_role.clone();
            let mut fired = false;
            by_role.retain(|role, commitments| {
                if roles.contains(role) {
                    commitments.retain(|commitment| {
                        let matches = commitment.event_class == *event_class
                            && payload_matches_filter(commitment.filter.as_ref(), payload);
                        fired |= matches;
                        !matches
                    });
                }
                !commitments.is_empty()
            });
            fired.then_some(CommitmentsState { by_role })
        }
    }
}
