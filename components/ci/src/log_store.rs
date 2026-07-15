use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::CiRunFailures;

#[derive(Clone, Default)]
pub struct CiLogStore {
    inner: Arc<Mutex<HashMap<u64, CiRunFailures>>>,
}

impl CiLogStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(&self, run_id: u64, failures: CiRunFailures) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.insert(run_id, failures);
    }

    pub fn read(&self, run_id: u64) -> Option<CiRunFailures> {
        let guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.get(&run_id).cloned()
    }
}
