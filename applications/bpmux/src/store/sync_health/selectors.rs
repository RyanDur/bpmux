use super::SyncHealthState;

pub fn snapshot(state: &SyncHealthState) -> SyncHealthState {
    state.clone()
}

pub fn dropped_count(state: &SyncHealthState) -> usize {
    state.dropped_events.len()
}

pub fn ingest_polling(state: &SyncHealthState) -> bool {
    state.ingest_polling
}

pub fn hydration_polling(state: &SyncHealthState) -> bool {
    state.hydration_polling
}
