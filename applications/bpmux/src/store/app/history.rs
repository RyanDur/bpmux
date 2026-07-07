use std::sync::Arc;

pub struct SharedHistoryPersister {
    session: Arc<crate::session::Session>,
}

impl SharedHistoryPersister {
    pub fn new(session: Arc<crate::session::Session>) -> Self {
        Self { session }
    }
}

impl crate::kernel::Listener<crate::store::AppState, crate::store::AppAction>
    for SharedHistoryPersister
{
    fn on_action(
        &self,
        reaction: crate::kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> crate::kernel::ListenerFuture {
        if Arc::ptr_eq(
            &reaction.previous.shared_history,
            &reaction.current.shared_history,
        ) {
            return Box::pin(async {});
        }
        let history = reaction.current.shared_history.clone();
        let dir = self.session.dir.clone();
        Box::pin(async move {
            if let Err(e) = crate::product_management::history_persistence::save(&dir, &history) {
                crate::diag::log_diag(&format!("history: shared-history persist failed: {e}"));
            }
        })
    }
}

pub struct LocalHistoryPersister {
    session: Arc<crate::session::Session>,
}

impl LocalHistoryPersister {
    pub fn new(session: Arc<crate::session::Session>) -> Self {
        Self { session }
    }
}

impl crate::kernel::Listener<crate::store::AppState, crate::store::AppAction>
    for LocalHistoryPersister
{
    fn on_action(
        &self,
        reaction: crate::kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> crate::kernel::ListenerFuture {
        if Arc::ptr_eq(
            &reaction.previous.local_history,
            &reaction.current.local_history,
        ) {
            return Box::pin(async {});
        }
        let history = reaction.current.local_history.clone();
        let dir = self.session.dir.clone();
        Box::pin(async move {
            if let Err(e) = crate::board::history_persistence::save(&dir, &history) {
                crate::diag::log_diag(&format!("history: placement-history persist failed: {e}"));
            }
        })
    }
}
