use std::sync::Arc;

use crate::product_management::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::session;

pub(super) fn test_mode_enabled() -> bool {
    std::env::var("BPMUX_TEST_MODE")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

pub(super) fn race_window_barrier_from_env()
-> Option<Arc<crate::product_management::board::RaceWindowBarrier>> {
    let raw = std::env::var("BPMUX_TEST_RACE_WINDOW_REQUESTS").ok()?;
    let target: usize = raw.trim().parse().ok()?;
    if target < 2 {
        return None;
    }
    Some(Arc::new(
        crate::product_management::board::RaceWindowBarrier::new(target),
    ))
}

pub(super) fn seed_ci_green_for_tests(store: &crate::transport::ci_state_store::CiStateStore) {
    let enabled = std::env::var("BPMUX_TEST_SEED_CI_GREEN")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false);
    if !enabled {
        return;
    }
    store.write(crate::bridge::tools::CiStateReport {
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
    ci_state_store: crate::transport::ci_state_store::CiStateStore,
    race_window_barrier: Option<Arc<crate::product_management::board::RaceWindowBarrier>>,
) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let adapter = Arc::new(
        crate::product_management::board::adapters::in_memory::InMemorySourceAdapter::new(),
    );
    let orch_concrete = match SyncOrchestrator::with_storage(
        store.clone(),
        adapter.clone(),
        session.storage.clone(),
    ) {
        Ok(o) => o,
        Err(_) => SyncOrchestrator::new(store, adapter),
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
    if let Err(e) = orch.refresh_all().await {
        crate::diag::log_diag(&format!(
            "server: BPMUX_TEST_MODE initial ingest skipped — {e}"
        ));
    }
    orch
}

pub(super) fn build_prod_orchestrator(
    session: Arc<session::Session>,
    adapter: Arc<crate::integrations::github::GitHubSourceAdapter>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::store::AppAction>>,
    ci_state_store: crate::transport::ci_state_store::CiStateStore,
    race_window_barrier: Option<Arc<crate::product_management::board::RaceWindowBarrier>>,
) -> Arc<dyn RefreshDriver> {
    let store = Arc::new(BoardStore::new());
    let orch_concrete = match SyncOrchestrator::with_storage(
        store.clone(),
        adapter.clone(),
        session.storage.clone(),
    ) {
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
            SyncOrchestrator::new(store.clone(), adapter.clone())
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
    super::startup_tasks::spawn_background_ingest(orch.clone());
    orch
}
