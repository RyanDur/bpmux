use super::{BoardState, Gate, GateComment, Stage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: BoardState,
    pub to: BoardState,
    pub ts: DateTime<Utc>,
    pub actor: String,
    pub comment: Option<String>,
}

pub fn replay_transitions(gate_comments: &[GateComment]) -> Vec<StateTransition> {
    gate_comments
        .iter()
        .filter_map(|gc| {
            let (from, to) = match gc.gate {
                Gate::Filed => return None,
                Gate::Pickup => (BoardState::Ready, BoardState::InProgress(Stage::Dev)),
                Gate::Delivered => (
                    BoardState::InProgress(Stage::Dev),
                    BoardState::InProgress(Stage::DeskCheck),
                ),
                Gate::PassDeskCheck => (
                    BoardState::InProgress(Stage::DeskCheck),
                    BoardState::Delivered,
                ),
                Gate::FailDeskCheck => (
                    BoardState::InProgress(Stage::DeskCheck),
                    BoardState::InProgress(Stage::Dev),
                ),
                Gate::Rejected => (BoardState::Delivered, BoardState::Ready),
                Gate::Accepted => (BoardState::Delivered, BoardState::Accepted),
            };
            Some(StateTransition {
                from,
                to,
                ts: gc.ts,
                actor: gc.actor.clone(),
                comment: gc.payload.clone(),
            })
        })
        .collect()
}

fn state_ordinal(s: BoardState) -> u8 {
    match s {
        BoardState::Icebox => 0,
        BoardState::Ready => 1,
        BoardState::InProgress(Stage::Dev) => 2,
        BoardState::InProgress(Stage::DeskCheck) => 3,
        BoardState::Delivered => 4,
        BoardState::Accepted | BoardState::Retired => 5,
    }
}

pub fn is_backward(t: &StateTransition) -> bool {
    state_ordinal(t.to) < state_ordinal(t.from)
}

pub fn most_recent_backward_transition(
    transitions: &[StateTransition],
) -> Option<&StateTransition> {
    transitions.last().filter(|t| is_backward(t))
}
