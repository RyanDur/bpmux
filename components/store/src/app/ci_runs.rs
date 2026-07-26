use std::sync::Arc;

use ci::data_gateway::CiRunsDataGateway;

pub struct CiRunsPersister {
    ci_runs: Arc<CiRunsDataGateway>,
}

impl CiRunsPersister {
    pub fn new(ci_runs: Arc<CiRunsDataGateway>) -> Self {
        Self { ci_runs }
    }
}

impl kernel::Listener<crate::AppState, crate::AppAction> for CiRunsPersister {
    fn on_action(
        &self,
        reaction: kernel::Reaction<crate::AppState, crate::AppAction>,
    ) -> kernel::ListenerFuture {
        if Arc::ptr_eq(&reaction.previous.ci, &reaction.current.ci) {
            return Box::pin(async {});
        }
        let Some(snapshot) = ci::slice::snapshot(&reaction.current.ci) else {
            return Box::pin(async {});
        };
        let Some(run_id) = snapshot.run_id else {
            return Box::pin(async {});
        };
        if !matches!(snapshot.state, ci::CiState::Green | ci::CiState::Red) {
            return Box::pin(async {});
        }
        let state = match snapshot.state {
            ci::CiState::Green => "green",
            _ => "red",
        };
        let observed_at = chrono::Utc::now().timestamp();
        let ci_runs = self.ci_runs.clone();
        Box::pin(async move {
            ci_runs.record(run_id, state, observed_at);
        })
    }
}
