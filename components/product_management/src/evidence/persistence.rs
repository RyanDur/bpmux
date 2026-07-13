use super::{EvidenceState, EvidenceStore};

pub fn to_store(state: &EvidenceState) -> EvidenceStore {
    EvidenceStore {
        stories: state.stories.clone(),
    }
}

pub fn from_store(store: &EvidenceStore) -> EvidenceState {
    EvidenceState {
        stories: store.stories.clone(),
    }
}
