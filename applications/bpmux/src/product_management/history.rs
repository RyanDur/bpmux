use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::product_management::story::events::{StoryEvent, StoryEventKind};
use crate::product_management::story::slice::StoryAction;
use crate::product_management::story::{CastVerdict, StoryId, Verdict};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictRecord {
    pub story: StoryId,
    pub verdict: Verdict,
    pub ts: DateTime<Utc>,
    pub actor: String,
}

impl VerdictRecord {
    pub fn from_cast(story: StoryId, cast: &CastVerdict) -> Self {
        Self {
            story,
            verdict: cast.verdict.clone(),
            ts: cast.ts,
            actor: cast.actor.clone(),
        }
    }
}

#[derive(Clone)]
pub enum SharedHistoryAction {
    Backfilled {
        story: StoryId,
        records: Vec<VerdictRecord>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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

pub fn reduce(state: &SharedHistory, action: &SharedHistoryAction) -> Option<SharedHistory> {
    match action {
        SharedHistoryAction::Backfilled { story, records } => backfill(state, *story, records),
    }
}

fn backfill(
    state: &SharedHistory,
    story: StoryId,
    records: &[VerdictRecord],
) -> Option<SharedHistory> {
    let mut next: Vec<VerdictRecord> = state
        .records
        .iter()
        .filter(|r| r.story != story)
        .cloned()
        .collect();
    next.extend(records.iter().cloned());
    if next == state.records {
        return None;
    }
    Some(SharedHistory { records: next })
}
