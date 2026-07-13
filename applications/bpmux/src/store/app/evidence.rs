use std::sync::Arc;

pub struct EvidencePersister {
    hub: Arc<crate::hub::Hub>,
}

impl EvidencePersister {
    pub fn new(hub: Arc<crate::hub::Hub>) -> Self {
        Self { hub }
    }
}

impl kernel::Listener<crate::store::AppState, crate::store::AppAction> for EvidencePersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.evidence, &reaction.current.evidence) {
            return Box::pin(async {});
        }
        let store = product_management::evidence::to_store(&reaction.current.evidence);
        let dir = self.hub.dir.clone();
        Box::pin(async move {
            if let Err(e) = product_management::evidence::save(&dir, &store) {
                diag::log_diag(&format!("evidence: persist failed: {e}"));
            }
        })
    }

    fn delivery_mode(&self) -> kernel::DeliveryMode {
        kernel::DeliveryMode::LatestState
    }
}
