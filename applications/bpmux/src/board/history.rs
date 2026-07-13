use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use product_management::story::events::{StoryEvent, StoryEventKind};
use product_management::story::slice::StoryAction;
use product_management::story::{StoryId, Work};
use workflow::StageId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlacementRecord {
    pub story: StoryId,
    pub placement: Option<StageId>,
    pub to: Work,
    pub ts: DateTime<Utc>,
    pub actor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PlacementHistory {
    records: Vec<PlacementRecord>,
}

impl PlacementHistory {
    pub fn records(&self) -> &[PlacementRecord] {
        &self.records
    }
}

fn record_of(event: &StoryEvent) -> Option<PlacementRecord> {
    match &event.kind {
        StoryEventKind::StateTransitioned {
            new_state,
            placement,
        } => Some(PlacementRecord {
            story: event.id,
            placement: placement.clone(),
            to: new_state.clone(),
            ts: event.meta.ts,
            actor: event.meta.actor.clone(),
        }),
        _ => None,
    }
}

pub fn react_to_story(state: &PlacementHistory, action: &StoryAction) -> Option<PlacementHistory> {
    let appended: Vec<PlacementRecord> = match action {
        StoryAction::Event(event) => record_of(event).into_iter().collect(),
        StoryAction::Events(events) => events.iter().filter_map(record_of).collect(),
        _ => Vec::new(),
    };
    if appended.is_empty() {
        return None;
    }
    let mut records = state.records.clone();
    records.extend(appended);
    Some(PlacementHistory { records })
}
