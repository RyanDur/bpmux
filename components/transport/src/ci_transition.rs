#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transition {
    Changed { new_conclusion: &'static str },
    Same,
    NoData,
}

pub type Observation = (Option<u64>, &'static str);

pub fn classify_transition(prev: Option<Observation>, current: Observation) -> Transition {
    match prev {
        Some(p) if p == current => Transition::Same,
        _ => Transition::Changed {
            new_conclusion: current.1,
        },
    }
}
