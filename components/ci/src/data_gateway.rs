use std::sync::Arc;

use crate::db;
use database_support::DatabaseGateway;

pub struct CiRunsDataGateway {
    gateway: Arc<DatabaseGateway>,
}

impl CiRunsDataGateway {
    pub fn new(gateway: Arc<DatabaseGateway>) -> Self {
        Self { gateway }
    }

    pub fn record(&self, run_id: u64, state: &str, observed_at: i64) {
        if let Err(e) = db::record_if_changed(&self.gateway.lock(), run_id, state, observed_at) {
            diag::log_diag(&format!("ci-runs: record failed for run {run_id}: {e}"));
        }
    }

    pub fn since(&self, cutoff: i64) -> Vec<(u64, String, i64)> {
        db::since(&self.gateway.lock(), cutoff).unwrap_or_else(|e| {
            diag::log_diag(&format!("ci-runs: read failed: {e}"));
            Vec::new()
        })
    }
}
