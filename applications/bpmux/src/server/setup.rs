use super::*;
use std::sync::Arc;

use crate::board::{BoardStore, RefreshDriver, SyncOrchestrator};
use crate::session;

pub(super) fn test_mode_enabled() -> bool {
    std::env::var("BPMUX_TEST_MODE")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

pub(super) fn race_window_barrier_from_env() -> Option<Arc<crate::board::RaceWindowBarrier>> {
    let raw = std::env::var("BPMUX_TEST_RACE_WINDOW_REQUESTS").ok()?;
    let target: usize = raw.trim().parse().ok()?;
    if target < 2 {
        return None;
    }
    Some(Arc::new(crate::board::RaceWindowBarrier::new(target)))
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
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>>,
    ci_state_store: crate::transport::ci_state_store::CiStateStore,
    race_window_barrier: Option<Arc<crate::board::RaceWindowBarrier>>,
) -> Arc<dyn RefreshDriver> {
    let mut corruption_detected = false;
    let store = match BoardStore::load_from_storage(session.storage.as_ref()) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            if matches!(e, crate::board::PersistError::Serde(_)) {
                corruption_detected = true;
            }
            crate::session::diag::log_diag(&format!(
                "server: BPMUX_TEST_MODE board-store disk-load failed ({e}); starting empty"
            ));
            Arc::new(BoardStore::new())
        }
    };
    let adapter = Arc::new(crate::board::adapters::in_memory::InMemorySourceAdapter::new());
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
    if corruption_detected {
        orch.mark_recovering_from_corruption();
    }
    if let Err(e) = orch.refresh_all().await {
        crate::session::diag::log_diag(&format!(
            "server: BPMUX_TEST_MODE initial ingest skipped — {e}"
        ));
    }
    orch
}

pub(super) async fn build_prod_orchestrator(
    session: Arc<session::Session>,
    adapter: Arc<crate::board::adapters::github::GitHubSourceAdapter>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::orchestrator::OrchestratorAction>>,
    ci_state_store: crate::transport::ci_state_store::CiStateStore,
    race_window_barrier: Option<Arc<crate::board::RaceWindowBarrier>>,
) -> Arc<dyn RefreshDriver> {
    let mut corruption_detected = false;
    let store = match BoardStore::load_from_storage(session.storage.as_ref()) {
        Ok(s) => {
            let count = s.all_stories().len();
            crate::session::diag::log_diag(&format!(
                "server: board-store hydrated via storage ({count} stories)"
            ));
            Arc::new(s)
        }
        Err(e) => {
            if matches!(e, crate::board::PersistError::Serde(_)) {
                corruption_detected = true;
            }
            crate::session::diag::log_diag(&format!(
                "server: board-store disk-load failed ({e}); starting empty"
            ));
            Arc::new(BoardStore::new())
        }
    };
    let orch_concrete = match SyncOrchestrator::with_storage(
        store.clone(),
        adapter.clone(),
        session.storage.clone(),
    ) {
        Ok(o) => {
            let pending_count = o.snapshot_status().pending_events;
            crate::session::diag::log_diag(&format!(
                "server: board-outbox hydrated via storage ({pending_count} pending events)"
            ));
            o
        }
        Err(e) => {
            crate::session::diag::log_diag(&format!(
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
    if corruption_detected {
        orch.mark_recovering_from_corruption();
        crate::session::diag::log_diag(
            "server: corruption-detected on board-store load; sync-status flagged recovering until first ingest succeeds",
        );
    }
    let backoffs = [0u64, 500, 1500, 4000];
    for (attempt, delay_ms) in backoffs.iter().enumerate() {
        if *delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
        }
        match orch.refresh_all().await {
            Ok(()) => {
                let count = orch.all_stories().len();
                crate::session::diag::log_diag(&format!(
                    "server: production orchestrator ingest succeeded on attempt {} ({count} stories)",
                    attempt + 1
                ));
                crate::board::iteration_order_backfill::backfill_iteration_order(orch.as_ref());
                if let Err(e) = store.persist_to_storage(session.storage.as_ref()) {
                    crate::session::diag::log_diag(&format!(
                        "server: post-ingest snapshot to storage failed: {e}"
                    ));
                }
                spawn_periodic_snapshot(store.clone(), session.storage.clone());
                return orch;
            }
            Err(e) if attempt + 1 < backoffs.len() => {
                crate::session::diag::log_diag(&format!(
                    "server: production orchestrator ingest attempt {} failed: {e}; retrying with backoff",
                    attempt + 1
                ));
            }
            Err(e) => {
                let count = orch.all_stories().len();
                crate::session::diag::log_diag(&format!(
                    "server: production orchestrator ingest exhausted retries; last error: {e}; BoardStore has {count} stories from disk-hydrate (read paths offline-capable)"
                ));
                spawn_periodic_snapshot(store.clone(), session.storage.clone());
            }
        }
    }
    orch
}
