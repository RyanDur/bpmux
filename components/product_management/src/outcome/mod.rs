use chrono::{DateTime, Utc};

use crate::iteration::IterationId;
use crate::story::Horizon;

entity_id!(OutcomeId);

mod slice;
pub use slice::{OutcomeAction, OutcomeState, all, reduce};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeLifecycle {
    Active {
        horizon: Option<Horizon>,
        horizon_order: Option<u32>,
    },
    Retired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub id: OutcomeId,
    pub title: String,
    pub body: Option<String>,
    pub filed_at: DateTime<Utc>,
    pub filed_by: String,
    pub lifecycle: OutcomeLifecycle,
    pub iterations: Vec<IterationId>,
}

impl Outcome {
    pub fn position_of(&self, iteration_id: IterationId) -> Option<u32> {
        self.iterations
            .iter()
            .position(|n| *n == iteration_id)
            .map(|index| index as u32)
    }

    pub fn horizon(&self) -> Option<Horizon> {
        match &self.lifecycle {
            OutcomeLifecycle::Active { horizon, .. } => *horizon,
            OutcomeLifecycle::Retired => None,
        }
    }

    pub fn horizon_order(&self) -> Option<u32> {
        match &self.lifecycle {
            OutcomeLifecycle::Active { horizon_order, .. } => *horizon_order,
            OutcomeLifecycle::Retired => None,
        }
    }

    pub fn is_retired(&self) -> bool {
        matches!(self.lifecycle, OutcomeLifecycle::Retired)
    }

    pub fn retired(&self) -> Outcome {
        Outcome {
            lifecycle: OutcomeLifecycle::Retired,
            ..self.clone()
        }
    }

    pub fn with_horizon(&self, horizon: Option<Horizon>, horizon_order: Option<u32>) -> Outcome {
        let lifecycle = match self.lifecycle {
            OutcomeLifecycle::Active { .. } => OutcomeLifecycle::Active {
                horizon,
                horizon_order,
            },
            OutcomeLifecycle::Retired => OutcomeLifecycle::Retired,
        };
        Outcome {
            lifecycle,
            ..self.clone()
        }
    }

    pub fn with_title(&self, title: &str) -> Outcome {
        Outcome {
            title: title.to_string(),
            ..self.clone()
        }
    }
}
