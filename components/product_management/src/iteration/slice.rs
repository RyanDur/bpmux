use super::{Iteration, IterationId};

#[derive(Default, Clone)]
pub struct IterationState {
    pub iterations: Vec<Iteration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IterationAction {
    Seeded(Vec<Iteration>),
    Upserted(Vec<Iteration>),
}

pub fn reduce(state: &IterationState, action: &IterationAction) -> Option<IterationState> {
    match action {
        IterationAction::Seeded(iterations) => {
            if state.iterations == *iterations {
                None
            } else {
                Some(IterationState {
                    iterations: iterations.clone(),
                })
            }
        }
        IterationAction::Upserted(incoming) => {
            let mut iterations = state.iterations.clone();
            for item in incoming {
                match iterations
                    .iter()
                    .position(|existing| existing.id == item.id)
                {
                    Some(index) => iterations[index] = item.clone(),
                    None => iterations.push(item.clone()),
                }
            }
            if iterations == state.iterations {
                None
            } else {
                Some(IterationState { iterations })
            }
        }
    }
}

pub fn all(state: &IterationState) -> Vec<Iteration> {
    state.iterations.clone()
}

pub fn get(state: &IterationState, id: IterationId) -> Option<Iteration> {
    state
        .iterations
        .iter()
        .find(|iteration| iteration.id == id)
        .cloned()
}
