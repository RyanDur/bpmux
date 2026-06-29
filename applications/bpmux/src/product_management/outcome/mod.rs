use crate::product_management::board::adapters::gh_label_codec::GhLabelCodec;
use crate::product_management::board::adapters::github::{
    horizon_from_labels, horizon_order_from_labels,
};
use crate::product_management::board::adapters::relationships::{
    OutcomeRelationships, Relationships,
};
use crate::product_management::board::item::Content;
use crate::product_management::story::{Horizon, Story, StoryKey, StoryState};

entity_id!(OutcomeId);

#[cfg(test)]
mod outcome_tests;

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
    pub content: Content,
    pub lifecycle: OutcomeLifecycle,
    pub iterations: Vec<u64>,
}

impl Outcome {
    pub fn from_story(story: &Story) -> Self {
        let content = Content {
            id: story.id(),
            title: story.title().to_string(),
            body: story.body().map(str::to_string),
            filed_at: story.filed_at(),
            filed_by: story.filed_by().to_string(),
        };
        let lifecycle = if story.state() == StoryState::Retired {
            OutcomeLifecycle::Retired
        } else {
            OutcomeLifecycle::Active {
                horizon: horizon_from_labels(story.raw_labels()),
                horizon_order: horizon_order_from_labels(story.raw_labels()),
            }
        };
        let iterations = story
            .body()
            .and_then(OutcomeRelationships::read_from_body)
            .unwrap_or_default()
            .iterations;
        Outcome {
            content,
            lifecycle,
            iterations,
        }
    }

    pub fn id(&self) -> StoryKey {
        self.content.id
    }

    pub fn position_of(&self, iteration_number: u64) -> Option<u32> {
        self.iterations
            .iter()
            .position(|n| *n == iteration_number)
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
            content: self.content.clone(),
            lifecycle: OutcomeLifecycle::Retired,
            iterations: self.iterations.clone(),
        }
    }

    pub fn with_horizon_labels(&self, labels: &[String]) -> Outcome {
        let lifecycle = match self.lifecycle {
            OutcomeLifecycle::Active { .. } => OutcomeLifecycle::Active {
                horizon: horizon_from_labels(labels),
                horizon_order: horizon_order_from_labels(labels),
            },
            OutcomeLifecycle::Retired => OutcomeLifecycle::Retired,
        };
        Outcome {
            content: self.content.clone(),
            lifecycle,
            iterations: self.iterations.clone(),
        }
    }

    pub fn with_body(&self, body: &str) -> Outcome {
        Outcome {
            content: Content {
                body: Some(body.to_string()),
                ..self.content.clone()
            },
            lifecycle: self.lifecycle.clone(),
            iterations: OutcomeRelationships::read_from_body(body)
                .unwrap_or_default()
                .iterations,
        }
    }

    pub fn with_title(&self, title: &str) -> Outcome {
        Outcome {
            content: Content {
                title: title.to_string(),
                ..self.content.clone()
            },
            lifecycle: self.lifecycle.clone(),
            iterations: self.iterations.clone(),
        }
    }

    pub fn encoded_labels(&self) -> Vec<String> {
        let mut labels = Vec::new();
        if let OutcomeLifecycle::Active {
            horizon,
            horizon_order,
        } = self.lifecycle
        {
            if let Some(horizon) = horizon {
                labels.push(GhLabelCodec.encode_horizon(horizon));
            }
            if let Some(order) = horizon_order {
                labels.push(GhLabelCodec.encode_horizon_order(order));
            }
        }
        labels
    }
}
