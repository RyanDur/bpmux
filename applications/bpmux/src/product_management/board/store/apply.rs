use super::*;
use crate::product_management::board::events::StoryEvent;
use crate::product_management::story::slice::StoryAction;

impl BoardStore {
    pub fn apply(&self, event: &StoryEvent) {
        self.store.dispatch(StoryAction::Event(event.clone()));
        let _ = self.event_tx.send(StoreChange::Event(event.clone()));
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
