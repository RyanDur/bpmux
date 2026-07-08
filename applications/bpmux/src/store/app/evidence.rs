use std::sync::Arc;

pub struct EvidencePersister {
    hub: Arc<crate::hub::Hub>,
}

impl EvidencePersister {
    pub fn new(hub: Arc<crate::hub::Hub>) -> Self {
        Self { hub }
    }
}

impl crate::kernel::Listener<crate::store::AppState, crate::store::AppAction>
    for EvidencePersister
{
    fn on_action(
        &self,
        reaction: crate::kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> crate::kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.evidence, &reaction.current.evidence) {
            return Box::pin(async {});
        }
        let store = crate::product_management::evidence::to_store(&reaction.current.evidence);
        let dir = self.hub.dir.clone();
        Box::pin(async move {
            if let Err(e) = crate::product_management::evidence::save(&dir, &store) {
                crate::diag::log_diag(&format!("evidence: persist failed: {e}"));
            }
        })
    }

    fn delivery_mode(&self) -> crate::kernel::DeliveryMode {
        crate::kernel::DeliveryMode::LatestState
    }
}
