use std::sync::Arc;

use user_routing::UserRepo;

pub struct RoutingPersister {
    repo: Arc<dyn UserRepo>,
}

impl RoutingPersister {
    pub fn new(repo: Arc<dyn UserRepo>) -> Self {
        Self { repo }
    }
}

impl kernel::Listener<crate::store::AppState, crate::store::AppAction> for RoutingPersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::store::AppState, crate::store::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.routing, &reaction.current.routing) {
            return Box::pin(async {});
        }
        let plan =
            crate::store::routing::reconcile(&reaction.previous.routing, &reaction.current.routing);
        let repo = self.repo.clone();
        Box::pin(async move {
            for (id, name, role) in &plan.upserts {
                if let Err(e) = repo.upsert_user(id, name, role) {
                    diag::log_diag(&format!("routing: upsert_user failed: {e}"));
                }
            }
            for (id, at) in &plan.cursors {
                if let Err(e) = repo.set_user_last_delivered(id, *at) {
                    diag::log_diag(&format!("routing: set_user_last_delivered failed: {e}"));
                }
            }
            if !plan.deletes.is_empty()
                && let Err(e) = repo.delete_users(&plan.deletes)
            {
                diag::log_diag(&format!("routing: delete_users failed: {e}"));
            }
        })
    }

    fn delivery_mode(&self) -> kernel::DeliveryMode {
        kernel::DeliveryMode::LatestState
    }
}
