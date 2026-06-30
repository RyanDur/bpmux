use super::Outcome;

#[derive(Default, Clone)]
pub struct OutcomeState {
    pub outcomes: Vec<Outcome>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutcomeAction {
    Seeded(Vec<Outcome>),
    Upserted(Vec<Outcome>),
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
        OutcomeAction::Upserted(incoming) => {
            let mut outcomes = state.outcomes.clone();
            for item in incoming {
                match outcomes
                    .iter()
                    .position(|existing| existing.id() == item.id())
                {
                    Some(index) => outcomes[index] = item.clone(),
                    None => outcomes.push(item.clone()),
                }
            }
            if outcomes == state.outcomes {
                None
            } else {
                Some(OutcomeState { outcomes })
            }
        }
    }
}

pub fn all(state: &OutcomeState) -> Vec<Outcome> {
    state.outcomes.clone()
}
