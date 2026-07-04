use std::sync::Arc;

use crate::session;
use crate::sync::{RefreshDriver, SyncOrchestrator};

pub(super) fn test_mode_enabled() -> bool {
    std::env::var("BPMUX_TEST_MODE")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

pub(super) fn race_window_barrier_from_env() -> Option<Arc<crate::sync::RaceWindowBarrier>> {
    let raw = std::env::var("BPMUX_TEST_RACE_WINDOW_REQUESTS").ok()?;
    let target: usize = raw.trim().parse().ok()?;
    if target < 2 {
        return None;
    }
    Some(Arc::new(crate::sync::RaceWindowBarrier::new(target)))
}

pub(super) fn seed_ci_green_for_tests(store: &crate::ci::CiStateStore) {
    let enabled = std::env::var("BPMUX_TEST_SEED_CI_GREEN")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false);
    if !enabled {
        return;
    }
    store.write(crate::ci::CiStateReport {
        conclusion: "success",
        last_run_id: None,
        branch: "main",
        url: String::new(),
        head_sha: Some("test-mode-fixture-sha".to_string()),
        display_title: None,
        last_updated_at: None,
        last_successful_poll_at: None,
        poll_interval_secs: 0,
        backoff_cap_secs: 0,
        workflows: Vec::new(),
    });
}

pub(super) async fn build_test_orchestrator(
    session: Arc<session::Session>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::store::AppAction>>,
    ci_state_store: crate::ci::CiStateStore,
    race_window_barrier: Option<Arc<crate::sync::RaceWindowBarrier>>,
) -> Arc<dyn RefreshDriver> {
    let adapter = Arc::new(crate::integrations::in_memory::InMemorySourceAdapter::new());
    let orch_concrete =
        match SyncOrchestrator::with_storage(adapter.clone(), session.storage.clone()) {
            Ok(o) => o,
            Err(_) => SyncOrchestrator::new(adapter),
        };
    let orch_concrete = match action_tx {
        Some(tx) => orch_concrete.with_action_tx(tx),
        None => orch_concrete,
    };
    seed_ci_green_for_tests(&ci_state_store);
    let mut orch_concrete = orch_concrete.with_ci_state_store(ci_state_store);
    if let Some(barrier) = race_window_barrier {
        orch_concrete = orch_concrete.with_race_window_barrier(barrier);
    }
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);
    orch
}

pub(super) fn build_prod_orchestrator(
    session: Arc<session::Session>,
    adapter: Arc<crate::integrations::github::GitHubSourceAdapter>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::store::AppAction>>,
    ci_state_store: crate::ci::CiStateStore,
    race_window_barrier: Option<Arc<crate::sync::RaceWindowBarrier>>,
) -> Arc<dyn RefreshDriver> {
    let orch_concrete =
        match SyncOrchestrator::with_storage(adapter.clone(), session.storage.clone()) {
            Ok(o) => {
                let pending_count = o.snapshot_status().pending_events;
                crate::diag::log_diag(&format!(
                    "server: board-outbox hydrated via storage ({pending_count} pending events)"
                ));
                o
            }
            Err(e) => {
                crate::diag::log_diag(&format!(
                    "server: board-outbox storage-load failed ({e}); starting with empty outbox"
                ));
                SyncOrchestrator::new(adapter.clone())
            }
        };
    let orch_concrete = match action_tx {
        Some(tx) => orch_concrete.with_action_tx(tx),
        None => orch_concrete,
    };
    seed_ci_green_for_tests(&ci_state_store);
    let mut orch_concrete = orch_concrete.with_ci_state_store(ci_state_store);
    if let Some(barrier) = race_window_barrier {
        orch_concrete = orch_concrete.with_race_window_barrier(barrier);
    }
    let orch: Arc<dyn RefreshDriver> = Arc::new(orch_concrete);

    orch
}
