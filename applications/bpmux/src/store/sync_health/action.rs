use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub enum SyncHealthAction {
    IngestPollingStarted,
    IngestPollingStopped,
    HydrationPollingStarted,
    HydrationPollingStopped,
    IngestSucceeded {
        at: DateTime<Utc>,
    },
    PushSucceeded {
        at: DateTime<Utc>,
    },
    ErrorRecorded {
        msg: String,
    },
    EventDropped {
        story_id: u64,
        error: String,
        ts: DateTime<Utc>,
    },
    DroppedAcknowledged,
}
