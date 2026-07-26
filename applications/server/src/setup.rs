use std::sync::Arc;

use source_sync::{PendingWrite, QueuedWrite};

pub(super) fn hydrate_outbox(outbox: &Arc<dyn source_sync::OutboxRepo>) -> Vec<QueuedWrite> {
    let codec = source_codec::gh_label_codec::GhLabelCodec;
    match outbox.load_outbox() {
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

pub(super) fn race_window_barrier_from_env() -> sync::RaceWindowHandle {
    let raw = std::env::var("BPMUX_TEST_RACE_WINDOW_REQUESTS").ok()?;
    let target: usize = raw.trim().parse().ok()?;
    if target < 2 {
        return None;
    }
    Some(Arc::new(sync::RaceWindowBarrier::new(target)))
}

pub(super) fn boot_ci_state() -> store::ci::CiState {
    let seeded = std::env::var("BPMUX_TEST_SEED_CI_GREEN")
        .map(|value| !value.is_empty() && value != "0")
        .unwrap_or(false);
    if !seeded {
        return store::ci::CiState::default();
    }
    store::ci::hydrated(store::CiSnapshot {
        state: ci::CiState::Green,
        run_id: None,
        head_sha: Some("test-mode-fixture-sha".to_string()),
        display_title: None,
        conclusion: "success",
        workflows: Vec::new(),
    })
}
