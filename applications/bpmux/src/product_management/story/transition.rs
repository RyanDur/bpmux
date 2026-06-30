use super::{Gate, GateComment, Stage, StoryState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub from: StoryState,
    pub to: StoryState,
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
                Gate::Pickup => (StoryState::Ready, StoryState::InProgress(Stage::Dev)),
                Gate::Delivered => (
                    StoryState::InProgress(Stage::Dev),
                    StoryState::InProgress(Stage::DeskCheck),
                ),
                Gate::PassDeskCheck => (
                    StoryState::InProgress(Stage::DeskCheck),
                    StoryState::Delivered,
                ),
                Gate::FailDeskCheck => (
                    StoryState::InProgress(Stage::DeskCheck),
                    StoryState::InProgress(Stage::Dev),
                ),
                Gate::Rejected => (StoryState::Delivered, StoryState::Ready),
                Gate::Accepted => (StoryState::Delivered, StoryState::Accepted),
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

fn state_ordinal(s: StoryState) -> u8 {
    match s {
        StoryState::Icebox => 0,
        StoryState::Ready => 1,
        StoryState::InProgress(Stage::Dev) => 2,
        StoryState::InProgress(Stage::DeskCheck) => 3,
        StoryState::Delivered => 4,
        StoryState::Accepted | StoryState::Retired => 5,
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
