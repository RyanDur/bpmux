use crate::CiStateStore;
use crate::RunEntry;

use super::{CiBroadcastListener, CiSnapshot};

pub struct CiStateStoreWriter {
    store: CiStateStore,
}

impl CiStateStoreWriter {
    pub fn new(store: CiStateStore) -> Self {
        Self { store }
    }
}

impl CiBroadcastListener for CiStateStoreWriter {
    fn on_changed(&self, snap: &CiSnapshot) {
        let runs = vec![RunEntry {
            status: snap.conclusion.to_string(),
            conclusion: Some(snap.conclusion.to_string()),
            head_sha: snap.head_sha.clone(),
            display_title: snap.display_title.clone(),
            database_id: snap.run_id,
            name: None,
            annotation_message: None,
        }];
        let report = crate::build_ci_state_report(&runs);
        self.store.write(report);
    }
}
