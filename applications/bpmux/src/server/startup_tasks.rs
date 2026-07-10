use std::sync::Arc;

use crate::store::Dispatch;
use crate::sync::RefreshDriver;

pub(super) fn spawn_background_ingest(orch: Arc<dyn RefreshDriver>, dispatch: Dispatch) {
    tokio::spawn(async move {
        let backoffs = [0u64, 500, 1500, 4000];
        for (attempt, delay_ms) in backoffs.iter().enumerate() {
            if *delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(*delay_ms)).await;
            }
            match orch.refresh_all().await {
                Ok(()) => {
                    let count = orch.all_stories().len();
                    diag::log_diag(&format!(
                        "server: production orchestrator ingest succeeded on attempt {} ({count} stories)",
                        attempt + 1
                    ));
                    crate::sync::backfill_shared_history(orch.as_ref(), |action| {
                        dispatch.dispatch(action)
                    })
                    .await;
                    return;
                }
                Err(e) if attempt + 1 < backoffs.len() => {
                    diag::log_diag(&format!(
                        "server: production orchestrator ingest attempt {} failed: {e}; retrying with backoff",
                        attempt + 1
                    ));
                }
                Err(e) => {
                    diag::log_diag(&format!(
                        "server: production orchestrator ingest exhausted retries; last error: {e}; board stays empty until a later refresh re-derives it from the source"
                    ));
                }
            }
        }
    });
}
