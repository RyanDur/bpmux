use std::sync::Arc;

use crate::hub;
use crate::sync::{QueuedWrite, RefreshDriver, SyncOrchestrator};

fn hydrate_outbox(storage: &Arc<dyn crate::storage::Storage>) -> Vec<QueuedWrite> {
    let codec = crate::integrations::github::gh_label_codec::GhLabelCodec;
    match storage.load_outbox() {
        Ok(rows) => rows
            .into_iter()
            .map(|row| QueuedWrite::from_event(&codec, row.id, row.event))
            .collect(),
        Err(e) => {
            crate::diag::log_diag(&format!(
                "server: board-outbox storage-load failed ({e}); starting with empty outbox"
            ));
            Vec::new()
        }
    }
}

fn next_mutation_id_for(hydrated: &[QueuedWrite]) -> u64 {
    hydrated.iter().map(|write| write.id.0).max().unwrap_or(0) + 1
}

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
    hub: Arc<hub::Hub>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::store::AppAction>>,
    ci_state_store: crate::ci::CiStateStore,
    race_window_barrier: Option<Arc<crate::sync::RaceWindowBarrier>>,
) -> (Arc<dyn RefreshDriver>, Vec<QueuedWrite>) {
    let adapter = Arc::new(crate::integrations::in_memory::InMemorySourceAdapter::new());
    let hydrated = hydrate_outbox(&hub.storage);
    let orch_concrete =
        SyncOrchestrator::new(adapter).with_next_mutation_id(next_mutation_id_for(&hydrated));
    let orch_concrete = match action_tx {
        Some(tx) => orch_concrete.with_action_tx(tx),
        None => orch_concrete,
    };
    seed_ci_green_for_tests(&ci_state_store);
    let mut orch_concrete = orch_concrete.with_ci_state_store(ci_state_store);
    if let Some(barrier) = race_window_barrier {
        orch_concrete = orch_concrete.with_race_window_barrier(barrier);
    }
    (Arc::new(orch_concrete), hydrated)
}

pub(super) fn build_prod_orchestrator(
    hub: Arc<hub::Hub>,
    adapter: Arc<crate::integrations::github::GitHubSourceAdapter>,
    action_tx: Option<tokio::sync::mpsc::UnboundedSender<crate::store::AppAction>>,
    ci_state_store: crate::ci::CiStateStore,
    race_window_barrier: Option<Arc<crate::sync::RaceWindowBarrier>>,
) -> (Arc<dyn RefreshDriver>, Vec<QueuedWrite>) {
    let hydrated = hydrate_outbox(&hub.storage);
    crate::diag::log_diag(&format!(
        "server: board-outbox hydrated via storage ({} pending events)",
        hydrated.len()
    ));
    let orch_concrete =
        SyncOrchestrator::new(adapter).with_next_mutation_id(next_mutation_id_for(&hydrated));
    let orch_concrete = match action_tx {
        Some(tx) => orch_concrete.with_action_tx(tx),
        None => orch_concrete,
    };
    seed_ci_green_for_tests(&ci_state_store);
    let mut orch_concrete = orch_concrete.with_ci_state_store(ci_state_store);
    if let Some(barrier) = race_window_barrier {
        orch_concrete = orch_concrete.with_race_window_barrier(barrier);
    }
    (Arc::new(orch_concrete), hydrated)
}
