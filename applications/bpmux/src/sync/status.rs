use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize)]
pub struct SyncStatus {
    pub last_ingest_at: Option<DateTime<Utc>>,
    pub last_push_at: Option<DateTime<Utc>>,
    pub pending_events: usize,
    pub last_error: Option<String>,
    pub dropped_events: Vec<DroppedEvent>,
    #[serde(default)]
    pub stale: bool,
    #[serde(default)]
    pub startup_fail: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroppedEvent {
    pub story_id: u64,
    pub error: String,
    pub ts: DateTime<Utc>,
}

pub fn staleness_threshold_secs() -> u64 {
    std::env::var("BPMUX_STALENESS_THRESHOLD_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(180)
}

pub fn is_stale(
    last_ingest_at: Option<DateTime<Utc>>,
    threshold_secs: u64,
    now: DateTime<Utc>,
) -> bool {
    let Some(last) = last_ingest_at else {
        return false;
    };
    let elapsed = now.signed_duration_since(last);
    elapsed.num_seconds() >= 0 && (elapsed.num_seconds() as u64) > threshold_secs
}

pub fn is_startup_fail(last_ingest_at: Option<DateTime<Utc>>, last_error: Option<&str>) -> bool {
    last_ingest_at.is_none() && last_error.is_some()
}
