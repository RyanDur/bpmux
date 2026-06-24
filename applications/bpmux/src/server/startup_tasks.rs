use std::sync::Arc;

use crate::product_management::board::RefreshDriver;

pub(super) fn spawn_background_ingest(orch: Arc<dyn RefreshDriver>) {
    tokio::spawn(async move {
        let backoffs = [0u64, 500, 1500, 4000];
        for (attempt, delay_ms) in backoffs.iter().enumerate() {
            if *delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
            }
            match orch.refresh_all().await {
                Ok(()) => {
                    let count = orch.all_stories().len();
                    crate::diag::log_diag(&format!(
                        "server: production orchestrator ingest succeeded on attempt {} ({count} stories)",
                        attempt + 1
                    ));
                    crate::product_management::board::iteration_order_backfill::backfill_iteration_order(orch.as_ref());
                    return;
                }
                Err(e) if attempt + 1 < backoffs.len() => {
                    crate::diag::log_diag(&format!(
                        "server: production orchestrator ingest attempt {} failed: {e}; retrying with backoff",
                        attempt + 1
                    ));
                }
                Err(e) => {
                    crate::diag::log_diag(&format!(
                        "server: production orchestrator ingest exhausted retries; last error: {e}; board stays empty until a later refresh re-derives it from the source"
                    ));
                }
            }
        }
    });
}

#[cfg(test)]
#[path = "startup_tasks_tests.rs"]
mod startup_tasks_tests;
