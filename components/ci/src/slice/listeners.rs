use crate::CiStateStore;

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
        self.store
            .write(crate::report_from_event(snap.to_ci_event()));
    }
}
