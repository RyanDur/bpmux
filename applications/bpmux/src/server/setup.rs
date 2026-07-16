use std::sync::Arc;

use crate::hub;
use source_codec::gh_label_codec::GhLabelCodec;
use source_sync::{PendingWrite, QueuedWrite};

use crate::sync::{RefreshDriver, SyncOrchestrator};

fn hydrate_outbox(storage: &Arc<dyn storage::Storage>) -> Vec<QueuedWrite> {
    let codec = source_codec::gh_label_codec::GhLabelCodec;
    match storage.load_outbox() {
        Ok(rows) => rows
            .into_iter()
            .map(|row| QueuedWrite::minted(row.id, PendingWrite::from_event(&codec, row.event)))
            .collect(),
        Err(e) => {
            diag::log_diag(&format!(
                "server: board-outbox storage-load failed ({e}); starting with empty outbox"
            ));
            Vec::new()
        }
    }
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

pub(super) async fn seed_ci_green_for_tests(store: &Arc<crate::store::Store>) {
    let enabled = std::env::var("BPMUX_TEST_SEED_CI_GREEN")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false);
    if !enabled {
        return;
    }
    store
        .dispatch(crate::store::AppAction::Ci(
            crate::store::CiAction::Updated(crate::store::CiSnapshot {
                state: crate::ci::CiState::Green,
                run_id: None,
                head_sha: Some("test-mode-fixture-sha".to_string()),
                display_title: None,
                conclusion: "success",
                workflows: Vec::new(),
            }),
        ))
        .await;
}

pub(super) async fn build_test_orchestrator(
    hub: Arc<hub::Hub>,
    ci_state_store: crate::ci::CiStateStore,
    race_window_barrier: Option<Arc<crate::sync::RaceWindowBarrier>>,
) -> (Arc<dyn RefreshDriver>, Vec<QueuedWrite>) {
    let adapter = Arc::new(crate::integrations::in_memory::InMemorySourceAdapter::new());
    let hydrated = hydrate_outbox(&hub.storage);
    let orch_concrete = SyncOrchestrator::new(adapter, Arc::new(GhLabelCodec));
    let mut orch_concrete = orch_concrete.with_ci_state_store(ci_state_store);
    if let Some(barrier) = race_window_barrier {
        orch_concrete = orch_concrete.with_race_window_barrier(barrier);
    }
    (Arc::new(orch_concrete), hydrated)
}

pub(super) fn build_prod_orchestrator(
    hub: Arc<hub::Hub>,
    adapter: Arc<crate::integrations::github::GitHubSourceAdapter>,
    ci_state_store: crate::ci::CiStateStore,
    race_window_barrier: Option<Arc<crate::sync::RaceWindowBarrier>>,
) -> (Arc<dyn RefreshDriver>, Vec<QueuedWrite>) {
    let hydrated = hydrate_outbox(&hub.storage);
    diag::log_diag(&format!(
        "server: board-outbox hydrated via storage ({} pending events)",
        hydrated.len()
    ));
    let orch_concrete = SyncOrchestrator::new(adapter, Arc::new(GhLabelCodec));
    let mut orch_concrete = orch_concrete.with_ci_state_store(ci_state_store);
    if let Some(barrier) = race_window_barrier {
        orch_concrete = orch_concrete.with_race_window_barrier(barrier);
    }
    (Arc::new(orch_concrete), hydrated)
}
