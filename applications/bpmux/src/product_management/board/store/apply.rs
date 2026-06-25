use super::*;
use crate::product_management::board::events::BoardEvent;

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
        let label_projection = super::middleware::project_label_delta(&w, event);
        super::apply_events::apply_event(&mut w, event, label_projection.as_ref());
    }
}
