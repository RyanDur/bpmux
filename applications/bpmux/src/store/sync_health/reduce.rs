use crate::sync::DroppedEvent;

use super::{SyncHealthAction, SyncHealthState};

pub fn reduce(state: &SyncHealthState, action: &SyncHealthAction) -> Option<SyncHealthState> {
    match action {
        SyncHealthAction::IngestSucceeded { at } => Some(SyncHealthState {
            last_ingest_at: Some(*at),
            last_error: None,
            ..state.clone()
        }),
        SyncHealthAction::PushSucceeded { at } => Some(SyncHealthState {
            last_push_at: Some(*at),
            ..state.clone()
        }),
        SyncHealthAction::ErrorRecorded { msg } => Some(SyncHealthState {
            last_error: Some(msg.clone()),
            ..state.clone()
        }),
        SyncHealthAction::EventDropped {
            story_id,
            error,
            ts,
        } => {
            let mut dropped_events = state.dropped_events.clone();
            dropped_events.push(DroppedEvent {
                story_id: *story_id,
                error: error.clone(),
                ts: *ts,
            });
            Some(SyncHealthState {
                dropped_events,
                ..state.clone()
            })
        }
        SyncHealthAction::DroppedAcknowledged => {
            if state.dropped_events.is_empty() {
                return None;
            }
            Some(SyncHealthState {
                dropped_events: Vec::new(),
                ..state.clone()
            })
        }
    }
}
