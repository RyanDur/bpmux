use std::sync::Arc;

use crate::db;
use database_support::DatabaseGateway;

pub struct TransitionsDataGateway {
    gateway: Arc<DatabaseGateway>,
}

impl TransitionsDataGateway {
    pub fn new(gateway: Arc<DatabaseGateway>) -> Self {
        Self { gateway }
    }

    pub fn record(&self, story_id: u64, new_state: &str, observed_at: i64) {
        if let Err(e) =
            db::record_if_changed(&self.gateway.lock(), story_id, new_state, observed_at)
        {
            diag::log_diag(&format!(
                "story-transitions: record failed for #{story_id} -> {new_state}: {e}"
            ));
        }
    }

    pub fn since(&self, cutoff: i64) -> Vec<(u64, String, i64)> {
        db::since(&self.gateway.lock(), cutoff).unwrap_or_else(|e| {
            diag::log_diag(&format!("story-transitions: read failed: {e}"));
            Vec::new()
        })
    }
}
