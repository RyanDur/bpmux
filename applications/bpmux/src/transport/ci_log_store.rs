use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use crate::bridge::tools::CiRunFailures;

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

pub type CiLogStoreHandle = CiLogStore;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::bridge::tools::{CiRunFailures, CiTestFailure};

    fn sample(run_id: u64, test_name: &'static str) -> CiRunFailures {
        CiRunFailures {
            run_id,
            conclusion: "failure".to_string(),
            failures: vec![CiTestFailure {
                shard_id: Some("6/6".to_string()),
                test_name: test_name.to_string(),
                file: Some("tests/playwright/offline-board.spec.ts".to_string()),
                line: Some(121),
                error_message: "Test timeout 30000ms exceeded.".to_string(),
                retry_count: 1,
                stage: None,
                class: None,
            }],
        }
    }

    #[test]
    fn empty_store_returns_none() {
        let store = CiLogStore::new();
        assert!(store.read(42).is_none());
    }

    #[test]
    fn write_then_read_round_trips_record() {
        let store = CiLogStore::new();
        store.write(42, sample(42, "F8"));
        let stored = store.read(42).expect("after write read returns Some");
        assert_eq!(stored.run_id, 42);
        assert_eq!(stored.failures.len(), 1);
        assert_eq!(stored.failures[0].test_name, "F8");
    }

    #[test]
    fn distinct_run_ids_are_independently_keyed() {
        let store = CiLogStore::new();
        store.write(1, sample(1, "F8"));
        store.write(2, sample(2, "F9"));
        assert_eq!(store.read(1).unwrap().failures[0].test_name, "F8");
        assert_eq!(store.read(2).unwrap().failures[0].test_name, "F9");
    }

    #[test]
    fn write_overwrites_prior_failures_for_same_run() {
        let store = CiLogStore::new();
        store.write(42, sample(42, "F8"));
        store.write(42, sample(42, "F9"));
        let stored = store.read(42).unwrap();
        assert_eq!(stored.failures[0].test_name, "F9");
    }

    #[test]
    fn clone_shares_inner_state() {
        let store_a = CiLogStore::new();
        let store_b = store_a.clone();
        store_a.write(42, sample(42, "F8"));
        let stored = store_b
            .read(42)
            .expect("clone must observe writes through shared Arc");
        assert_eq!(stored.failures[0].test_name, "F8");
    }
}
