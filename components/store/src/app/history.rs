use std::sync::Arc;

pub struct SharedHistoryPersister {
    hub: Arc<hub::Hub>,
}

impl SharedHistoryPersister {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

impl kernel::Listener<crate::AppState, crate::AppAction> for SharedHistoryPersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::AppState, crate::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(
            &reaction.previous.shared_history,
            &reaction.current.shared_history,
        ) {
            return Box::pin(async {});
        }
        let history = reaction.current.shared_history.clone();
        let dir = self.hub.dir.clone();
        Box::pin(async move {
            if let Err(e) = product_management::history_persistence::save(&dir, &history) {
                diag::log_diag(&format!("history: shared-history persist failed: {e}"));
            }
        })
    }

    fn delivery_mode(&self) -> kernel::DeliveryMode {
        kernel::DeliveryMode::LatestState
    }
}

pub struct LocalHistoryPersister {
    hub: Arc<hub::Hub>,
}

impl LocalHistoryPersister {
    pub fn new(hub: Arc<hub::Hub>) -> Self {
        Self { hub }
    }
}

impl kernel::Listener<crate::AppState, crate::AppAction> for LocalHistoryPersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::AppState, crate::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(
            &reaction.previous.local_history,
            &reaction.current.local_history,
        ) {
            return Box::pin(async {});
        }
        let history = reaction.current.local_history.clone();
        let dir = self.hub.dir.clone();
        Box::pin(async move {
            if let Err(e) = board::history_persistence::save(&dir, &history) {
                diag::log_diag(&format!("history: placement-history persist failed: {e}"));
            }
        })
    }

    fn delivery_mode(&self) -> kernel::DeliveryMode {
        kernel::DeliveryMode::LatestState
    }
}
