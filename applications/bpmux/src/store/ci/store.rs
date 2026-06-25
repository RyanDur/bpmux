use std::sync::Mutex;

use super::CiSnapshot;
use crate::store::StoreAction;

#[derive(Default)]
pub struct CiStore {
    inner: Mutex<CiStoreState>,
}

#[derive(Default, Clone)]
struct CiStoreState {
    last_seen: Option<(Option<u64>, &'static str)>,
    snapshot: Option<CiSnapshot>,
}

impl CiStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> Option<CiSnapshot> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.snapshot.clone()
    }

    pub fn last_seen(&self) -> Option<(Option<u64>, &'static str)> {
        let state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        state.last_seen
    }

    pub(crate) fn apply(&self, action: &StoreAction) -> CiReduceOutcome {
        match action {
            StoreAction::CiUpdated(snap) => {
                let mut state = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                let key = (snap.run_id, snap.conclusion);
                let changed = state.last_seen != Some(key);
                state.last_seen = Some(key);
                state.snapshot = Some(snap.clone());
                if changed {
                    CiReduceOutcome::Changed
                } else {
                    CiReduceOutcome::Unchanged
                }
            }
            _ => CiReduceOutcome::Unchanged,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CiReduceOutcome {
    Changed,
    Unchanged,
}
