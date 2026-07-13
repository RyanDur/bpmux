use chrono::{DateTime, Utc};

use crate::sync::DroppedEvent;

#[derive(Default, Clone)]
pub struct SyncHealthState {
    pub last_ingest_at: Option<DateTime<Utc>>,
    pub last_push_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub dropped_events: Vec<DroppedEvent>,
    pub ingest_polling: bool,
    pub hydration_polling: bool,
}
