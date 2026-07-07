use chrono::{DateTime, Utc};

use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{StoryId, Verdict};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerdictRecord {
    pub story: StoryId,
    pub verdict: Verdict,
    pub ts: DateTime<Utc>,
    pub actor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SharedHistory {
    records: Vec<VerdictRecord>,
}

impl SharedHistory {
    pub fn records(&self) -> &[VerdictRecord] {
        &self.records
    }
}

fn record_of(event: &StoryEvent) -> Option<VerdictRecord> {
    match &event.kind {
        StoryEventKind::VerdictCast { verdict, .. } => Some(VerdictRecord {
            story: event.id,
            verdict: verdict.clone(),
            ts: event.meta.ts,
            actor: event.meta.actor.clone(),
        }),
        _ => None,
    }
}

pub fn react_to_story(state: &SharedHistory, action: &StoryAction) -> Option<SharedHistory> {
    let appended: Vec<VerdictRecord> = match action {
        StoryAction::Event(event) => record_of(event).into_iter().collect(),
        StoryAction::Events(events) => events.iter().filter_map(record_of).collect(),
        _ => Vec::new(),
    };
    if appended.is_empty() {
        return None;
    }
    let mut records = state.records.clone();
    records.extend(appended);
    Some(SharedHistory { records })
}
