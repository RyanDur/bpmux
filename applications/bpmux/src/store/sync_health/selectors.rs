use super::SyncHealthState;

pub fn snapshot(state: &SyncHealthState) -> SyncHealthState {
    state.clone()
}

pub fn dropped_count(state: &SyncHealthState) -> usize {
    state.dropped_events.len()
}
