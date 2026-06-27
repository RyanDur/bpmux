use super::*;
use crate::product_management::board::events::BoardEvent;

impl BoardStore {
    pub fn apply(&self, event: &BoardEvent) {
        self.store.dispatch(BoardAction::Event(event.clone()));
        let _ = self.event_tx.send(event.clone());
        #[cfg(test)]
        {
            let hook = self
                .test_post_broadcast_hook
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(f) = hook.as_ref() {
                f();
            }
        }
    }
}
