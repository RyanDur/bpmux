use super::RosterAction;
use crate::store::{Store, StoreReduceOutcome};

impl Store {
    pub(crate) fn middleware_roster(&self, action: RosterAction) {
        let outcome = self.roster_store.apply(&action);
        if outcome != StoreReduceOutcome::Changed {
            return;
        }
        let members = self.roster_store.members();
        let listeners = {
            let l = self
                .roster_listeners
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            l.clone()
        };
        for listener in listeners {
            listener.on_changed(&members);
        }
    }
}
