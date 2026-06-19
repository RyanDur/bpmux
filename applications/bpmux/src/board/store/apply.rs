use super::*;
use crate::board::events::BoardEvent;
use std::sync::atomic::Ordering;

impl BoardStore {
    pub fn apply(&self, event: &BoardEvent) {
        let _ = self.event_tx.send(event.clone());
        #[cfg(test)]
        {
            let hook = self
                .test_post_send_hook
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(f) = hook.as_ref() {
                f();
            }
        }
        let mut w = self.stories.write().unwrap_or_else(|e| e.into_inner());
        super::apply_events::apply_event(&mut w, event);
        drop(w);
        self.dirty.store(true, Ordering::Release);
    }
}
