use super::Outcome;

#[derive(Default, Clone)]
pub struct OutcomeState {
    pub outcomes: Vec<Outcome>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutcomeAction {
    Seeded(Vec<Outcome>),
}

pub fn reduce(state: &OutcomeState, action: &OutcomeAction) -> Option<OutcomeState> {
    match action {
        OutcomeAction::Seeded(outcomes) => {
            if state.outcomes == *outcomes {
                None
            } else {
                Some(OutcomeState {
                    outcomes: outcomes.clone(),
                })
            }
        }
    }
}

pub fn all(state: &OutcomeState) -> Vec<Outcome> {
    state.outcomes.clone()
}
