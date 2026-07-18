use std::sync::Arc;

pub struct ReactionsPersister {
    hub: Arc<hub::Hub>,
}

impl ReactionsPersister {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

impl kernel::Listener<crate::AppState, crate::AppAction> for ReactionsPersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::AppState, crate::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.reactions, &reaction.current.reactions) {
            return Box::pin(async {});
        }
        let store = crate::reactions::to_store(&reaction.current.reactions);
        let dir = self.hub.dir.clone();
        Box::pin(async move {
            if let Err(e) = reactions::save(&dir, &store) {
                diag::log_diag(&format!("reactions: persist failed: {e}"));
            }
        })
    }

    fn delivery_mode(&self) -> kernel::DeliveryMode {
        kernel::DeliveryMode::LatestState
    }
}
