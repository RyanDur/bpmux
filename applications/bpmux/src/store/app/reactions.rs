use std::sync::Arc;

pub struct ReactionsPersister {
    hub: Arc<crate::hub::Hub>,
}

impl ReactionsPersister {
    pub fn new(hub: Arc<crate::hub::Hub>) -> Self {
        Self { hub }
    }
}

impl kernel::Listener<crate::store::AppState, crate::store::AppAction> for ReactionsPersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.reactions, &reaction.current.reactions) {
            return Box::pin(async {});
        }
        let store = crate::store::reactions::to_store(&reaction.current.reactions);
        let dir = self.hub.dir.clone();
        Box::pin(async move {
            if let Err(e) = crate::reactions::save(&dir, &store) {
                crate::diag::log_diag(&format!("reactions: persist failed: {e}"));
            }
        })
    }

    fn delivery_mode(&self) -> kernel::DeliveryMode {
        kernel::DeliveryMode::LatestState
    }
}
